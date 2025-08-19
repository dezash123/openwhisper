use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use hound::{WavSpec, WavWriter};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

// Safe global state for recording
static RECORDING: AtomicBool = AtomicBool::new(false);
static AUDIO_LEVEL: AtomicU32 = AtomicU32::new(0);

struct RecordingState {
    stream: cpal::Stream,
    writer: Arc<Mutex<WavWriter<std::io::BufWriter<std::fs::File>>>>,
    path: PathBuf,
}

static RECORDING_STATE: Mutex<Option<RecordingState>> = Mutex::new(None);

#[tauri::command]
async fn start_recording() -> Result<String, String> {
    if RECORDING.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }

    let device = cpal::default_host().default_input_device()
        .ok_or_else(|| "No input device available".to_string())?;

    let device_name = device.name().unwrap_or_else(|_| "Unknown device (probably an error)".to_string());
    println!("Using microphone: {}", device_name);
    
    let config = device.default_input_config()
        .map_err(|e| e.to_string())?;
    
    println!("Selected config: {:?}", config);
    
    let home_dir = std::env::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;
    let recordings_dir = std::path::Path::new(&home_dir).join("openwhisper");
    
    std::fs::create_dir_all(&recordings_dir)
        .map_err(|e| e.to_string())?;
    
    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let recording_path = recordings_dir.join(format!("recording_{}.wav", timestamp));
    
    let spec = WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let writer = Arc::new(Mutex::new(WavWriter::create(&recording_path, spec)
        .map_err(|e| e.to_string())?));
    
    let writer_clone = Arc::clone(&writer);
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Calculate RMS volume for brightness
            let rms: f32 = data.iter().map(|x| x * x).sum::<f32>() / data.len() as f32;
            let volume = (rms.sqrt() * 100.0).min(100.0) as u32;
            AUDIO_LEVEL.store(volume, Ordering::Relaxed);
            
            if let Ok(mut writer) = writer_clone.lock() {
                for &sample in data {
                    let _ = writer.write_sample((sample * i16::MAX as f32) as i16);
                }
            }
        },
        |err| eprintln!("Stream error: {}", err),
        None,
    ).map_err(|e| e.to_string())?;
    
    stream.play().map_err(|e| e.to_string())?;
    
    // Store the recording state safely
    let recording_state = RecordingState {
        stream,
        writer,
        path: recording_path.clone(),
    };
    
    *RECORDING_STATE.lock().unwrap() = Some(recording_state);
    RECORDING.store(true, Ordering::SeqCst);
    
    Ok("Recording started".to_string())
}

#[tauri::command]
async fn stop_recording_and_transcribe() -> Result<String, String> {
    if !RECORDING.load(Ordering::SeqCst) {
        return Err("Not recording".to_string());
    }
    
    RECORDING.store(false, Ordering::SeqCst);
    
    // Take the recording state and properly close everything
    let recording_state = RECORDING_STATE.lock().unwrap().take()
        .ok_or("No recording state available")?;
    
    // Drop the stream first
    drop(recording_state.stream);
    
    // Close the writer properly (this flushes the data)
    match Arc::try_unwrap(recording_state.writer) {
        Ok(writer_mutex) => {
            let writer = writer_mutex.into_inner().unwrap();
            writer.finalize().map_err(|e| format!("Failed to finalize WAV file: {}", e))?;
        }
        Err(_) => return Err("Writer still has references".to_string()),
    }
    
    let recording_path = recording_state.path;
    
    // Transcribe the audio
    transcribe_audio(recording_path).await
        .map_err(|e| format!("Transcription failed: {}", e))
}

async fn transcribe_audio(audio_path: PathBuf) -> Result<String> {
    // Look for whisper model in models directory
    let model_path = std::env::current_dir()?
        .join("models")
        .join("ggml-base.en.bin");
    
    if !model_path.exists() {
        return Err(anyhow::anyhow!(
            "Whisper model not found at {:?}",
            model_path
        ));
    }
    
    // Initialize whisper context
    let ctx = WhisperContext::new_with_params(
        model_path.to_string_lossy().as_ref(),
        WhisperContextParameters::default()
    )?;
    
    // Create a new session
    let mut state = ctx.create_state()?;
    
    // Set up parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    
    // Load and convert audio - Whisper expects 16kHz mono
    let mut reader = hound::WavReader::open(&audio_path)?;
    let spec = reader.spec();
    
    let mut samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / 32768.0) // Normalize to [-1, 1]
        .collect();
    
    // Convert to mono if stereo
    if spec.channels == 2 {
        samples = samples
            .chunks(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect();
    }
    
    // Resample to 16kHz if needed (simple approach)
    if spec.sample_rate != 16000 {
        let ratio = spec.sample_rate as f32 / 16000.0;
        let new_len = (samples.len() as f32 / ratio) as usize;
        let mut resampled = Vec::with_capacity(new_len);
        
        for i in 0..new_len {
            let src_idx = (i as f32 * ratio) as usize;
            if src_idx < samples.len() {
                resampled.push(samples[src_idx]);
            }
        }
        samples = resampled;
    }
    
    println!("Processed {} samples for whisper", samples.len());
    
    // Process the audio
    state.full(params, &samples)?;
    
    // Extract the transcription
    let num_segments = state.full_n_segments();
    let mut transcription = String::new();
    
    for i in 0..num_segments {
        if let Some(segment) = state.get_segment(i) {
            transcription.push_str(segment.to_str()?);
            if i < num_segments - 1 {
                transcription.push(' ');
            }
        }
    }
    
    // Keep the recording file (don't delete it)
    println!("Recording saved to: {:?}", audio_path);
    
    Ok(transcription.trim().to_string())
}

#[tauri::command]
fn get_audio_level() -> u32 {
    AUDIO_LEVEL.load(Ordering::Relaxed)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording_and_transcribe, get_audio_level])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
