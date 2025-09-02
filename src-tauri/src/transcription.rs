use crate::config;
use anyhow::Result;
use dasp::Sample;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::io::Write;
use std::path::PathBuf;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

async fn get_model_path(config: &config::Config) -> Result<PathBuf> {
    let model_filename = format!("{}.bin", config.model_name);
    let model_path = std::env::current_dir()?
        .join("models")
        .join(&model_filename);

    if !model_path.exists() {
        if let Some(parent) = model_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let download_url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            config.model_name
        );

        let mut response = reqwest::get(&download_url).await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download model: HTTP {}",
                response.status()
            ));
        }

        let mut file = std::fs::File::create(&model_path)?;
        while let Some(bytes) = response.chunk().await? {
            file.write_all(&bytes)?;
        }
    }

    Ok(model_path)
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
        2 => raw_samples
            .chunks_exact(2)
            .map(|stereo| {
                let left = stereo[0];
                let right = stereo[1];
                ((left * left + right * right) / 2.0).sqrt() * left.signum().max(right.signum())
            })
            .collect(),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported channel count: {}",
                spec.channels
            ))
        }
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
            1,
        )?;

        let waves_in = vec![mono_samples];
        let waves_out = resampler.process(&waves_in, None)?;
        waves_out[0].clone()
    } else {
        mono_samples
    };

    println!(
        "Processed {} samples for whisper (16kHz mono)",
        samples.len()
    );
    Ok(samples)
}

fn save_preprocessed_audio(samples: &[f32], output_path: &PathBuf) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(output_path, spec)?;

    for &sample in samples {
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;
    Ok(())
}

pub async fn transcribe_audio(audio_path: PathBuf, session_dir: PathBuf) -> Result<String> {
    let config = config::get();
    println!("Transcribing audio file: {:?}", audio_path);
    let model_path = get_model_path(&config).await?;
    println!("Using model: {:?}", model_path);

    let ctx = WhisperContext::new_with_params(
        model_path.to_string_lossy().as_ref(),
        WhisperContextParameters::default(),
    )?;

    let mut state = ctx.create_state()?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_progress(false);

    let samples = preprocess_audio(&audio_path)?;

    // Save whisper-preprocessed audio
    let whisper_wav_path = session_dir.join("whisper.wav");
    save_preprocessed_audio(&samples, &whisper_wav_path)?;

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

    // Save transcription to markdown file
    let transcription_path = session_dir.join("transcription.md");
    std::fs::write(&transcription_path, &transcription)?;

    println!("Session saved to: {:?}", session_dir);

    Ok(transcription.trim().to_string())
}
