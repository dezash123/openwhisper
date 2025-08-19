use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::collections::VecDeque;
use crate::config::Config;
use crate::audio::RecordingState;

#[derive(Default)]
pub struct AppState {
    pub config: Arc<Mutex<Option<Config>>>,
    pub recording: Arc<AtomicBool>,
    pub audio_levels: Arc<Mutex<Vec<f32>>>,
    pub audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    pub recording_state: Arc<Mutex<Option<RecordingState>>>,
}