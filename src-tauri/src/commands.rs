use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use crate::audio::{RecordingState, calculate_frequency_bands};
use crate::state::AppState;
use crate::transcription::transcribe_audio;

#[tauri::command]
pub async fn start_recording(state: tauri::State<'_, AppState>) -> Result<String, String> {
    if state.recording.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }

    let device = cpal::default_host().default_input_device()
        .ok_or_else(|| "No input device available".to_string())?;

    let device_name = device.name().unwrap_or_else(|_| "Unknown device (probably an error)".to_string());
    println!("Using microphone: {}", device_name);
    
    let audio_config = device.default_input_config()
        .map_err(|e| e.to_string())?;
    
    println!("Selected config: {:?}", audio_config);
    
    let config_guard = state.config.lock().unwrap();
    let app_config = config_guard.as_ref().ok_or("Config not initialized")?;
    
    let recordings_dir = PathBuf::from(&app_config.recording_dir);
    std::fs::create_dir_all(&recordings_dir)
        .map_err(|e| e.to_string())?;
    
    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let recording_path = recordings_dir.join(format!("recording_{}.wav", timestamp));
    
    let spec = WavSpec {
        channels: audio_config.channels(),
        sample_rate: audio_config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let writer = Arc::new(Mutex::new(WavWriter::create(&recording_path, spec)
        .map_err(|e| e.to_string())?));
    
    let writer_clone = Arc::clone(&writer);
    let audio_buffer = Arc::clone(&state.audio_buffer);
    let audio_levels = Arc::clone(&state.audio_levels);
    let config = Arc::clone(&state.config);
    
    let stream = device.build_input_stream(
        &audio_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Add samples to buffer for FFT analysis
            if let Ok(mut buffer) = audio_buffer.lock() {
                for &sample in data {
                    buffer.push_back(sample);
                    if buffer.len() > 1024 {
                        buffer.pop_front();
                    }
                }
                
                // Perform FFT analysis if we have enough samples
                if buffer.len() >= 1024 {
                    if let Ok(config_guard) = config.lock() {
                        if let Some(app_config) = config_guard.as_ref() {
                            let levels = calculate_frequency_bands(&buffer.make_contiguous()[..1024], app_config.frequency_bars);
                            if let Ok(mut levels_guard) = audio_levels.lock() {
                                *levels_guard = levels;
                            }
                        }
                    }
                }
            }
            
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
    
    *state.recording_state.lock().unwrap() = Some(recording_state);
    state.recording.store(true, Ordering::SeqCst);
    
    Ok("Recording started".to_string())
}

#[tauri::command]
pub async fn stop_recording_and_transcribe(state: tauri::State<'_, AppState>) -> Result<String, String> {
    if !state.recording.load(Ordering::SeqCst) {
        return Err("Not recording".to_string());
    }
    
    state.recording.store(false, Ordering::SeqCst);
    
    let recording_state = state.recording_state.lock().unwrap().take()
        .ok_or("No recording state available")?;
    
    drop(recording_state.stream);
    
    match Arc::try_unwrap(recording_state.writer) {
        Ok(writer_mutex) => {
            let writer = writer_mutex.into_inner().unwrap();
            writer.finalize().map_err(|e| format!("Failed to finalize WAV file: {}", e))?;
        }
        Err(_) => return Err("Writer still has references".to_string()),
    }
    
    transcribe_audio(recording_state.path, &state).await
        .map_err(|e| format!("Transcription failed: {}", e))
}

#[tauri::command]
pub fn get_audio_levels(state: tauri::State<'_, AppState>) -> Vec<f32> {
    state.audio_levels.lock().unwrap().clone()
}
