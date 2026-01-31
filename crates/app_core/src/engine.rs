use std::{collections::HashMap, io::{self, Write}};

use bars::{BarSeries, BarType, BarBuildError};
use database_ops::*;

use crate::{
    app_state::AppState,
    errors::{RunTimeError},
    arg_parsing::{
        Command,
        DataResponse,
        ParsedArgs,
        Response,
        parse_args
    },
    DataDownloadStatus,
    DownloadStatusViewer,
    PgPool
};

use reqwest::Client;
use tokio::{sync::mpsc::unbounded_channel};


pub struct Engine {
    pub state: AppState,
    pub database: Db,
    pub request_client: Client,
    pub args: ParsedArgs
}

impl Engine {
   
    pub fn new(database: Db) -> Result<Self, RunTimeError> {

        let state: AppState = AppState::new()
            .map_err(|e| RunTimeError::Init(e))?;

        let request_client: Client = Client::new();

        let args: ParsedArgs = parse_args(None);

        Ok(Engine { state, database, request_client, args })

    }

    pub async fn execute_commands(&mut self) -> Result<Response, RunTimeError> {
        
        let mut response: Option<Response> = None;

        for _ in 0..self.args.commands.len() {
            
            let cmd = self.args.commands.remove(0);
            
            match self.handle(cmd).await? {
                Response::Ok => {},
                Response::Data(data) => {
                    response = Some(Response::Data(data));
                }
            }; 
        };

        Ok(match response {
            Some(data) => data,
            None => Response::Ok
        })
    }

    pub async fn handle(&mut self, cmd: Command) 
        -> Result<Response, RunTimeError> {
        
        match cmd {
            
            Command::AddPair { exchange, ticker } => {
              
                add_new_pair(
                    &exchange, 
                    &ticker, 
                    self.state.time_offset(),
                    self.database.get_pool(),
                    &self.request_client
                ).await.map_err(|e| RunTimeError::DataBase(e))?;

                Ok(Response::Ok)
            },

            Command::DropPair { exchange, ticker } => {
                
                drop_pair(&exchange, &ticker, self.database.get_pool())
                    .await 
                    .map_err(|e| RunTimeError::DataBase(e))?;

                Ok(Response::Ok)
            },

            Command::StartServer => { 
                // TODO: Add server starting logic 
                Ok(Response::Ok)
            },

            Command::UpdatePairs => {
                run_database_table_updates(
                    &self.state, 
                    &self.request_client, 
                    self.database.get_pool(),
                ).await?;
                
                Ok(Response::Ok)
            },

            Command::CandleBuilder { 
                exchange, ticker, period, integrity_check 
            } => {
    
                let bars = BarSeries::new(
                    exchange, 
                    ticker, 
                    period, 
                    BarType::Candle, 
                    self.database.get_pool() 
                )
                    .await
                    .map_err(|e| RunTimeError::Bar(e))?;

                if integrity_check {
                    let is_ok: bool = bars.bar_integrity_check();
                    if !is_ok {
                        return Err(RunTimeError::Bar(
                            BarBuildError::IntegrityCorruption
                        )) 
                    }; 
                };

                Ok(Response::Data(DataResponse::Bars(bars)))
            },

            Command::DbIntegrityCheck { exchange, ticker } => {
                db_integrity_check(
                    &exchange, 
                    &ticker, 
                    self.database.get_pool() 
                ).await;

                Ok(Response::Ok)
            }
        }    
    }
}


pub async fn run_database_table_updates(
    state: &AppState,
    client: &reqwest::Client,
    db_pool: PgPool
) -> Result<(), RunTimeError> {

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
            io::stdout().flush().ok();
        
        }
        print!("\x1b[?25h");  // Show cursor
    });

    update_database_tables(
        &state.get_active_exchanges(),
        state.time_offset(),
        client,
        db_pool,
        prog_tx.clone()
    )
        .await
        .map_err(|e| RunTimeError::DataBase(e))?;

    Ok(())

}

async fn db_integrity_check(
    exchange: &str, 
    ticker: &str, 
    db_pool: PgPool
) -> String {
  
    let tables: Vec<String> = match fetch_tables(db_pool.clone()).await {
        Ok(d) => d,
        Err(_) => Vec::new()
    };

    let mut tables_to_check: HashMap<String, Vec<String>> = HashMap::new();

    if exchange != "all" {
        tables_to_check.entry(exchange.to_string())
            .or_insert(Vec::new());
    };

    if ticker != "all" {
        tables_to_check.entry(exchange.to_string())
            .or_insert(Vec::new())
            .push(ticker.to_lowercase());
    };

    for table in &tables {
        
        if !table.starts_with("asset") { continue };
      
        let tokens: Vec<&str> = table.split("_").skip(1).collect();
        if !tokens.len() == 2 { continue };
        
        let ex = tokens[0];
        let t = tokens[1];
        
        if exchange == "all" { 
            tables_to_check.entry(ex.to_string())
                .or_insert(Vec::new());
        };

        if ticker == "all" { 
             tables_to_check.entry(ex.to_string())
                .or_insert(Vec::new())
                .push(t.to_string());
        };
    
    };

    let mut integrity = String::new();
    
    for (exc, pairs) in tables_to_check {
        for pair in pairs {
            let check = database_ops::integrity_check(
                &exc, &pair, db_pool.clone(), None 
            ).await;
            integrity.push_str(&format!("{}\n", check));
        }; 
    };

    integrity

}


