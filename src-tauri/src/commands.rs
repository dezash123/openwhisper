use crate::config;
use crate::fft::{calculate_frequency_bands, FFT_SIZE};
use crate::transcription::transcribe_audio;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavWriter;
use log::{info, error};
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use tauri::{ipc::Channel, Listener};
use tokio::sync::mpsc;

#[derive(Clone, Serialize)]
pub struct AudioLevels {
    pub levels: Vec<f32>,
}

#[tauri::command]
pub async fn record_and_transcribe(
    app: tauri::AppHandle,
    audio_level_chan: Channel<AudioLevels>,
) -> Result<String, String> {
    info!("recording");
    let config = config::get();

    let device = cpal::default_host().default_input_device().unwrap();

    let mic_config = device.default_input_config().unwrap();

    let recordings_dir = PathBuf::from(&config.recording_dir);
    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let session_dir = recordings_dir.join(timestamp.to_string());
    std::fs::create_dir_all(&session_dir).unwrap();

    let recording_path = session_dir.join("high-quality.wav");

    let spec = hound::WavSpec {
        channels: mic_config.channels(),
        sample_rate: mic_config.sample_rate().0,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create(&recording_path, spec).unwrap();

    // channels for non-blocking
    let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();

    // Listen for stop events from frontend
    let _stop_listener = app.listen("stop-recording", move |_event| {
        stop_tx.send(()).unwrap();
    });

    let mut audio_buffer = VecDeque::new();
    // let sample_rate = mic_config.sample_rate().0;
    // let frequency_bars = config.frequency_bars;

    // may want to use raw stream and write directly to file
    let stream = device
        .build_input_stream(
            &mic_config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                audio_tx.send(data.to_vec()).unwrap();
            },
            |err| error!("Stream error: {}", err),
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().unwrap();

    info!("playing");

    // Process audio data until stop signal
    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            Some(data) = audio_rx.recv() => {
                // Write to WAV file
                info!("received {} samples", data.len());
                for &sample in &data {
                    writer.write_sample(sample).unwrap();
                    audio_buffer.push_back(sample);
                    if audio_buffer.len() > 16 {
                        audio_buffer.pop_front();
                    }
                }

                if audio_buffer.len() >= 16 {
                    // let levels = calculate_frequency_bands(&audio_buffer.make_contiguous()[..FFT_SIZE], frequency_bars, sample_rate);
                    info!("sent audio");
                    let levels = audio_buffer.make_contiguous()[..16].to_vec();
                    let _ = audio_level_chan.send(AudioLevels { levels });
                }
            }
        }
    }

    drop(stream);

    writer.finalize().unwrap();

    Ok("Done".to_string())

    // transcribe_audio(recording_path, session_dir).await.unwrap();
}
