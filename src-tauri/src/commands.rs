use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::path::PathBuf;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use tauri::{Listener, ipc::Channel};
use serde::Serialize;
use crate::fft::{calculate_frequency_bands, FFT_SIZE};
use crate::transcription::transcribe_audio;
use crate::config;

#[derive(Clone, Serialize)]
pub struct AudioLevels {
    pub levels: Vec<f32>,
}

#[tauri::command]
pub async fn record_and_transcribe(
    app: tauri::AppHandle,
    on_audio_levels: Channel<AudioLevels>
) -> Result<String, String> {
    let config = config::get();
    
    let device = cpal::default_host().default_input_device()
        .ok_or_else(|| "No input device available".to_string())?;

    let device_name = device.name().unwrap_or_else(|_| "Unknown device".to_string());
    println!("Using microphone: {}", device_name);
    
    let audio_config = device.default_input_config()
        .map_err(|e| e.to_string())?;
    
    println!("Selected config: {:?}", audio_config);
    
    let recordings_dir = PathBuf::from(&config.recording_dir);
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
    
    let mut writer = WavWriter::create(&recording_path, spec)
        .map_err(|e| e.to_string())?;
    
    let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();
    
    // Listen for stop events from frontend
    let _stop_listener = app.listen("stop-recording", move |_event| {
        let _ = stop_tx.send(());
    });
    
    let mut audio_buffer = VecDeque::new();
    let sample_rate = audio_config.sample_rate().0;
    let frequency_bars = config.frequency_bars;
    
    let stream = device.build_input_stream(
        &audio_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let _ = audio_tx.send(data.to_vec());
        },
        |err| eprintln!("Stream error: {}", err),
        None,
    ).map_err(|e| e.to_string())?;
    
    stream.play().map_err(|e| e.to_string())?;
    
    // Process audio data until stop signal
    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            Some(data) = audio_rx.recv() => {
                // Write to WAV file
                for &sample in &data {
                    let _ = writer.write_sample((sample * i16::MAX as f32) as i16);
                    audio_buffer.push_back(sample);
                    if audio_buffer.len() > FFT_SIZE {
                        audio_buffer.pop_front();
                    }
                }

                if audio_buffer.len() >= FFT_SIZE {
                    let levels = calculate_frequency_bands(&audio_buffer.make_contiguous()[..FFT_SIZE], frequency_bars, sample_rate);
                    let _ = on_audio_levels.send(AudioLevels { levels });
                }
            }
        }
    }
    
    drop(stream);
    
    writer.finalize().map_err(|e| format!("Failed to finalize WAV file: {}", e))?;
    
    transcribe_audio(recording_path).await
        .map_err(|e| format!("Transcription failed: {}", e))
}

