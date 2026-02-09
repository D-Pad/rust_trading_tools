pub use app_core::*;
pub use app_core::{
    errors::error_handler, 
    engine::{Engine, Server}, 
    RunTimeError,
    Response,
    DataResponse,
    initialize_app_engine,
};
use tui::{TerminalInterface};

// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
async fn dev_testing(engine: Engine) {
    
    println!("\x1b[1;33m------------- DEVELOPMENT MODE -------------\x1b[0m");

}


pub async fn app_start() -> i32 {

    let mut exit_code: i32 = 0;

    let mut engine: Engine = match initialize_app_engine().await {
        Ok(s) => s,
        Err(e) => {
            error_handler(e); 
            exit_code = 2;
            return exit_code
        }
    };

    if engine.args.dev_mode {
        dev_testing(engine).await; 
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
                };
                error_handler(e);
                return exit_code;
            }
        };

        if let Response::Data(data) = response {
            match data {
                DataResponse::Bars(_) => {
                        
                }
            }
        };

        // Start the server if 'start' was passed as the first argument 
        if let Server::CLI = engine.op_mode {
            let mut tui = TerminalInterface::new(engine).await;
            tui.run().await;
        }

        else if let Server::HTTP = engine.op_mode {
            todo!();
        };
    };

    exit_code

}


