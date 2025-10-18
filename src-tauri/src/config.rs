use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub recording_dir: String,
    pub model_name: String,
    pub audio_quality: String,
    pub model_weights_path: String,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        Self {
            recording_dir: format!("{}/recordings", home_dir),
            model_name: "base.en".to_string(),
            audio_quality: "high".to_string(),
            model_weights_path: format!("{}/recordings/models", home_dir),
        }
    }
}

pub fn get() -> Config {
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("openwhisper"),
        None => return Config::default(),
    };

    let config_path = config_dir.join("config.toml");

    match fs::read_to_string(&config_path) {
        Ok(content) => match toml::from_str::<Config>(&content) {
            Ok(cfg) => return cfg,
            Err(e) => {
                eprintln!("Failed to parse config file ({}), regenerating defaults", e);
            }
        },
        Err(_) => {
            eprintln!("Config file not found, creating defaults");
        }
    }

    let default_config = Config::default();
    if let Err(e) = save(&default_config) {
        eprintln!("Failed to write default config: {}", e);
    }
    default_config
}

pub fn save(config: &Config) -> Result<(), String> {
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("openwhisper"),
        None => return Err("Unable to resolve config directory".into()),
    };

    if let Err(e) = fs::create_dir_all(&config_dir) {
        return Err(format!("Failed to create config dir: {}", e));
    }

    let config_path = config_dir.join("config.toml");
    let content = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_config() -> Config {
    get()
}

#[tauri::command]
pub async fn set_config(config: Config) -> Result<(), String> {
    save(&config)
}
