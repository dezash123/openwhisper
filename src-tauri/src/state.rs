use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

#[derive(Default)]
pub struct AppState {
    pub audio_buffer: Arc<Mutex<VecDeque<f32>>>,
}