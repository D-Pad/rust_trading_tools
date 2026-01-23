use trading_app;
use dotenvy::dotenv;
use std::process;


#[tokio::main]
async fn main() {
  
    dotenv().ok(); 

    let app_state = match app_state::AppState::new().await {
        Ok(a) => a,
        Err(_) => {
            println!("\x1b[1;31mCould not initialize app state\x1b[0m");
            process::exit(1);
        } 
    };

    if let Err(_) = trading_app::initiailze(&app_state).await {
        process::exit(1); 
    };

    if let Err(e) = trading_app::dev_test(&app_state).await {
        println!("ERROR: {}", e);
        trading_app::error_handler(e); 
    };

}

