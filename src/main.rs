use std::process;
use trading_app::{error_handler, RunTimeError, dev_test};


fn local_error_handler(err: RunTimeError) {
    error_handler(err); 
}


#[tokio::main]
async fn main() {

    let app_state = match app_core::initiailze().await {
        Ok(s) => s,
        Err(e) => {
            local_error_handler(e); 
            process::exit(1);
        }
    };

    if let Err(e) = dev_test(&app_state).await {
        local_error_handler(e); 
    };

}

