use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{PathBuf};
use database_ops::{Db, DbLogin, DbError};
use timestamp_tools;


// ------------------------- APP STATE MANAGEMENT -------------------------- //
#[derive(Debug)]
pub enum InitializationError {
    Db(DbError),
    Config(ConfigError),
    InitFailure
}


#[derive(Debug)]
pub struct AppState {
    pub database: Db,
    pub config: AppConfig
}

impl AppState {
    
    pub async fn new() -> Result<Self, InitializationError> {
        
        let db_login: DbLogin = DbLogin::new(); 
                
        if !&db_login.is_valid() {
            return Err(
                InitializationError::Db(
                    DbError::CredentialsMissing
                )
            )
        };
        
        let database = match Db::new(
            &db_login.host,
            3306,
            &db_login.user,
            &db_login.password,
        ).await {
            Ok(d) => d,
            Err(_) => return Err(
                InitializationError::Db(
                    DbError::ConnectionFailed
                )
            )
        };

        let config = match load_config() {
            Ok(c) => c,
            Err(e) => return Err(
                InitializationError::Config(e)
            )
        }; 

        Ok(AppState { database, config })

    }
}


// --------------------------- APP CONFIGURATION --------------------------- //
#[derive(Debug)]
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
    pub chart_parameters: ChartParams,
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


fn get_path_state() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cache") 
}


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
    
    // Fallback to default .toml file if not .json file is present
    let toml_config_path: PathBuf = cache_path.join("config.toml");
    
    let contents = match fs::read_to_string(toml_config_path) {
        Ok(d) => d,
        Err(_) => {
            return Err(ConfigError::FileNotFound)
        }
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


