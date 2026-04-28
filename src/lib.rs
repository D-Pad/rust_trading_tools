use std::{
    fs,
};

use app_core::{
    DataResponse, Response, config::SystemPaths, database_ops::fetch_enabled_assets, engine::{Engine, Server}, errors::{ConfigError, RunTimeError, error_handler}, initialize_app_engine, strategies::*
};
use tui::{TerminalInterface};
use webserver::{WebServer};

use dotenvy;


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
async fn dev_testing(engine: &Engine) { 
    println!("\x1b[1;33m------------- DEVELOPMENT MODE -------------\x1b[0m");
    let _ = fetch_enabled_assets(engine.database.get_pool()).await;
}


pub async fn app_start() -> i32 {

    dotenvy::dotenv().ok(); 

    let mut exit_code: i32 = 0;

    match SystemPaths::new() {
        Ok(s) => {
            if let Err(_) = first_time_setup(&s) {
                exit_code = 2;
                return exit_code
            };  
        },
        Err(_) => {
            exit_code = 2;
            return exit_code 
        }
    };
    
    let mut engine: Engine = match initialize_app_engine().await {
        Ok(s) => s,
        Err(e) => {
            error_handler(e); 
            exit_code = 2;
            return exit_code
        }
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


        if let Err(_) = fs::create_dir_all(&paths.strategy_templates) {
            return Err(ConfigError::MissingDirectory(
                "Failed to create 'dtrade/strategies' directory"
            ));
        };

    }; 

    Ok(())
}

