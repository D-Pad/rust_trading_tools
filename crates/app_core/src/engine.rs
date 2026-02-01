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


const HELP_STRING: &'static str = r#"
NAME
    dtrade — Cryptocurrency data management and candle builder tool

SYNOPSIS
    dtrade COMMAND [OPTIONS]...

    dtrade --help | -h
    dtrade --version

DESCRIPTION
    dtrade is a command-line tool for managing cryptocurrency pair data in a database
    and building OHLCV candles from exchange data.

    It supports multiple sub-commands for database maintenance and candle generation.

COMMANDS
    candles EXCHANGE TICKER PERIOD [--integrity | -i]
        Build OHLCV candles for the given exchange, trading pair and timeframe.

        Examples:
            dtrade candles kraken btcusd 1h
            dtrade candles binance ethusdt 15m -i

        Arguments:
            EXCHANGE     Name of the exchange (kraken, binance, ...)
            TICKER       Trading pair symbol (btcusd, ethusdt, solusd, ...)
            PERIOD       Candle timeframe (1m, 5m, 15m, 1h, 4h, 1d, ...)

        Options:
            --integrity, -i
                Perform database integrity check before/after building candles

    database --add-pairs EXCHANGE TICKER [TICKER...]
        Add one or more trading pairs to the database for the given exchange.

        Example:
            dtrade database --add-pairs kraken SOLUSD ETHUSD XRPUSD

    database --rm-pairs EXCHANGE TICKER [TICKER...]
        Remove one or more trading pairs from the database for the given exchange.

        Example:
            dtrade database --rm-pairs kraken SOLUSD

    database --update
        Update/fetch latest pair metadata and information from exchanges.

        Example:
            dtrade database --update

    database --integrity [EXCHANGE [TICKER]]
        Check database integrity (missing candles, duplicates, gaps, etc.).

        When no arguments are given, checks all exchanges and pairs.
        When only EXCHANGE is given, checks all pairs on that exchange.
        When both are given, checks only the specified pair.

        Examples:
            dtrade database --integrity
            dtrade database --integrity kraken
            dtrade database --integrity kraken BTCUSD

    start
        Start the trading server / background service.

OPTIONS (global)
    --help, -h
        Show this help message and exit.

    --version
        Show version information and exit.

EXAMPLES
    Fetch and add new pairs from Kraken:
        dtrade database --add-pairs kraken SOLUSD ETHUSD

    Build 1-hour candles for BTC/USD on Kraken with integrity check:
        dtrade candles kraken btcusd 1h --integrity

    Full integrity check across everything:
        dtrade database --integrity

    Update pair metadata for all configured exchanges:
        dtrade database --update

    Mixed command (integrity check + update):
        dtrade database --integrity kraken BTCUSD --update

EXIT STATUS
    0     Success
    1     General error / invalid usage
    2     App initialization error 
    3     Parser error (unknown flags, missing arguments, ...)
    4     Database connection / query failure
    5     Candle builder error

BUGS / LIMITATIONS
    Currently only Kraken is fully tested for pair adding/removal.
    More exchanges will be added in future versions.
    --integrity on very large datasets may be slow.

SEE ALSO
    Rust crates: sqlx, reqwest, clap (for future refactors), tokio
    Related projects: ccxt (exchange library inspiration)

Report bugs or suggestions at: <https://github.com/D-Pad/rust_trading_tools/issues>
"#;


pub enum Server {
    CLI,
    HTTP,
    OneShot,
}

impl std::fmt::Display for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Server::CLI => { write!(f, "CLI Mode") },
            Server::HTTP => { write!(f, "HTTP Mode") },
            Server::OneShot => { write!(f, "One-Shot Mode") }
        }
    }
}


pub struct Engine {
    pub state: AppState,
    pub database: Db,
    pub request_client: Client,
    pub args: ParsedArgs,
    pub op_mode: Server,
}

impl Engine {
   
    pub fn new(database: Db) -> Result<Self, RunTimeError> {

        let state: AppState = AppState::new()
            .map_err(|e| RunTimeError::Init(e))?;

        let request_client: Client = Client::new();

        let args: ParsedArgs = parse_args(None);

        if let Some(e) = args.parser_error {
            return Err(RunTimeError::Arguments(e))
        };

        let op_mode: Server = Server::OneShot;

        Ok(Engine { state, database, request_client, args, op_mode })

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

            Command::StartServer { http } => {
                if http {
                    self.op_mode = Server::HTTP;
                }
                else {
                    self.op_mode = Server::CLI;
                };
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
                let check = db_integrity_check(
                    &exchange, 
                    &ticker, 
                    self.database.get_pool() 
                ).await;

                println!("{check}");
                Ok(Response::Ok)
            },

            Command::Help => {
                println!("{}", HELP_STRING);
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


