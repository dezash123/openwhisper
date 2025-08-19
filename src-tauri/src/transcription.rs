use anyhow::Result;
use dasp::Sample;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::path::PathBuf;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use crate::state::AppState;

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

pub async fn transcribe_audio(audio_path: PathBuf, state: &AppState) -> Result<String> {
    let config_guard = state.config.lock().unwrap();
    let config = config_guard.as_ref().ok_or_else(|| anyhow::anyhow!("Config not initialized"))?;
    
    let model_path = std::env::current_dir()?
        .join("models")
        .join(&config.model_name);
    
    if !model_path.exists() {
        return Err(anyhow::anyhow!(
            "Whisper model not found at {:?}",
            model_path
        ));
    }
    
    let ctx = WhisperContext::new_with_params(
        model_path.to_string_lossy().as_ref(),
        WhisperContextParameters::default()
    )?;
    
    let mut state = ctx.create_state()?;
    
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_progress(false);
    
    let samples = preprocess_audio(&audio_path)?;
    
    state.full(params, &samples)?;
    
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
    
    println!("Recording saved to: {:?}", audio_path);
    
    Ok(transcription.trim().to_string())
}
