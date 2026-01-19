use database_ops;
use bars;
use crate::config::AppConfig;
pub mod config;


pub enum InitializationError {
    InitFailure
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
    database_ops::download_new_data_to_db_table(
        "kraken", "BTCUSD", Some(4000000), None 
    ).await;
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


