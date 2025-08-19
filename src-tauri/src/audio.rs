use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use cpal::Stream;
use hound::WavWriter;
use rustfft::{FftPlanner, num_complex::Complex};

pub struct RecordingState {
    pub stream: Stream,
    pub writer: Arc<Mutex<WavWriter<std::io::BufWriter<std::fs::File>>>>,
    pub path: PathBuf,
}

pub fn calculate_frequency_bands(samples: &[f32], num_bands: usize) -> Vec<f32> {
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