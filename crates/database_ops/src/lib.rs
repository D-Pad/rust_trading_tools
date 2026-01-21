use std::{cmp::min};
use mysql_async::{self, prelude::*, Pool, Conn};
use reqwest;
use timestamp_tools::*;
pub mod connection;
pub use connection::{Db, DbLogin, DbError, FetchError};
pub mod kraken;


pub async fn download_new_data_to_db_table(
    exchange: &str, 
    ticker: &str,
    initial_unix_timestamp_offset: u64,
    db_pool: Pool,
    http_client: Option<&reqwest::Client>
) -> Result<(), FetchError> {
  
    let current_time: u64 = get_current_unix_timestamp();
    let start_timestamp: u64 = current_time - initial_unix_timestamp_offset;

    let client = match http_client {
        Some(c) => c,
        None => &reqwest::Client::new()
    }; 

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(FetchError::Db(DbError::ConnectionFailed))
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
                &ticker, 
                start_timestamp, 
                Some(&client),
                db_pool.clone()
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
    db_pool: Pool 
) -> Result<Vec<(u64, u64, f64, f64)>, FetchError> {

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(FetchError::Db(DbError::ConnectionFailed))
    };
  
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
    limit: Option<u64>,
    db_pool: Pool
) -> Result<Vec<(u64, u64, f64, f64)>, FetchError> {

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(FetchError::Db(DbError::ConnectionFailed))
    };

    let limit: u64 = match limit {
        Some(i) => i,
        None => 1_000
    };

    let first_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM asset_{exchange}_{ticker} 
            ORDER BY id LIMIT 1"#
        ), ()
    ).await {
        Ok(Some(d)) => d,
        Ok(None) | Err(_) => return Err(FetchError::Db(DbError::QueryFailed))
    };

    let last_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM asset_{exchange}_{ticker} 
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
        " FROM asset_{exchange}_{ticker} WHERE id >= {tick_id}"
    ));

    let rows: Vec<(u64, u64, f64, f64)> = conn.exec(query, ()).await?;

    Ok(rows)
}


pub async fn first_time_setup(
    active_exchanges: &Vec<String>, 
    db_pool: Pool 
) -> Result<(), DbError> {
   
    for exchange_name in active_exchanges {

        if exchange_name == "kraken" { 

            let mut conn: Conn = match db_pool.get_conn().await {
                Ok(c) => c,
                Err(_) => return Err(DbError::ConnectionFailed)
            };

            let table_request = conn.exec("SHOW TABLES;", ()).await;
            let existing_tables: Vec<String> = match table_request {
                Ok(d) => d,
                Err(_) => return Err(DbError::QueryFailed)
            };

            if !existing_tables.contains(&"_last_tick_history".to_string()) {
   
                println!(
                    "\x1b[1;33mInitializing {} database meta tables\x1b[0m",
                    &exchange_name
                );

                let query: &'static str = r#"
                    CREATE TABLE IF NOT EXISTS _last_tick_history (
                        asset VARCHAR(12) NOT NULL PRIMARY KEY,
                        next_tick_id BIGINT NOT NULL,
                        time VARCHAR(20)
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


pub async fn initialize(
    active_exchanges: Vec<String>,
    db_pool: Pool
) -> Result<(), DbError> {

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => {
            return Err(DbError::ConnectionFailed)
        }
    };

    first_time_setup(&active_exchanges, db_pool.clone()).await?;
    
    for exchange_name in active_exchanges {
   
        println!(
            "\x1b[1;33mUpdating existing {} database tables\x1b[0m",
            &exchange_name
        );

        if exchange_name == "kraken" {


        };

    };

    Ok(())

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_test() {
        assert_eq!(2, 2);
    }
}

