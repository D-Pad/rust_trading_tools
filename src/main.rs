use std::process;
use trading_app::{
    RunTimeError, 
    dev_test, 
    error_handler, 
    initialize_app_engine,
};


fn local_error_handler(err: RunTimeError) {
    error_handler(err); 
}


#[tokio::main]
async fn main() {

    let engine = match initialize_app_engine().await {
        Ok(s) => s,
        Err(e) => {
            local_error_handler(e); 
            process::exit(1);
        }
    };

    if let Err(e) = dev_test(
        &engine.state, 
        engine.database.get_pool()
    ).await 
    {
        local_error_handler(e); 
    };

}

