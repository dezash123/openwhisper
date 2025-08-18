use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use hound::{WavSpec, WavWriter};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

// Global state for recording
static RECORDING: AtomicBool = AtomicBool::new(false);
static mut RECORDING_PATH: Option<PathBuf> = None;
static mut STREAM: Option<cpal::Stream> = None;

#[tauri::command]
async fn start_recording() -> Result<String, String> {
    if RECORDING.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }

    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device available")?;
    
    let config = device.default_input_config()
        .map_err(|e| format!("Failed to get input config: {}", e))?;
    
    // Create output file path
    let temp_dir = std::env::temp_dir();
    let recording_path = temp_dir.join(format!("openwhisper_recording_{}.wav", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()));
    
    let spec = WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let writer = Arc::new(Mutex::new(
        WavWriter::create(&recording_path, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?
    ));
    
    let writer_clone = Arc::clone(&writer);
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if let Ok(mut writer) = writer_clone.lock() {
                for &sample in data {
                    let _ = writer.write_sample((sample * i16::MAX as f32) as i16);
                }
            }
        },
        |err| eprintln!("Stream error: {}", err),
        None,
    ).map_err(|e| format!("Failed to build stream: {}", e))?;
    
    stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;
    
    unsafe {
        RECORDING_PATH = Some(recording_path.clone());
        STREAM = Some(stream);
    }
    
    RECORDING.store(true, Ordering::SeqCst);
    
    Ok("Recording started".to_string())
}

#[tauri::command]
async fn stop_recording_and_transcribe() -> Result<String, String> {
    if !RECORDING.load(Ordering::SeqCst) {
        return Err("Not recording".to_string());
    }
    
    RECORDING.store(false, Ordering::SeqCst);
    
    // Stop and drop the stream
    unsafe {
        if let Some(stream) = STREAM.take() {
            drop(stream);
        }
    }
    
    // Get the recording path
    let recording_path = unsafe {
        RECORDING_PATH.take()
            .ok_or("No recording path available")?
    };
    
    // Wait a bit for the file to be fully written
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
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
    
    // Load and convert audio
    let mut reader = hound::WavReader::open(&audio_path)?;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    
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
    
    // Clean up the temporary file
    let _ = std::fs::remove_file(audio_path);
    
    Ok(transcription.trim().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording_and_transcribe])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
