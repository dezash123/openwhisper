mod config;
mod state;
mod audio;
mod transcription;
mod commands;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![commands::record_and_transcribe])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
