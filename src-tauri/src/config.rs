use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub recording_dir: String,
    pub model_name: String,
    pub frequency_bars: usize,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .to_string_lossy()
            .to_string();
        
        Self {
            recording_dir: format!("{}/recordings", home_dir),
            model_name: "ggml-base.en.bin".to_string(),
            frequency_bars: 16,
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("openwhisper");
    
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.toml"))
}

pub fn load_or_create_config() -> Result<Config> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    } else {
        let default_config = Config::default();
        let config_str = toml::to_string_pretty(&default_config)?;
        fs::write(&config_path, config_str)?;
        println!("Created default config at: {:?}", config_path);
        Ok(default_config)
    }
}