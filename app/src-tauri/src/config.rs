use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub recording_dir: String,
    pub model_name: String,
    pub frequency_bars: usize,
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
            frequency_bars: 16,
        }
    }
}

pub fn get() -> Config {
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("openwhisper"),
        None => return Config::default(),
    };

    let config_path = config_dir.join("config.toml");

    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(config) = toml::from_str::<Config>(&content) {
            return config;
        } else {
            eprintln!("Failed to parse config file");
        }
    } else {
        eprintln!("Config file not found");
    }

    let default_config = Config::default();
    if fs::create_dir_all(&config_dir).is_ok() {
        if let Ok(content) = toml::to_string_pretty(&default_config) {
            if let Err(e) = fs::write(&config_path, content) {
                eprintln!("Failed to write config file: {}", e);
            }
        } else {
            eprintln!("Failed to serialize default config to string");
        }
    } else {
        eprintln!("Failed to create config directory");
    }

    default_config
}
