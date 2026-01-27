use std::{collections::HashMap, io::{stdout, Write}};

pub use database_ops::{self, DbError, DataDownloadStatus};
pub use bars::BarBuildError;
pub mod app_state;
pub use app_state::{AppState, InitializationError};

use tokio::sync::mpsc::unbounded_channel;
use dotenvy::dotenv;


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


enum StatusMessageProgress {
    Started,
    Completed,
    Failed,
}

struct StatusMessage {
    percent_complete: u8,
    progress: StatusMessageProgress,
}

impl StatusMessage {
    fn new() -> Self {
        StatusMessage {
            percent_complete: 0,
            progress: StatusMessageProgress::Started,
        }
    }
}

struct DownloadStatusViewer {
    pairs: HashMap<String, HashMap<String, StatusMessage>>,
    last_rendered_lines: u16,
    rendered_text: String,
}

impl DownloadStatusViewer {

    fn new() -> Self {
        DownloadStatusViewer { 
            pairs: HashMap::new(), 
            last_rendered_lines: 0,
            rendered_text: String::new()
        } 
    }

    fn render_lines(&mut self) {
        // Action	              | Code
        // ----------------------------------
        // Move cursor up N lines |	\x1b[{N}A
        // Clear entire line	  | \x1b[2K
        // Move cursor to col 0	  | \r
        // Hide cursor	          | \x1b[?25l
        // Show cursor	          | \x1b[?25h
        let mut text = String::new();
        let mut line_count: u16 = 0;
        
        for (exchange, pairs) in &self.pairs {
            
            text.push_str(&format!("\x1b[1;36m{}\x1b[0m:\n", exchange));
            line_count += 1;
 
            for (token, status) in pairs {
                
                text.push_str(&format!("  \x1b[33m{}\x1b[0m: ", token));
                
                match status.progress {
                    StatusMessageProgress::Started => {
                        text.push_str(&format!(
                            "Download Progress: \x1b[1;32m{}%\x1b[0m\n",
                            status.percent_complete
                        ));
                    },
                    StatusMessageProgress::Completed => {
                        text.push_str("\x1b[1;32mComplete\x1b[0m\n");
                    },
                    StatusMessageProgress::Failed => {
                        text.push_str("\x1b[1;31mFAILED\x1b[0m\n"); 
                    }
                };
                
                line_count += 1;
            }
        };
     
        self.last_rendered_lines = line_count;
        self.rendered_text = text;
    }

    fn update_status(&mut self, status: DataDownloadStatus) {

        let (exchange, ticker) = status.exchange_and_ticker();

        let entry = self.pairs.entry(exchange.to_string())
            .or_insert_with(HashMap::new)
            .entry(ticker.to_string())
            .or_insert_with(StatusMessage::new);

        match status {
            DataDownloadStatus::Started { .. } => {
                entry.progress = StatusMessageProgress::Started;
            },
            DataDownloadStatus::Progress { percent, .. } => {
                entry.percent_complete = percent;
            },
            DataDownloadStatus::Finished { .. } => {
                entry.percent_complete = 100;
                entry.progress = StatusMessageProgress::Completed;
            },
            DataDownloadStatus::Error { .. } => {
                entry.progress = StatusMessageProgress::Failed;
            }
        };
    }
}

impl std::fmt::Display for DownloadStatusViewer { 
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.rendered_text)
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
        
        print!("\x1b[?25l");  // Hide cursor
        while let Some(event) = prog_rx.recv().await {
            
            viewer.update_status(event);
          
            // Move cursor to top
            if viewer.last_rendered_lines > 0 {
                print!("\x1b[{}A", viewer.last_rendered_lines);
            };

            // Clear old lines
            for _ in 0..viewer.last_rendered_lines {
                print!("\r\x1b[2K\n");
            };

            // Move cursor to top, again
            if viewer.last_rendered_lines > 0 {
                print!("\x1b[{}A", viewer.last_rendered_lines);
            };

            viewer.render_lines();
            print!("{}", viewer);
            stdout().flush().ok();
        
        }
        print!("\x1b[?25h");  // Show cursor
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


