use trading_app;
use dotenvy::dotenv;
use std::process;


#[tokio::main]
async fn main() {
  
    dotenv().ok(); 

    let config_path: &'static str = "../config.toml";

    let _config = match trading_app::config::load_toml_config(config_path) {
        Ok(c) => c,
        Err(_) => {
            println!("\x1b[1;31mCould not load config\x1b[0m");
            process::exit(1);
        } 
    };

    // let data = trading_app::add_new_data_to_db_table("XBTUSD");

    // let bars = trading_app::fetch_data_and_build_bars(
    //     "kraken", "SOLUSD", "100t", None 
    // ).await;

    let last = trading_app::dev_test().await;
}

