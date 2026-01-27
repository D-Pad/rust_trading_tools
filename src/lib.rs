pub use app_core::*;


// ------------------------ MAIN PROGRAM FUNCTIONS ------------------------- //
pub async fn dev_test(state: &AppState) -> Result<(), RunTimeError> {

    // let dbi = database_ops::integrity_check(
    //     "kraken", 
    //     "BTCUSD", 
    //     state.database.get_pool(), 
    //     None).await;

    database_ops::kraken::add_new_db_table(
        "BTCUSD", 
        state.time_offset(),
        None,
        state.database.get_pool() 
    ).await;

    Ok(())

}


