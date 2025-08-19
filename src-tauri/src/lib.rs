mod config;
mod state;
mod audio;
mod transcription;
mod commands;

use config::{Config, load_or_create_config};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::default();
    
    match load_or_create_config() {
        Ok(config) => {
            let mut audio_levels = app_state.audio_levels.lock().unwrap();
            *audio_levels = vec![0.0; config.frequency_bars];
            
            *app_state.config.lock().unwrap() = Some(config);
        }
        Err(e) => {
            eprintln!("Failed to load config: {}, using defaults", e);
            let default_config = Config::default();
            
            let mut audio_levels = app_state.audio_levels.lock().unwrap();
            *audio_levels = vec![0.0; default_config.frequency_bars];
            
            *app_state.config.lock().unwrap() = Some(default_config);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![commands::start_recording, commands::stop_recording_and_transcribe, commands::get_audio_levels])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
