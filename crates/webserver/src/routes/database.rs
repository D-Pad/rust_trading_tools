use std::{
    sync::Arc,
    collections::HashMap
};

use axum::{
    extract::State,
    Json
};
use app_core::{
    engine::Engine,
    database_ops::fetch_exchanges_and_pairs_from_db
};


pub async fn db_status(State(engine): State<Arc<Engine>>) 
    -> &'static str
{
    "TESTING"
}


pub async fn db_tables(State(engine): State<Arc<Engine>>) 
    -> Json<HashMap<String, Vec<String>>> 
{
    let pool = engine.database.get_pool();
    let tables = fetch_exchanges_and_pairs_from_db(pool).await; 
    Json(tables)
}



