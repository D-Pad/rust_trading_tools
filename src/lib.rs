use database_ops::{self, kraken};
use bars;
use crate::config::AppConfig;
pub mod config;

use std::collections:HashMap;


pub enum InitializationError {
    InitFailure
}


struct AppState {
    database: database_ops::Db
}

impl AppState {
    pub fn new(&self) {
        
    }
}


pub async fn fetch_data_and_build_bars(
    exchange: &str,
    ticker: &str,
    period: &str,
    number_of_ticks: Option<u64>
) -> bars::BarSeries {
 
    let num_ticks = match number_of_ticks {
        Some(t) => Some(t),
        None => Some(1_000_000)
    };

    let tick_data: Vec<(u64, u64, f64, f64)> = 
        match database_ops::fetch_rows(exchange, ticker, num_ticks).await {
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


pub async fn initiailze(config: &AppConfig) -> Result<(), InitializationError> {

    let mut active_exchanges: Vec<String> = Vec::new();
    
    for (exchange, activated) in &config.supported_exchanges.active {
        if *activated { active_exchanges.push(exchange.clone()) }
    };
    
    if let Err(_) = database_ops::initialize(active_exchanges).await {
        return Err(InitializationError::InitFailure) 
    }; 

    Ok(())
}


