use trading_app::{self, RunTimeError, error_handler};
use std::process;


fn local_error_handler(err: RunTimeError) {
    error_handler(err); 
}


#[tokio::main]
async fn main() {

    let app_state = match trading_app::initiailze().await {
        Ok(s) => s,
        Err(e) => {
            local_error_handler(e); 
            process::exit(1);
        }
    };

    if let Err(e) = trading_app::dev_test(&app_state).await {
        local_error_handler(e); 
    };

}

