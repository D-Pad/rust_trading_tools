pub use database_ops::DbError;
pub use bars::BarBuildError;
pub use crate::arg_parsing::{ParserError};


#[derive(Debug)]
pub enum RunTimeError {
    DataBase(DbError),
    Init(InitializationError),
    Bar(BarBuildError),
    Arguments(ParserError),
}

impl std::fmt::Display for RunTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RunTimeError::DataBase(e) => write!(f, "{}", e),
            RunTimeError::Init(e) => write!(f, "{}", e),
            RunTimeError::Bar(e) => write!(f, "{}", e),
            RunTimeError::Arguments(e) => write!(f, "{}", e),
        }
    }
}


pub fn error_handler(err: RunTimeError) {
    eprintln!("\x1b[1;31m{}\x1b[0m", err) 
}


#[derive(Debug)]
pub enum InitializationError {
    Db(DbError),
    Config(ConfigError),
    InitFailure
}

impl std::fmt::Display for InitializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InitializationError::Db(e) => write!(
                f, "InitializationError::DbError: {}", e
            ),
            InitializationError::Config(e) => write!(
                f, "InitializationError::Config: {}", e
            ),
            InitializationError::InitFailure => write!(
                f, "InitializationError::InitFailure"
            ),
        }
    }
}


#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(&'static str),
    ParseFailure,
    SaveStateFailed,
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
        }
    }
}




