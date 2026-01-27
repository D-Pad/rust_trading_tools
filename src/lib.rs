use std::{collections::HashMap};

use app_state::{AppState, InitializationError};
use database_ops::{DbError, DataDownloadStatus};
use dotenvy::dotenv;
use bars::BarBuildError;
use tokio::sync::mpsc::unbounded_channel;


#[derive(Debug)]
pub enum RunTimeError {
    DataBase(DbError),
    Init(InitializationError),
    Bar(BarBuildError),
}

impl std::fmt::Display for RunTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RunTimeError::DataBase(e) => write!(f, "{}", e),
            RunTimeError::Init(e) => write!(f, "{}", e),
            RunTimeError::Bar(e) => write!(f, "{}", e)
        }
    }
}


pub fn error_handler(err: RunTimeError) {
    eprintln!("\x1b[1;31m{}\x1b[0m", err) 
}


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
pub async fn dev_test(state: &AppState) -> Result<(), RunTimeError> {

    // let dbi = database_ops::integrity_check(
    //     "kraken", 
    //     "BTCUSD", 
    //     state.database.get_pool(), 
    //     None).await;

    Ok(())

}


struct StatusMessage {
    percent_complete: u8,
    message: String,
}

impl StatusMessage {
    fn new() -> Self {
        StatusMessage {
            message: String::new(),
            percent_complete: 0,
        }
    }
}

struct DownloadStatusViewer {
    pairs: HashMap<String, HashMap<String, StatusMessage>>
}

impl DownloadStatusViewer {

    fn new() -> Self {
        DownloadStatusViewer { pairs: HashMap::new() } 
    }

    fn update_status(&mut self, status: DataDownloadStatus) {

        let (exchange, ticker) = status.exchange_and_ticker();

        let entry = self.pairs.entry(exchange.to_string())
            .or_insert_with(HashMap::new)
            .entry(ticker.to_string())
            .or_insert_with(StatusMessage::new);

        match status {
            DataDownloadStatus::Started { .. } => {
                entry.message = "Downloading".to_string();
            },
            DataDownloadStatus::Progress { percent, .. } => {
                entry.percent_complete = percent;
            },
            DataDownloadStatus::Finished { .. } => {
                entry.percent_complete = 100;
                entry.message = "Finished".to_string();
            },
            DataDownloadStatus::Error { message, .. } => {
                entry.message = format!("Failed: {}", message);
            }
        }

    }

}

impl std::fmt::Display for DownloadStatusViewer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       
        let mut text = String::new();
        
        for exchange in self.pairs.keys() {
            
            text.push_str(&format!("\x1b[1;36m{}\x1b[0m:\n", exchange));
            
            if let Some(pairs) = self.pairs.get(exchange) {
                for token in pairs.keys() {
                    text.push_str(&format!("  \x1b[1;33m{}", token));       
                }
            }
        }

        write!(f, "{}", text)

    }
}

pub async fn initiailze() -> Result<AppState, RunTimeError> {

    dotenv().ok(); 

    let state = app_state::AppState::new()
        .await 
        .map_err(|e| RunTimeError::Init(e))?;

    let mut active_exchanges: Vec<String> = Vec::new();
   
    for (exchange, activated) in &state.config.supported_exchanges.active {
        if *activated { active_exchanges.push(exchange.clone()) }
    };
 
    // Progress listener
    let (prog_tx, mut prog_rx) = unbounded_channel::<DataDownloadStatus>();

    tokio::spawn(async move {

        let mut viewer = DownloadStatusViewer::new();

        while let Some(event) = prog_rx.recv().await {
            
            viewer.update_status(event);
            print!("\r{}", viewer);
        
        }
    });

    database_ops::initialize(
        active_exchanges, 
        state.database.get_pool(),
        state.time_offset(),
        prog_tx.clone()
    )
        .await
        .map_err(|_| RunTimeError::Init(InitializationError::InitFailure))?; 

    Ok(state)
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
        
        let candles = match BarSeries::new(
            exchange, 
            ticker, 
            period, 
            BarType::Candle, 
            &app_state
        ).await {
            Ok(c) => c,
            Err(e) => {
                println!("Test failed: {}", e); 
                panic!()
            }
        };
    }

}


