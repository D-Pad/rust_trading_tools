use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use timestamp_tools;


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
    pub cache_size_units: u64,
    pub cache_size_period: char,
}

impl DataDownload {
    
    pub fn get_time_period(&self) -> String {
        format!("{}{}", self.cache_size_units, self.cache_size_period)
    }

    pub fn cache_size_settings_to_seconds(&self) -> u64 {
      
        const DEFAULT_RETURN_VAL: u64 = 60 * 60 * 24 * 30;  // ~1 Month

        let size = self.cache_size_units as u64;
        let period = self.cache_size_period;
        
        match timestamp_tools::calculate_seconds_in_period(size, period) {
            Ok(v) => v,
            Err(_) => DEFAULT_RETURN_VAL
        } 
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
                println!(
                    "\x1b[1;36mLoading app settings from saved state\x1b[0m"
                ); 
                return Ok(j) 
            }
        }
    };
    
    println!(
        "\x1b[1;33mNo save state detected. Loading initial config\x1b[0m"
    );
    
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

