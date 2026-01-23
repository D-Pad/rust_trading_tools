use std::{cmp::{min, max}, fmt};
use mysql_async::{self, prelude::*, Pool, Conn};
use reqwest;

pub mod connection;
pub use connection::{Db, DbLogin, DbError, FetchError};

use crate::connection::get_table_name;
pub mod kraken;

use timestamp_tools::db_timestamp_to_date_string;


pub async fn download_new_data_to_db_table(
    exchange: &str, 
    ticker: &str,
    db_pool: Pool,
    initial_unix_timestamp_offset: u64,
    http_client: Option<&reqwest::Client>,
    show_progress: Option<bool>
) -> Result<(), DbError> {
    
    let client = match http_client {
        Some(c) => c,
        None => &reqwest::Client::new()
    }; 
    
    if exchange == "kraken" {      
        kraken::download_new_data_to_db_table(
            ticker, 
            db_pool, 
            initial_unix_timestamp_offset,
            Some(client),
            show_progress
        ).await; 
    };

    Ok(())

}


pub async fn fetch_first_tick_by_time_column(
    exchange: &str,
    ticker: &str,
    timestamp: &u64,
    db_pool: Pool
) -> Vec<(u64, u64, f64, f64)> {
    
    let query: String = format!(
        r#"
        SELECT id, time, price, volume FROM asset_{}_{}
        WHERE time >= {}
        LIMIT 1;
        "#,
        exchange,
        ticker,
        timestamp 
    );

    type TickRow = Vec<(u64, u64, f64, f64)>;
    
    match db_pool.get_conn().await {
        Ok(mut c) => {
            if let Ok(d) = c.exec(query, ()).await {
                d  
            }
            else {
                Vec::new()
            }
        },
        Err(_) => Vec::new() 
    }
}


pub async fn fetch_tables(
    db_pool: Pool 
) -> Result<Vec<String>, DbError> {

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };
 
    let tables: Vec<String> = match conn.exec("SHOW TABLES", ()).await {
        Ok(d) => d,
        Err(_) => return Err(DbError::ConnectionFailed)
    };

    Ok(tables) 

}


pub async fn fetch_first_row(
    exchange: &str, 
    ticker: &str,
    db_pool: Pool 
) -> Result<Vec<(u64, u64, f64, f64)>, DbError> {

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };
 
    type TickRow = Vec<(u64, u64, f64, f64)>;
    let last_row: TickRow = conn.exec(
        &format!(
            r#"SELECT id, time, price, volume 
            FROM asset_{exchange}_{ticker} 
            ORDER BY id LIMIT 1"#
        ), ()
    ).await?;

    Ok(last_row) 

}


pub async fn fetch_last_row(
    exchange: &str, 
    ticker: &str,
    db_pool: Pool 
) -> Result<Vec<(u64, u64, f64, f64)>, DbError> {

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };
 
    type TickRow = Vec<(u64, u64, f64, f64)>;
    let last_row: TickRow = conn.exec(
        &format!(
            r#"SELECT id, time, price, volume 
            FROM asset_{exchange}_{ticker} 
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
) -> Result<Vec<(u64, u64, f64, f64)>, DbError> {

    let table_name = get_table_name(ticker, exchange);

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };

    let limit: u64 = match limit {
        Some(i) => i,
        None => 1_000
    };

    let last_id_query: &String = &format!(
        r#"SELECT id FROM {table_name} 
        ORDER BY id DESC LIMIT 1"#
    );
    
    let last_id: u64 = match conn.exec_first::<u64, _, _>(
        last_id_query, ()
    ).await {
        Ok(Some(d)) => d,
        Ok(None) | Err(_) => return Err(
            DbError::QueryFailed(
                "Failed to fetch last tick ID".to_string() 
            )
        )
    };

    let tick_id: u64 = max(1, last_id - limit);
    
    let query: String = format!(
        r#"
        SELECT id, timestamp, price, volume
        FROM {table_name} WHERE id >= {tick_id};
        "#,
    );

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

            let show_table_query: String = "SHOW TABLES".to_string();
            let table_request = conn.exec(show_table_query, ()).await;
            let existing_tables: Vec<String> = match table_request {
                Ok(d) => d,
                Err(_) => return Err(DbError::QueryFailed(
                    "Failed to fetch table names".to_string() 
                ))
            };

            if !existing_tables.contains(&"_last_tick_history".to_string()) {

                let query: &'static str = r#"
                    CREATE TABLE IF NOT EXISTS _last_tick_history (
                        asset VARCHAR(12) NOT NULL PRIMARY KEY,
                        next_tick_id BIGINT NOT NULL,
                        time VARCHAR(20)
                    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4; 
                "#;
                if let Err(_) = conn.query_drop(query).await {
                    return Err(DbError::QueryFailed(
                            "Failed to create '_last_tick_history'".to_string()
                        )
                    ); 
                };
            };
        };
    };

    Ok(())
    
}


