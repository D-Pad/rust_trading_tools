pub use app_core::*;
pub use app_core::{errors::error_handler};
use sqlx::PgPool;


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
pub async fn dev_test(
    state: &AppState, 
    time_offset: u64,
    client: &reqwest::Client,
    db_pool: PgPool,
) -> Result<(), RunTimeError> {

    // let dbi = database_ops::integrity_check(
    //     "kraken", 
    //     "BTCUSD", 
    //     state.database.get_pool(), 
    //     None).await;

    database_ops::kraken::add_new_db_table(
        "BTCUSD", 
        state.time_offset(),
        client,
        db_pool 
    ).await;

    Ok(())

}


