use serde::Deserialize;
use std::{collections::HashMap, fs};


#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub backtesting: BackTestSettings,
    pub supported_exchanges: SupportedExchanges,
}


#[derive(Debug, Deserialize)]
pub struct BackTestSettings {
    pub inside_bar: bool,
} 


#[derive(Debug, Deserialize)]
pub struct SupportedExchanges {
    pub active: HashMap<String, bool>,
}


pub fn load_toml_config(path: &str)
  -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&contents)?;
    Ok(config)
}


