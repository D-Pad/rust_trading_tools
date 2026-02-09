use serde::{
    Deserialize, 
    Serialize
};
use std::{
    collections::HashMap,
    fs,
    path::{
        PathBuf
    }
};
use timestamp_tools::{
    calculate_seconds_in_period,
    get_period_portions_from_string
};
use crate::errors::{
    InitializationError, 
    ConfigError
};


// ------------------------- APP STATE MANAGEMENT -------------------------- //
#[derive(Debug)]
pub struct AppState {
    pub config: AppConfig
}

impl AppState {
    
    pub fn new() -> Result<Self, InitializationError> {
        
        let config = match load_config() {
            Ok(c) => c,
            Err(e) => return Err(
                InitializationError::Config(e)
            )
        }; 

        Ok(AppState { config })

    }

    pub fn get_active_exchanges(&self) -> Vec<String> {

        let mut active_exchanges: Vec<String> = Vec::new();
   
        for (exchange, activated) in &self.config.supported_exchanges.active {
            if *activated { active_exchanges.push(exchange.clone()) }
        }; 

        active_exchanges

    }

    pub fn time_offset(&self) -> u64 {
        self.config.data_download.cache_size_settings_to_seconds()
    }

}


// --------------------------- APP CONFIGURATION --------------------------- //
/// Global app configuration
///
/// The config.json file that's read on startup gets parsed into an AppConfig
/// struct. An Engine gets an instance of the AppConfig. There really only 
/// ever needs to be one AppConfig value and it will be the one that's owned
/// by the Engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub backtesting: BackTestSettings,
    pub supported_exchanges: SupportedExchanges,
    pub data_download: DataDownload, 
    pub chart_parameters: ChartParams,
}

impl AppConfig {
    fn default() -> Self {
        Self {
            backtesting: BackTestSettings { 
                inside_bar: true 
            },
            supported_exchanges: SupportedExchanges { 
                active: HashMap::from([
                    ("kraken".to_string(), true)
                ]) 
            },
            data_download: DataDownload {
                cache_size: "6M".to_string() 
            },
            chart_parameters: ChartParams {
                num_bars: 1000
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackTestSettings {
    pub inside_bar: bool,
} 


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartParams {
    pub num_bars: u16,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedExchanges {
    pub active: HashMap<String, bool>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDownload {
    pub cache_size: String,
}

/// Configuration for data downloads. 
///
/// Used to set the initial data cache size when adding new pairs. For example,
/// if a new pair is added and the cache size is set to 6 months, then tick 
/// data from 6 months ago will be downloaded and put in the database.
impl DataDownload {
    
    pub fn cache_size_settings_to_seconds(&self) -> u64 {
      
        const DEFAULT_RETURN_VAL: u64 = 60 * 60 * 24 * 30;  // ~1 Month

        let (symbol, size) = match get_period_portions_from_string(
            &self.cache_size) 
        {
            Ok(d) => d,
            Err(_) => return DEFAULT_RETURN_VAL
        };
        
        match calculate_seconds_in_period(size, symbol) {
            Ok(v) => v,
            Err(_) => DEFAULT_RETURN_VAL
        } 
    }
}


fn get_path_state() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cache") 
}

/// Loads the config.json file into an AppConfig struct
pub fn load_config() -> Result<AppConfig, ConfigError> {
  
    let cache_path: PathBuf = get_path_state();
    let json_path: PathBuf = cache_path.join("config.json");

    if json_path.exists() {
        if let Ok(d) = fs::read_to_string(&json_path) {
            if let Ok(j) = serde_json::from_str::<AppConfig>(&d) {
                return Ok(j) 
            }
        }
    };
    
    println!(
        "\x1b[1;33mNo save state detected. Loading initial config\x1b[0m"
    );

    let config = AppConfig::default();

    if let Ok(_) = save_config(&config) {
        Ok(config)
    }
    else {
        Err(ConfigError::SaveStateFailed)
    }
}

/// Exports the AppConfig state into the config.json file.
pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {

    let path = get_path_state().join("config.json");
    
    let json = match serde_json::to_string_pretty(config) {
        Ok(d) => d,
        Err(_) => return Err(ConfigError::SaveStateFailed)
    };

    match fs::write(&path, json) {
        Ok(_) => Ok(()),
        Err(_) => Err(ConfigError::SaveStateFailed)
    }
}


