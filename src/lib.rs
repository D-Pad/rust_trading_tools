use app_state::{AppState, InitializationError};


// Modules and crates for main program functionality
pub mod bar_builders;
use crate::bar_builders::{fetch_data_and_build_bars};


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
pub async fn dev_test(state: &AppState) {

    let time_offset: u64 = state
        .config
        .data_download
        .cache_size_settings_to_seconds();

    let _ = database_ops::kraken::add_new_db_table(
        "BTCUSD", 
        time_offset, 
        None, 
        state.database.get_pool() 
    ).await;

    // database_ops::download_new_data_to_db_table(
    //     "kraken", "BTCUSD", Some(time_offset), None 
    // ).await;

}


pub async fn initiailze(state: &AppState) -> Result<(), InitializationError> {

    let mut active_exchanges: Vec<String> = Vec::new();
   
    for (exchange, activated) in &state.config.supported_exchanges.active {
        if *activated { active_exchanges.push(exchange.clone()) }
    };
   
    let pool = state.database.get_pool();

    if let Err(_) = database_ops::initialize(
        active_exchanges, pool
    ).await {
        return Err(InitializationError::InitFailure) 
    }; 

    Ok(())
}


