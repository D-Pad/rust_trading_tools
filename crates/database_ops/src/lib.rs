use std::{cmp::min};
use mysql_async::{self, prelude::*, Conn};
use reqwest;
pub mod connection;
pub use connection::{Db, DbLogin, DbError, FetchError};
pub mod kraken;


pub async fn download_new_data_to_db_table(
    exchange: &str, 
    ticker: &str,
    initial_unix_timestamp_offset: Option<u64>,
    http_client: Option<&reqwest::Client>
) -> Result<(), FetchError> {
   
    let client = match http_client {
        Some(c) => c,
        None => &reqwest::Client::new()
    }; 

    let db: Db = connection::get_db_connection(None, exchange).await?;
    let mut conn: Conn = match db.conn().await {
        Ok(c) => c,
        Err(_) => return Err(
            connection::FetchError::Db(DbError::ConnectionFailed)
        )
    };

    if exchange == "kraken" {
       
        let existing_tables: Vec<String> = match conn.exec(
            "SHOW TABLES", ()
        ).await {
            Ok(d) => d,
            Err(_) => return Err(
                connection::FetchError::Db(DbError::QueryFailed)
            )
        };
        
        if !existing_tables.contains(&ticker.to_string()) {
            kraken::add_new_db_table(
                &ticker, Some(&client), Some(conn)
            ).await?;
        };

        // let data = kraken::request_tick_data_from_kraken(
        //     ticker, 
        //     "1767850856".to_string(),
        //     Some(&client)
        // ).await?; 

        // println!("DATA: {:?}", data);
    };

    Ok(())
}


pub async fn fetch_last_row(
    exchange: &str, 
    ticker: &str,
    existing_db: Option<Db>
) -> Result<Vec<(u64, u64, f64, f64)>, FetchError> {

    let db: Db = connection::get_db_connection(existing_db, exchange).await?;
    
    let mut conn: Conn = db.conn().await?;
   
    type TickRow = Vec<(u64, u64, f64, f64)>;
    let last_row: TickRow = conn.exec(
        &format!(
            r#"SELECT id, timestamp, price, volume FROM {ticker} 
            ORDER BY id DESC LIMIT 1"#
        ), ()
    ).await?;

    Ok(last_row) 

}

pub async fn fetch_rows(
    exchange: &str, 
    ticker: &str,
    limit: Option<u64>
) -> Result<Vec<(u64, u64, f64, f64)>, FetchError> {

    let db: Db = connection::get_db_connection(None, exchange).await?;

    let limit: u64 = match limit {
        Some(i) => i,
        None => 1_000
    };

    let mut exchange_name = exchange.to_string();
    if !exchange_name.contains("_history") {
        exchange_name.push_str("_history");
    };

    let mut conn: Conn = db.conn().await?;

    let first_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM {ticker} 
            ORDER BY id LIMIT 1"#
        ), ()
    ).await {
        Ok(Some(d)) => d,
        Ok(None) | Err(_) => return Err(FetchError::Db(DbError::QueryFailed))
    };

    let last_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM {ticker} 
            ORDER BY id DESC LIMIT 1"#
        ), ()
    ).await {
        Ok(Some(d)) => d,
        Ok(None) | Err(_) => return Err(FetchError::Db(DbError::QueryFailed))
    };

    let mut query: String = String::from(
        "SELECT id, timestamp, price, volume" 
    );

    let tick_id: u64 = last_id - min(last_id - first_id, limit);

    query.push_str(&format!(
        " FROM {ticker} WHERE id >= {tick_id}"
    ));

    let rows: Vec<(u64, u64, f64, f64)> = conn.exec(query, ()).await?;

    Ok(rows)
}


pub async fn initialize(active_exchanges: Vec<String>) -> Result<(), DbError> {
   
    for exchange_name in active_exchanges {
   
        println!(
            "\x1b[1;36mInitializing {} database and tables\x1b[0m",
            &exchange_name
        );

        if exchange_name == "kraken" {
       
            let db: Db = connection::get_db_connection(None, "kraken").await?;

            let mut conn: Conn = match db.conn().await {
                Ok(d) => d,
                Err(_) => { 
                    return Err(DbError::ConnectionFailed)
                }
            };

            let table_request = conn.exec("SHOW TABLES;", ()).await;
            let existing_tables: Vec<String> = match table_request {
                Ok(d) => d,
                Err(_) => return Err(DbError::QueryFailed)
            };

            if !existing_tables.contains(&"_last_tick_history".to_string()) {
                let query: &'static str = r#"
                    CREATE TABLE IF NOT EXISTS _last_tick_history (
                        asset VARCHAR(12) NOT NULL,
                        id BIGINT NOT NULL,
                        timestamp VARCHAR(20)
                    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4; 
                "#;
                if let Err(_) = conn.query_drop(query).await {
                    return Err(DbError::QueryFailed); 
                };
            };
        };
    };

    Ok(())
    
    // 1767850856060224
    // UPDATE _last_tick_history SET
    // id = '{values['id']}',
    // timestamp = '{values['timestamp']}'
    // WHERE asset = '{values['asset']}';

}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_test() {
        assert_eq!(2, 2);
    }
}

