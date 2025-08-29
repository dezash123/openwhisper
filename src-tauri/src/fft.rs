use rustfft::{FftPlanner, num_complex::Complex};
use iter_num_tools::log_space;

const MIN_FREQUENCY: f32 = 50.0;
const MAX_FREQUENCY: f32 = 8000.0;
pub const FFT_SIZE: usize = 4096;

pub fn calculate_frequency_bands(samples: &[f32], num_bands: usize, sample_rate: u32) -> Vec<f32> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    
    let mut buffer: Vec<Complex<f32>> = samples.iter()
        .map(|&s| Complex::new(s, 0.0))
        .collect();
    
    buffer.resize(FFT_SIZE, Complex::new(0.0, 0.0));
    
    fft.process(&mut buffer);
    
    let min_bin = (MIN_FREQUENCY * FFT_SIZE as f32 / sample_rate as f32) as usize;
    let max_bin = (MAX_FREQUENCY * FFT_SIZE as f32 / sample_rate as f32) as usize;
    
    let band_boundaries: Vec<usize> = log_space(min_bin as f32..=max_bin as f32, num_bands + 1)
        .map(|x| x as usize)
        .collect();
    
    let mut bands = vec![0.0f32; num_bands];
    
    for (i, band) in bands.iter_mut().enumerate() {
        let start = band_boundaries[i];
        let end = band_boundaries[i + 1];
        
        let sum: f32 = buffer[start..end]
            .iter()
            .map(|sample| sample.norm())
            .sum();
        
        let avg_magnitude = sum / (end - start) as f32;
        
        let center_bin = (start + end) / 2;
        let frequency = center_bin as f32 * sample_rate as f32 / FFT_SIZE as f32;
        let weight = (frequency / (4.0 * FFT_SIZE as f32)).sqrt(); // looks right
        
        *band = (avg_magnitude * weight).min(1.0);
    }
    
    bands
}
