use trading_app;
use dotenvy::dotenv;
use std::process;


#[tokio::main]
async fn main() {
  
    dotenv().ok(); 

    let app_state = match trading_app::AppState::new().await {
        Ok(a) => a,
        Err(_) => {
            println!("\x1b[1;31mCould not initialize app state\x1b[0m");
            process::exit(1);
        } 
    };

    if let Err(_) = trading_app::initiailze(&app_state).await {
        println!("INIT FAILURE"); 
        process::exit(1); 
    };

    // trading_app::dev_test(&config).await;

}

