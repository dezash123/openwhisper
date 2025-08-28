use rustfft::{FftPlanner, num_complex::Complex};
use iter_num_tools::log_space;

pub fn calculate_frequency_bands(samples: &[f32], num_bands: usize, sample_rate: u32) -> Vec<f32> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(4096);
    
    // Convert samples to complex numbers
    let mut buffer: Vec<Complex<f32>> = samples.iter()
        .map(|&s| Complex::new(s, 0.0))
        .collect();
    
    // Pad or truncate to exactly 4096 samples
    buffer.resize(4096, Complex::new(0.0, 0.0));
    
    // Perform FFT
    fft.process(&mut buffer);
    
    // Convert 80 Hz to 8000 Hz range to bin indices
    let fft_size = 4096;
    let min_bin = (80.0 * fft_size as f32 / sample_rate as f32) as usize;
    let max_bin = ((8000.0 * fft_size as f32 / sample_rate as f32) as usize).min(fft_size / 2);
    
    // Create logarithmically spaced bin boundaries
    let band_boundaries: Vec<usize> = log_space(min_bin as f32..=max_bin as f32, num_bands + 1)
        .map(|x| x as usize)
        .collect();
    
    let mut bands = vec![0.0f32; num_bands];
    
    for (i, band) in bands.iter_mut().enumerate() {
        let start = band_boundaries[i];
        let end = band_boundaries[i + 1];
        
        if start < end {
            let sum: f32 = buffer[start..end]
                .iter()
                .map(|sample| sample.norm())
                .sum();
            
            *band = (sum / (end - start) as f32).min(1.0);
        }
    }
    
    bands
}
