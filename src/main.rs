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
            process::exit(1);
        }
    };

    let response = match engine.execute_commands().await {
        Ok(d) => d,
        Err(e) => {
            local_error_handler(e);
            process::exit(2);
        }
    };

    if let Response::Data(data) = response {
        match data {
            DataResponse::Bars(_) => {
                    
            }
        }
    };

    // if let Err(e) = dev_test().await {
    //     local_error_handler(e); 
    // };

}

