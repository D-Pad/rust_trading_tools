use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};


pub enum ConfigError {
    FileNotFound,
    ParseFailure,
    SaveStateFailed,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub backtesting: BackTestSettings,
    pub supported_exchanges: SupportedExchanges,
    pub data_download: DataDownload, 
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackTestSettings {
    pub inside_bar: bool,
} 


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedExchanges {
    pub active: HashMap<String, bool>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDownload {
    pub cache_size_units: u8,
    pub cache_size_period: String,
}

impl DataDownload {
    pub fn get_time_period(&self) -> String {
        format!("{}{}", self.cache_size_units, self.cache_size_period)
    }
}


fn get_json_path_state() -> PathBuf {
    Path::new("../cache/config.json").to_path_buf()
}


pub fn load_config() -> Result<AppConfig, ConfigError> {
  
    let json_path: PathBuf = get_json_path_state();

    if json_path.exists() {
        if let Ok(d) = fs::read_to_string(&json_path) {
            if let Ok(j) = serde_json::from_str::<AppConfig>(&d) {
                return Ok(j) 
            }
        }
    };

    // Fallback to default .toml file if not .json file is present
    let toml_config_path: &'static str = "../cache/config.toml";
    
    let contents = match fs::read_to_string(toml_config_path) {
        Ok(d) => d,
        Err(_) => return Err(ConfigError::FileNotFound)
    };

    let config: AppConfig = match toml::from_str(&contents) {
        Ok(d) => d,
        Err(_) => return Err(ConfigError::ParseFailure)
    };

    if let Ok(_) = save_config(&config) {
        Ok(config)
    }
    else {
        Err(ConfigError::SaveStateFailed)
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {

    let path = get_json_path_state();
    
    let json = match serde_json::to_string_pretty(config) {
        Ok(d) => d,
        Err(_) => return Err(ConfigError::SaveStateFailed)
    };

    match fs::write(&path, json) {
        Ok(_) => Ok(()),
        Err(_) => Err(ConfigError::SaveStateFailed)
    }
}

