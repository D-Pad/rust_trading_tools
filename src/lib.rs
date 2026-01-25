use app_state::{AppState, InitializationError};
use database_ops::{DbError, fetch_tables};


#[derive(Debug)]
pub enum RunTimeError {
    DataBase(DbError),
    Init(InitializationError),
}

impl std::fmt::Display for RunTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RunTimeError::DataBase(e) => write!(f, "{}", e),
            RunTimeError::Init(e) => write!(f, "{}", e),
        }
    }
}


pub fn error_handler(err: RunTimeError) {
    eprintln!("\x1b[1;31m{}\x1b[0m", err) 
}


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
pub async fn dev_test(state: &AppState) -> Result<(), RunTimeError> {

    // let time_offset: u64 = state
    //     .config
    //     .data_download
    //     .cache_size_settings_to_seconds();

    // if let Err(db) = database_ops::download_new_data_to_db_table(
    //     "kraken", 
    //     "BTCUSD", 
    //     state.database.get_pool(), 
    //     time_offset, 
    //     None,
    //     Some(true)
    // ).await {
    //     return Err(RunTimeError::DataBase(db)) 
    // };

    let dbi = database_ops::integrity_check(
        "kraken", 
        "BTCUSD", 
        state.database.get_pool(), 
        None).await;

    println!("{}", dbi);
    
    Ok(())

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
    use database_ops::{Db, DbLogin, fetch_tables, integrity_check};
    
    use dotenvy;
    use tokio;

    #[tokio::test]
    async fn database_connection_test() {
       
        dotenvy::dotenv().ok(); 
        
        let dbl = DbLogin::new();
        let db = match Db::new(
            &dbl.host,
            5432,
            &dbl.user,
            &dbl.password 
        ).await {
            Ok(d) => d,
            Err(e) => panic!("{:?}", e)
        };

        let pool = db.get_pool();

        if let Err(_) = fetch_tables(pool.clone()).await {
            panic!();
        };

    }

    #[tokio::test]
    async fn database_integrity_check() {
         
        dotenvy::dotenv().ok(); 
        
        let dbl = DbLogin::new();
        let db = match Db::new(
            &dbl.host,
            5432,
            &dbl.user,
            &dbl.password 
        ).await {
            Ok(d) => d,
            Err(e) => panic!("{:?}", e)
        };

        let db_pool = db.get_pool();

        let tables: Vec<String> = match fetch_tables(db_pool.clone()).await {
            Ok(d) => d,
            Err(_) => panic!("Failed to fetch tables")
        };

        for table_name in &tables {

            if table_name.contains("asset_") {
                
                let parts: Vec<&str> = table_name.split('_').collect();
                let exchange = parts[1];
                let ticker = &parts[2].to_uppercase();

                let check_val = integrity_check(
                    exchange, 
                    ticker, 
                    db_pool.clone(), 
                    None 
                ).await;

                println!("\n{}", check_val);

                if !check_val.is_ok {
                    let msg = format!(
                        "Failed check on asset_{exchange}_{ticker}"
                    );
                    panic!("{}", msg); 
                };
            };
        };
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


