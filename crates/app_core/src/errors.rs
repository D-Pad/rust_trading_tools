pub use database_ops::DbError;
pub use crate::{
    arg_parsing::{ParserError},
    bars::{BarBuildError}
};
pub use config::ConfigError;


#[derive(Debug)]
pub enum RunTimeError {
    DataBase(DbError),
    Init(InitializationError),
    Bar(BarBuildError),
    Arguments(ParserError),
    TuiError,
}

impl std::fmt::Display for RunTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RunTimeError::DataBase(e) => write!(f, "{}", e),
            RunTimeError::Init(e) => write!(f, "{}", e),
            RunTimeError::Bar(e) => write!(f, "{}", e),
            RunTimeError::Arguments(e) => write!(f, "{}", e),
            RunTimeError::TuiError => write!(f, "TUI Crashed"),
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



