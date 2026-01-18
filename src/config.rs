use serde::Deserialize;
use std::{collections::HashMap, fs};


#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub backtesting: BackTestSettings,
    pub supported_exchanges: SupportedExchanges,
    pub data_download: DataDownload, 
}


#[derive(Debug, Deserialize)]
pub struct BackTestSettings {
    pub inside_bar: bool,
} 


#[derive(Debug, Deserialize)]
pub struct SupportedExchanges {
    pub active: HashMap<String, bool>,
}


#[derive(Debug, Deserialize)]
pub struct DataDownload {
    pub cache_size_units: u8,
    pub cache_size_period: String,
}

impl DataDownload {
    pub fn get_time_period(&self) -> String {
        format!("{}{}", self.cache_size_units, self.cache_size_period)
    }
}


pub fn load_toml_config(path: &str)
  -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&contents)?;
    Ok(config)
}


