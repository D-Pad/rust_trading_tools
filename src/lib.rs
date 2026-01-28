pub use app_core::*;
pub use app_core::{errors::error_handler};
// use sqlx::PgPool;


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
pub async fn dev_test(
    // state: &AppState, 
    // client: &reqwest::Client,
    // db_pool: PgPool,
) -> Result<(), RunTimeError> {

    parse_args();
    // database_ops::kraken::add_new_db_table(
    //     "BTCUSD", 
    //     state.time_offset(),
    //     client,
    //     db_pool 
    // ).await;

    Ok(())

}


