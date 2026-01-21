use database_ops::{self, DbError, Db};
use bars;
use crate::config::{AppConfig, ConfigError, load_config};
pub mod config;


pub enum InitializationError {
    Db(DbError),
    Config(ConfigError),
    InitFailure
}


pub struct AppState {
    database: database_ops::Db,
    config: AppConfig
}

impl AppState {
    
    pub async fn new() -> Result<Self, InitializationError> {
        
        let db_login: database_ops::DbLogin = database_ops::DbLogin::new(); 
                
        if !&db_login.is_valid() {
            println!("\x1b[1;31mMissing DB credentials\x1b[0m"); 
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


pub async fn fetch_data_and_build_bars(
    exchange: &str,
    ticker: &str,
    period: &str,
    number_of_ticks: Option<u64>,
    app_state: &AppState 
) -> bars::BarSeries {
 
    let num_ticks = match number_of_ticks {
        Some(t) => Some(t),
        None => Some(1_000_000)
    };

    let tick_data: Vec<(u64, u64, f64, f64)> = match database_ops::fetch_rows(
        exchange, 
        ticker, 
        num_ticks,
        app_state.database.get_pool()
    ).await {
        Ok(d) => d,
        Err(_) => {
            println!("Failed to fetch ticks");
            return bars::BarSeries::empty(); 
        }
    };

    let bar_type = bars::BarType::Candle;
    
    match bars::BarSeries::new(tick_data, period, bar_type) {
        Ok(b) => b,
        Err(_) => bars::BarSeries::empty()
    } 

}


pub async fn dev_test(config: &AppConfig) {

    // let time_offset: u64 = config
    //     .data_download
    //     .cache_size_settings_to_seconds();

    // database_ops::download_new_data_to_db_table(
    //     "kraken", "BTCUSD", Some(time_offset), None 
    // ).await;

    // let _ = kraken::add_new_db_table(
    //     "BTCUSD", 
    //     time_offset, 
    //     None, 
    //     None
    // ).await;

}


pub async fn initiailze(state: &AppState) -> Result<(), InitializationError> {

    let mut active_exchanges: Vec<String> = Vec::new();
   
    for (exchange, activated) in &state.config.supported_exchanges.active {
        if *activated { active_exchanges.push(exchange.clone()) }
    };
   
    let pool = state.database.get_pool();

    if let Err(_) = database_ops::initialize(
        active_exchanges, pool
    ).await {
        return Err(InitializationError::InitFailure) 
    }; 

    Ok(())
}


