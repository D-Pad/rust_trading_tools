use std::process;
use trading_app::{
    RunTimeError,
    Response,
    DataResponse,
    error_handler, 
    initialize_app_engine,
};


fn local_error_handler(err: RunTimeError) {
    error_handler(err); 
}


#[tokio::main]
async fn main() {

    let mut engine = match initialize_app_engine().await {
        Ok(s) => s,
        Err(e) => {
            local_error_handler(e); 
            process::exit(2);
        }
    };

    let response = match engine.execute_commands().await {
        Ok(d) => d,
        Err(e) => {
            let exit_code: i32 = match e {
                RunTimeError::Init(_) => 2,
                RunTimeError::Arguments(_) => 3,
                RunTimeError::DataBase(_) => 4,
                RunTimeError::Bar(_) => 5,
            };
            local_error_handler(e);
            process::exit(exit_code);
        }
    };

    if let Response::Data(data) = response {
        match data {
            DataResponse::Bars(_) => {
                    
            }
        }
    };

}

