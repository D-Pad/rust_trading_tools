use serde::Deserialize;
use std::fs;


#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub backtesting: BackTestSettings
}


#[derive(Debug, Deserialize)]
pub struct BackTestSettings {
    pub inside_bar: bool
} 


pub fn load_toml_config(path: &str)
  -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&contents)?;
    Ok(config)
}


