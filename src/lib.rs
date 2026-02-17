use dotenvy;

pub use app_core::*;
pub use app_core::{
    errors::{error_handler, ConfigError}, 
    engine::{Engine, Server},
    app_state::{SystemPaths},
    indicators::*,
    Chart,
    RunTimeError,
    Response,
    DataResponse,
    initialize_app_engine,
    build_candles,
};
use tui::{TerminalInterface};
use webserver::{WebServer};

use std::{
    fs,
};


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
async fn dev_testing(engine: &Engine) { 
    println!("\x1b[1;33m------------- DEVELOPMENT MODE -------------\x1b[0m");
   
    println!("\x1b[33mBuilding candles...\x1b[0m");
    let candles = match build_candles(
        "kraken", "BTCUSD", "5m", engine.database.get_pool()
    ).await {
        Ok(c) => c,
        Err(_) => return 
    };

    println!("\x1b[33mBuilding chart...\x1b[0m");
    let mut chart = Chart::new(candles);

    println!("\x1b[33mAdding moving averages...\x1b[0m");
    let ma_inputs: Vec<MaInputs> = Vec::from([
        MaInputs::SMA(SmaInputs::default())
    ]);

    chart.indicator_set.set_moving_averages(ma_inputs);
    chart.populate_indicator_values();

    if let Some(ma_container) = chart.indicator_set.ma_container {
   
        for ma in ma_container.moving_averages {
            println!("MA: {}", ma);
        };

    };

}


pub async fn app_start() -> i32 {

    dotenvy::dotenv().ok(); 

    let mut exit_code: i32 = 0;

    let mut engine: Engine = match initialize_app_engine().await {
        Ok(s) => s,
        Err(e) => {
            error_handler(e); 
            exit_code = 2;
            return exit_code
        }
    };

    if let Err(_) = first_time_setup(&engine.state.paths) {
        exit_code = 2;
        return exit_code
    };

    if engine.args.dev_mode {
        dev_testing(&engine).await; 
    }
    else {
        
        let response = match engine.execute_commands().await {
            Ok(d) => d,
            Err(e) => {
                exit_code = match e {
                    RunTimeError::Init(_) => 2,
                    RunTimeError::Arguments(_) => 3,
                    RunTimeError::DataBase(_) => 4,
                    RunTimeError::Bar(_) => 5,
                    RunTimeError::TuiError => 6,
                };
                error_handler(e);
                return exit_code;
            }
        };

        if let Response::Data(data) = response {
            match data {
                DataResponse::Bars(b) => {
                    println!("{b}"); 
                }
            }
        };

        // Start the HTTP server is 'start --http' was passed.
        if let Server::HTTP = engine.op_mode {
            
            match WebServer::new(engine).await {
                Ok(server) => {
                    
                    if let Err(_) = server.serve().await {
                        exit_code = 7;
                    }; 

                },
                Err(_) => { exit_code = 7 } 
            };

        }

        // Start the TUI if 'start' was passed without a flag.
        else if let Server::TUI = engine.op_mode {
            let mut tui = TerminalInterface::new(engine).await;
            if let Err(_) = tui.run().await {
                exit_code = 6;
            };
        };

    };

    exit_code

}


fn first_time_setup(paths: &SystemPaths) -> Result<(), ConfigError> {
   
    if !paths.base.exists() {
        
        if let Err(_) = fs::create_dir_all(&paths.base) {
            return Err(ConfigError::MissingDirectory(
                "Failed to create 'dtrade' directory"
            ));
        };

        if let Err(_) = fs::create_dir_all(&paths.candle_data) {
            return Err(ConfigError::MissingDirectory(
                "Failed to create 'dtrade/candle_data' directory"
            ));
        };

    }; 

    Ok(())
}

