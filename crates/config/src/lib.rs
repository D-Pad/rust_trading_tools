use std::{
    path::PathBuf,
    env,
};


#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(&'static str),
    ParseFailure,
    SaveStateFailed,
    MissingDirectory(&'static str),
    NoChangesMade,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(e) => write!(
                f, "ConfigError::FileNotFound: {}", e
            ),
            ConfigError::ParseFailure => write!(
                f, "ConfigError::ParseFailure: Couldn't parse config file" 
            ),
            ConfigError::SaveStateFailed => write!(
                f, "ConfigError::SaveStateFailed" 
            ),
            ConfigError::MissingDirectory(e) => write!(
                f, "ConfigError::MissingDirectory: {}", e 
            ),
            ConfigError::NoChangesMade => write!(
                f, "ConfigError::NoChangesMade: New config matches old one" 
            ),

        }
    }
}


#[derive(Debug)]
pub struct SystemPaths {
    pub base: PathBuf,
    pub candle_data: PathBuf,
    pub strategy_templates: PathBuf,
}

impl SystemPaths {
    
    pub fn new() -> Result<Self, ConfigError> {

        let mut base = if cfg!(target_os = "windows") {
            // Windows: %APPDATA%
            env::var_os("APPDATA")
                .map(PathBuf::from)
                .ok_or(ConfigError::MissingDirectory("APPDATA not set"))?
        
        } else if cfg!(target_os = "macos") {
            // macOS: ~/Library/Application Support
            let home = env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or(ConfigError::MissingDirectory("HOME not set"))?;
            home.join("Library").join("Application Support")
        
        } else {
            
            // Linux / Unix: XDG spec
            if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
                PathBuf::from(xdg)
            } else {
                let home = env::var_os("HOME")
                    .map(PathBuf::from)
                    .ok_or(ConfigError::MissingDirectory("HOME not set"))?;
                home.join(".config")
            }
        };

        base.push("dtrade");
        let candle_data = base.join("candle_data");
        let strategy_templates = base.join("strategies");
    
        Ok(Self { base, candle_data, strategy_templates })

    }
}


