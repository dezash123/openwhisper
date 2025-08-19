use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use hound::{WavSpec, WavWriter};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use dasp::Sample;
use serde::{Deserialize, Serialize};
use std::fs;
use rustfft::{FftPlanner, num_complex::Complex};
use std::collections::VecDeque;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    recording_dir: String,
    model_name: String,
    frequency_bars: usize,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .to_string_lossy()
            .to_string();
        
        Self {
            recording_dir: format!("{}/recordings", home_dir),
            model_name: "ggml-base.en.bin".to_string(),
            frequency_bars: 16,
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("openwhisper");
    
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.toml"))
}

fn load_or_create_config() -> Result<Config> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    } else {
        let default_config = Config::default();
        let config_str = toml::to_string_pretty(&default_config)?;
        fs::write(&config_path, config_str)?;
        println!("Created default config at: {:?}", config_path);
        Ok(default_config)
    }
}

static RECORDING: AtomicBool = AtomicBool::new(false);
static AUDIO_LEVELS: Mutex<Vec<f32>> = Mutex::new(Vec::new());
static AUDIO_BUFFER: Mutex<VecDeque<f32>> = Mutex::new(VecDeque::new());
static APP_CONFIG: Mutex<Option<Config>> = Mutex::new(None);

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
    
    let audio_config = device.default_input_config()
        .map_err(|e| e.to_string())?;
    
    println!("Selected config: {:?}", audio_config);
    
    let app_config = APP_CONFIG.lock().unwrap();
    let app_config = app_config.as_ref().ok_or("Config not initialized")?;
    
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
    let stream = device.build_input_stream(
        &audio_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Add samples to buffer for FFT analysis
            if let Ok(mut buffer) = AUDIO_BUFFER.lock() {
                for &sample in data {
                    buffer.push_back(sample);
                    if buffer.len() > 1024 {
                        buffer.pop_front();
                    }
                }
                
                // Perform FFT analysis if we have enough samples
                if buffer.len() >= 1024 {
                    if let Ok(config_guard) = APP_CONFIG.lock() {
                        if let Some(config) = config_guard.as_ref() {
                            let levels = calculate_frequency_bands(&buffer.make_contiguous()[..1024], config.frequency_bars);
                            if let Ok(mut audio_levels) = AUDIO_LEVELS.lock() {
                                *audio_levels = levels;
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

fn preprocess_audio(audio_path: &PathBuf) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(audio_path)?;
    let spec = reader.spec();
    
    println!("Audio file specs: {:?}", spec);
    
    let raw_samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap().to_sample::<f32>())
        .collect();
    
    let mono_samples: Vec<f32> = match spec.channels {
        1 => raw_samples,
        2 => {
            raw_samples
                .chunks_exact(2)
                .map(|stereo| {
                    let left = stereo[0];
                    let right = stereo[1];
                    ((left * left + right * right) / 2.0).sqrt() * left.signum()
                        .max(right.signum())
                })
                .collect()
        },
        _ => return Err(anyhow::anyhow!("Unsupported channel count: {}", spec.channels)),
    };
    
    // Resample to 16kHz if needed using rubato
    let samples = if spec.sample_rate != 16000 {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: WindowFunction::BlackmanHarris2,
        };
        
        let mut resampler = SincFixedIn::<f32>::new(
            16000f64 / spec.sample_rate as f64,
            2.0,
            params,
            mono_samples.len(),
            1, // mono
        )?;
        
        let waves_in = vec![mono_samples];
        let waves_out = resampler.process(&waves_in, None)?;
        waves_out[0].clone()
    } else {
        mono_samples
    };
    
    println!("Processed {} samples for whisper (16kHz mono)", samples.len());
    Ok(samples)
}

async fn transcribe_audio(audio_path: PathBuf) -> Result<String> {
    let config = APP_CONFIG.lock().unwrap();
    let config = config.as_ref().ok_or_else(|| anyhow::anyhow!("Config not initialized"))?;
    
    // Look for whisper model in models directory
    let model_path = std::env::current_dir()?
        .join("models")
        .join(&config.model_name);
    
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
    params.set_print_progress(false);
    
    let samples = preprocess_audio(&audio_path)?;
    
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

fn calculate_frequency_bands(samples: &[f32], num_bands: usize) -> Vec<f32> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(1024);
    
    // Convert samples to complex numbers
    let mut buffer: Vec<Complex<f32>> = samples.iter()
        .map(|&s| Complex::new(s, 0.0))
        .collect();
    
    // Pad or truncate to exactly 1024 samples
    buffer.resize(1024, Complex::new(0.0, 0.0));
    
    // Perform FFT
    fft.process(&mut buffer);
    
    // Calculate magnitudes and split into frequency bands
    let nyquist = 512; // Half of 1024
    let band_size = nyquist / num_bands;
    
    let mut bands = vec![0.0f32; num_bands];
    
    for (i, band) in bands.iter_mut().enumerate() {
        let start = i * band_size;
        let end = ((i + 1) * band_size).min(nyquist);
        
        let sum: f32 = buffer[start..end]
            .iter()
            .map(|sample| (sample.re * sample.re + sample.im * sample.im).sqrt())
            .sum();
        
        *band = (sum / (end - start) as f32).min(1.0);
    }
    
    bands
}

#[tauri::command]
fn get_audio_levels() -> Vec<f32> {
    AUDIO_LEVELS.lock().unwrap().clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    match load_or_create_config() {
        Ok(config) => {
            let mut audio_levels = AUDIO_LEVELS.lock().unwrap();
            *audio_levels = vec![0.0; config.frequency_bars];
            
            *APP_CONFIG.lock().unwrap() = Some(config);
        }
        Err(e) => {
            eprintln!("Failed to load config: {}, using defaults", e);
            let default_config = Config::default();
            
            let mut audio_levels = AUDIO_LEVELS.lock().unwrap();
            *audio_levels = vec![0.0; default_config.frequency_bars];
            
            *APP_CONFIG.lock().unwrap() = Some(default_config);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording_and_transcribe, get_audio_levels])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
