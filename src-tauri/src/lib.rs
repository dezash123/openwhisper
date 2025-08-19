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
static AUDIO_LEVEL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[tauri::command]
async fn start_recording() -> Result<String, String> {
    if RECORDING.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }

    let avail_hosts = cpal::available_hosts();
    println!("Available hosts: {:?}", avail_hosts);

    let host = cpal::default_host();
    
    // List all available input devices for debugging
    println!("Available input devices:");
    for (index, device) in host.input_devices().unwrap().enumerate() {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        println!("  {}: {}", index, name);
        
        // Print supported configs
        // if let Ok(configs) = device.supported_input_configs() {
        //     for config in configs {
        //         println!("    - {:?}", config);
        //     }
        // }
    }
    
    // Try to use device index 3, fallback to pipewire, then default
    let device = host.input_devices()
        .unwrap()
        .nth(3)
        .or_else(|| {
            host.input_devices()
                .unwrap()
                .find(|d| {
                    if let Ok(name) = d.name() {
                        name.to_lowercase().contains("pipewire")
                    } else {
                        false
                    }
                })
        })
        .or_else(|| host.default_input_device())
        .ok_or("No input device available")?;
    
    // Print the microphone name
    let device_name = device.name().unwrap_or_else(|_| "Unknown device".to_string());
    println!("Using microphone: {}", device_name);
    
    let config = device.default_input_config()
        .map_err(|e| format!("Failed to get input config: {}", e))?;
    
    println!("Selected config: {:?}", config);
    
    // Create output file path in ~/recordings/
    let home_dir = std::env::var("HOME").map_err(|_| "Could not find home directory")?;
    let recordings_dir = std::path::Path::new(&home_dir).join("recordings");
    
    // Create recordings directory if it doesn't exist
    std::fs::create_dir_all(&recordings_dir)
        .map_err(|e| format!("Failed to create recordings directory: {}", e))?;
    
    let recording_path = recordings_dir.join(format!("openwhisper_recording_{}.wav", 
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
            // Debug: print when we receive audio data
            // if data.len() > 0 {
            //     println!("Received {} audio samples, first sample: {}", data.len(), data[0]);
            // }
            
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
