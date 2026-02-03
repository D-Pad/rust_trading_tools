use std::process;
use trading_app::{app_start, test_fn};


#[tokio::main]
async fn main() {

    let exit_code = app_start().await;
    process::exit(exit_code); 

}

