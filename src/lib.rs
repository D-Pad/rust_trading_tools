use app_state::{AppState, InitializationError};


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
pub async fn dev_test(state: &AppState) {

    let time_offset: u64 = state
        .config
        .data_download
        .cache_size_settings_to_seconds();

    let _ = database_ops::download_new_data_to_db_table(
        "kraken", "BTCUSD", state.database.get_pool(), time_offset, None 
    ).await;

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


#[cfg(test)]
mod tests {

    use bars::*;
    use app_state::*;
    use database_ops::{DbLogin, Db};
    
    use dotenvy;
    use tokio;

    #[tokio::test]
    async fn database_connection_test() {
       
        dotenvy::dotenv().ok(); 
        
        let dbl = DbLogin::new();
        let db = match Db::new(
            &dbl.host,
            3306,
            &dbl.user,
            &dbl.password 
        ).await {
            Ok(d) => d,
            Err(e) => panic!("{:?}", e)
        };

        let db_pool = db.get_pool();

        db_pool.get_conn().await.unwrap();
    }

    #[tokio::test]
    async fn candle_test() {
        
        dotenvy::dotenv().ok(); 
        
        let app_state = AppState::new().await.unwrap();

        let exchange = "kraken".to_string();
        let ticker = "BTCUSD".to_string();
        let period = "1h".to_string();
        
        BarSeries::new(exchange, ticker, period, BarType::Candle, &app_state);
    }

}