pub async fn initialize(
    active_exchanges: Vec<String>,
    db_pool: Pool
) -> Result<(), DbError> {

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


pub struct DatabaseIntegrity {
    pub table_name: String,
    pub is_ok: bool,
    pub first_tick_id: u64,
    pub last_tick_id: u64,
    pub first_date: String, 
    pub last_date: String, 
    pub total_ticks: u64,
    pub missing_ticks: Vec<u64>,
    pub error: String 
}

impl fmt::Display for DatabaseIntegrity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       
        fn col(passes: bool) -> &'static str {
            match passes {
                true => "\x1b[32m",
                false => "\x1b[31m",
            }
        }
        
        write!(f, "\x1b[1;36mDatabase Integrity:\x1b[0m\n");
        write!(f, "  \x1b[33mtable_name   \x1b[0m: {}\n", 
            self.table_name);
        write!(f, "  \x1b[33mis_ok        \x1b[0m: {}{}\n", 
            col(self.is_ok), self.is_ok);
        write!(f, "  \x1b[33mfirst_tick_id\x1b[0m: {}\n", self.first_tick_id);
        write!(f, "  \x1b[33mlast_tick_id \x1b[0m: {}\n", self.last_tick_id);
        write!(f, "  \x1b[33mfirst_date   \x1b[0m: {}\n", self.first_date);
        write!(f, "  \x1b[33mlast_date    \x1b[0m: {}\n", self.last_date);
        write!(f, "  \x1b[33mtotal_ticks  \x1b[0m: {}\n", self.total_ticks);
        
        if self.missing_ticks.len() > 0 {
            write!(f, "  \x1b[33mmissing_ticks\x1b[0m: [\n\x1b[1;31m");
            for missing in &self.missing_ticks {
                write!(f, "    {}\n", missing);
            };
            write!(f, "\x1b[0m\n");
        }
        else {
            write!(f, "  \x1b[33mmissing_ticks\x1b[0m: \x1b[32mnone\x1b[0m\n");
        };

        if self.error.len() > 0 {
            write!(f, "  \x1b[33merror\x1b[0m: \x1b[1:31m{}", self.error)
        }
        else {
            Ok(())
        }
    }
} 


pub async fn integrity_check(
    exchange: &str, 
    ticker: &str,
    db_pool: Pool,
    tick_step_value: Option<u16>
) -> DatabaseIntegrity {

    let table_name = get_table_name(exchange, ticker); 
    
    let mut dbi: DatabaseIntegrity = DatabaseIntegrity { 
        table_name: table_name.clone(), 
        is_ok: false, 
        first_tick_id: 0, 
        last_tick_id: 0,
        first_date: String::new(),
        last_date: String::new(),
        total_ticks: 0,
        missing_ticks: Vec::new(), 
        error: String::new() 
    };

    let mut conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => {
            dbi.error.push_str("Failed to establish a Database Connection");
            return dbi
        } 
    };    

    (dbi.first_tick_id, dbi.first_date) = match fetch_first_row(
        exchange, ticker, db_pool.clone()
    ).await {
        Ok(d) => (d[0].0, db_timestamp_to_date_string(d[0].1)),
        Err(_) => {
            dbi.error.push_str("Failed to fetch first tick ID");
            return dbi
        }
    };
     
    (dbi.last_tick_id, dbi.last_date) = match fetch_last_row(
        exchange, ticker, db_pool.clone()
    ).await {
        Ok(d) => (d[0].0, db_timestamp_to_date_string(d[0].1)),
        Err(_) => {
            dbi.error.push_str("Failed to fetch last tick ID"); 
            return dbi 
        }
    };

    const DEFAULT_STEP_VALUE: u16 = 10000;
    let step_val = match tick_step_value {
        Some(s) => s,
        None => DEFAULT_STEP_VALUE
    };

    let range_vals = dbi.first_tick_id..dbi.last_tick_id;
    let mut last_id = 0;
    
    for start in range_vals.step_by(step_val as usize) {
       
        let end = min(start + (step_val as u64) - 1, dbi.last_tick_id); 
        
        let query = format!(
            "SELECT id FROM {} WHERE id BETWEEN {} AND {}",
            table_name,
            start,
            end
        );
        
        let tick_slice: Vec<u64> = match conn.exec(&query, ()).await {
            Ok(d) => d,
            Err(_) => {
                dbi.error.push_str("Failed to fetch tick slice");
                return dbi
            }
        };

        dbi.total_ticks += tick_slice.len() as u64;

        for tick_id in tick_slice {
            if last_id != 0 && tick_id != last_id + 1 {
                for i in last_id..tick_id {
                    dbi.missing_ticks.push(i);
                }
            };
            last_id = tick_id;
        }; 
    
    }; 

    dbi.is_ok = true;
    dbi

}


