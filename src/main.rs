use trading_app;
use dotenvy::dotenv;
use std::process;


#[tokio::main]
async fn main() {
  
    dotenv().ok(); 

    let config = match trading_app::config::load_config() {
        Ok(c) => c,
        Err(_) => {
            println!("\x1b[1;31mCould not load config\x1b[0m");
            process::exit(1);
        } 
    };

    if let Err(_) = trading_app::initiailze(&config).await {
        process::exit(1); 
    };

    trading_app::dev_test().await;
}

