use std::{
    collections::HashMap, 
    time::{SystemTime, UNIX_EPOCH}
};

use reqwest;
use serde::Deserialize;
use mysql_async::{prelude::Queryable, Pool, Conn};
use tokio::time::{sleep, Duration};

use timestamp_tools::{get_current_unix_timestamp};
use crate::{DbError, FetchError};
pub use crate::connection;


// Tick data structs
#[derive(Deserialize, Debug)]
pub struct TickDataResponse {
    error: Vec<String>,
    result: Option<TickDataResult>,
}

#[derive(Deserialize, Debug)]
struct TickDataResult {
    #[serde(flatten)]
    trades: HashMap<String, Vec<Trade>>,
    last: String,  // The 'since' value for pagination 
}

#[derive(Deserialize, Debug, Clone)]
struct Trade {
    price: String,         // Price as string
    volume: String,        // Volume as string
    time: f64,             // Unix timestamp (seconds with decimal)
    buy_sell: String,      // "b" or "s"
    market_limit: String,  // "m" or "l"
    miscellaneous: String, // Usually an empty string
    tick_id: u64,
}

impl Trade {
    pub fn to_db_row(&self) -> String {
        format!(
            "({}, {}, {}, {}, '{}', '{}', '{}')",
            self.tick_id,
            self.price, 
            self.volume,
            (self.time * 1_000_000.0) as u128,
            self.buy_sell,
            self.market_limit,
            self.miscellaneous
        ) 
    }
}

// Token info structs
#[derive(Debug, Deserialize)]
pub struct AssetPairsResponse {
    pub error: Vec<String>,
    pub result: HashMap<String, AssetPairInfo>,
}

#[derive(Debug, Deserialize)]
pub struct AssetPairInfo {
    pub altname: String,
    pub wsname: String,

    pub aclass_base: String,
    pub base: String,

    pub aclass_quote: String,
    pub quote: String,

    pub lot: String,

    pub cost_decimals: u32,
    pub pair_decimals: u32,
    pub lot_decimals: u32,
    pub lot_multiplier: u32,

    pub leverage_buy: Vec<u8>,
    pub leverage_sell: Vec<u8>,

    pub fees: Vec<[f64; 2]>,
    pub fees_maker: Option<Vec<[f64; 2]>>,

    pub fee_volume_currency: String,

    pub margin_call: u32,
    pub margin_stop: u32,

    pub ordermin: String,
    pub costmin: String,
    pub tick_size: String,

    pub status: String,

    pub long_position_limit: u32,
    pub short_position_limit: u32,
}

// Error enums
#[derive(Debug)]
pub enum RequestError {
    Http(reqwest::Error),
    BadStatus(reqwest::StatusCode),
    Deserialize(serde_json::Error),
    RequestFailed(String),
    NoData,
}

impl From<reqwest::Error> for RequestError {
    fn from(e: reqwest::Error) -> Self {
        RequestError::Http(e)
    }
}

impl From<serde_json::Error> for RequestError {
    fn from(e: serde_json::Error) -> Self {
        RequestError::Deserialize(e)
    }
}


pub async fn add_new_db_table(
    ticker: &str,
    start_date_unix_timestamp_offset: u64,
    http_client: Option<&reqwest::Client>,
    db_pool: Pool 
) -> Result<(), FetchError> {

    let table_name: String = format!("asset_kraken_{}", ticker);
           
    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(FetchError::Db(DbError::ConnectionFailed))
    };

    let show_table_query: String = "SHOW TABLES".to_string();
    let existing_tables: Vec<String> = match conn.exec(
        show_table_query, ()
    ).await {
        Ok(d) => d,
        Err(_) => return Err(
            connection::FetchError::Db(
                DbError::QueryFailed(
                    "Failed to fetch table names".to_string() 
                )
            )
        )
    };

    if existing_tables.contains(&table_name) {
        return Err(
            connection::FetchError::Db(
                DbError::TableCreationFailed(
                    format!("{} table already exists", ticker)
                )
            )
        )
    };
    
    const INIT_TIME_OFFSET: u64 = 60 * 60 * 24 * 14;  // 2 weeks of seconds
    
    let current_ts = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(t) => t.as_secs(),
        Err(_) => return Err(
            connection::FetchError::SystemError(
                "Couldn't retrieve system time".to_string()
            )
        ) 
    };

    let mut initial_fetch_time: u64 = current_ts - INIT_TIME_OFFSET;

    let initial_trade: Trade = match request_tick_data_from_kraken(
        ticker, 
        initial_fetch_time.to_string(),
        http_client
    ).await {
        Ok(d) => {
       
            let result = d.result.ok_or_else(|| {
                connection::FetchError::Api(
                    RequestError::RequestFailed(
                        "Could not fetch trade data".to_string()
                    )
                )
            })?;

            let trades_vec = result 
                .trades 
                .values() 
                .next() 
                .ok_or_else(|| {
                    connection::FetchError::SystemError(
                        "No trades detected in response".to_string()
                    )
                })?;

            trades_vec.last().cloned().ok_or_else(|| {
                connection::FetchError::SystemError(
                    "Trades list was empty".to_string()
                )
            })?

        },
        Err(_) => return Err(
            connection::FetchError::Api(
                RequestError::RequestFailed(
                    "Could not fetch trade data".to_string()
                )
            )
        )
    };

    let price_string = initial_trade.price.to_string();
    let left_digits = match price_string.split_once(".") {
        Some((left, _right)) => left.len(),
        None => price_string.len()
    };

    let price_digits_for_db_table = left_digits + 5;

    sleep(Duration::from_millis(500)).await;
    
    let tick_info = match request_asset_info_from_kraken(
        &ticker,
        http_client
    ).await {
        Ok(d) => d,
        Err(e) => return Err(
            connection::FetchError::Api(RequestError::Http(e))
        ) 
    };
    
    let create_table_query: String = format!(r#"
        CREATE TABLE IF NOT EXISTS {} (
            id BIGINT PRIMARY KEY,
            price DECIMAL({},{}) NOT NULL, 
            volume DECIMAL({},{}) NOT NULL, 
            time BIGINT NOT NULL, 
            buy_sell CHAR(1) NOT NULL, 
            market_limit CHAR(1) NOT NULL, 
            misc VARCHAR(16)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
        "#,
        table_name,
        price_digits_for_db_table,
        tick_info.pair_decimals,
        price_digits_for_db_table,
        tick_info.lot_decimals
    ); 
   
    if let Err(_) = conn.query_drop(create_table_query).await {
        return Err(FetchError::Db(DbError::TableCreationFailed(
            format!("Failed to create asset_kraken_{} table", ticker) 
        ))); 
    };

    let initial_time_stamp_query: String = format!(r#"
        INSERT INTO _last_tick_history (asset, next_tick_id, time) 
        VALUES ('{}', 0, 0);"#, ticker);

    if let Err(_) = conn.query_drop(initial_time_stamp_query).await {
        return Err(
            FetchError::Db(
                DbError::QueryFailed(
                    format!(
                        "Failed to fetch _last_tick_history for {}",
                        ticker
                    )
                )
            )
        ); 
    };

    sleep(Duration::from_millis(500)).await;
    
    initial_fetch_time = current_ts - start_date_unix_timestamp_offset;  
   
    let initial_data: TickDataResponse = match request_tick_data_from_kraken(
        ticker, 
        initial_fetch_time.to_string(),
        http_client
    ).await {
        Ok(d) => d,
        Err(e) => return Err(FetchError::Api(e))
    };

    match write_data_to_db_table(ticker, initial_data, db_pool.clone()).await {
        Ok(_) => Ok(()),
        Err(e) => Err(FetchError::Db(e)) 
    }
}


pub async fn download_new_data_to_db_table(
    ticker: &str,
    db_pool: Pool,
    initial_unix_timestamp_offset: u64,
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

    let show_table_query: String = "SHOW TABLES".to_string();
    let existing_tables: Vec<String> = match conn.exec(
        show_table_query, ()
    ).await {
        Ok(d) => d,
        Err(_) => return Err(
            FetchError::Db(DbError::QueryFailed(
                "Failed to fetch table names".to_string()
            ))
        )
    };
    
    let table_name = format!("asset_kraken_{}", ticker);
    if !existing_tables.contains(&table_name) {
        add_new_db_table(
            &ticker, 
            start_timestamp, 
            Some(&client),
            db_pool.clone()
        ).await?;
    };

    // Get the last recorded timestamp from _last_tick_history
    let query: String = format!(
        r#"
        SELECT next_tick_id, time 
        FROM _last_tick_history
        WHERE asset = '{}' 
        "#,
        ticker
    ); 
    let valid_row: Vec<(u64, String)> = match conn.exec(
        query, ()
    ).await {
        Ok(r) => r,
        Err(_) => return Err(FetchError::Db(DbError::QueryFailed(
            "Couldn't fetch last tick time from _last_tick_history".to_string()
        ))) 
    };

    let (next_tick_id, timestamp) = match valid_row.len() > 0 {
        true => (valid_row[0].0, &valid_row[0].1),
        false => return Err(FetchError::Db(DbError::QueryFailed(
            "Couldn't fetch last tick time from _last_tick_history".to_string()
        )))
    }; 

    println!("LAST TICK: {} / {}", next_tick_id, timestamp);

    Ok(())

}


pub async fn request_tick_data_from_kraken(
    ticker: &str, 
    since_unix_timestamp: String, 
    http_client: Option<&reqwest::Client> 
) -> Result<TickDataResponse, RequestError> {
    
    let url = format!(
        "https://api.kraken.com/0/public/Trades?pair={}&since={}", 
        ticker,
        since_unix_timestamp
    );
  
    let client = match http_client {
        Some(c) => c,
        None => &reqwest::Client::new()
    };
    
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(RequestError::BadStatus(response.status()));
    }

    let raw_text = response.text().await?;

    let kraken_resp: TickDataResponse = serde_json::from_str(&raw_text)
        .map_err(|e| {
            println!("\x1b[1;31mDeserialization error:\n\x1b[0m{}", e);
            RequestError::Deserialize(e) 
        })?;

    if kraken_resp.error.len() > 0 {
        return Err(RequestError::RequestFailed(
            format!("Request failed: {:?}", kraken_resp.error)
        ))
    }; 

    Ok(kraken_resp)

}


pub async fn request_asset_info_from_kraken(
    ticker: &str,
    http_client: Option<&reqwest::Client> 
) 
  -> Result<AssetPairInfo, reqwest::Error> {
    
    let url = format!(
        "https://api.kraken.com/0/public/AssetPairs?pair={}",
        ticker
    );

    let client = match http_client {
        Some(c) => c,
        None => &reqwest::Client::new()
    };

    let response = client 
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<AssetPairsResponse>()
        .await?;

    let pair_info: AssetPairInfo = response.result
        .into_values()
        .next()
        .expect("Could not parse response");

    Ok(pair_info)
}


pub async fn write_data_to_db_table(
    ticker: &str,
    tick_data: TickDataResponse, 
    db_pool: Pool 
) -> Result<(), DbError> {
 
    // Insert tick data first
    let mut data_insert_query: String = format!(
        r#"INSERT INTO `asset_kraken_{}` (
            id, 
            price, 
            volume, 
            time, 
            buy_sell, 
            market_limit,
            misc
        ) VALUES "#, 
        ticker
    );
  
    let trade_fetch_response = match tick_data.result {
        Some(d) => d,
        None => return Err(DbError::ParseError)
    };

    let tick_data = match trade_fetch_response
        .trades
        .into_values()
        .next()
        .ok_or(DbError::ParseError)

    {
        Ok(d) => d,
        Err(_) => return Err(DbError::ParseError)
    };
 
    let max_index = tick_data.len() - 1;
    for (index, tick) in tick_data.iter().enumerate() {
        
        data_insert_query.push_str(&tick.to_db_row());
        
        if index < max_index {
            data_insert_query.push_str(",\n");
        };
    };
    
    data_insert_query.push_str(";");
   
    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };

    if let Err(_) = conn.query_drop(data_insert_query).await {
        return Err(DbError::QueryFailed(
            "Failed to insert tick data into database".to_string()
        )); 
    };

    // Update _last_tick_history
    let last_tick_timestamp = trade_fetch_response.last;
    let last_tick_id = match tick_data.iter().last() {
        Some(t) => t.tick_id + 1,
        None => return Err(DbError::ParseError) 
    };

    let last_tick_query: String = format!(r#"
        UPDATE _last_tick_history
        SET next_tick_id = ?, time = ?
        WHERE asset = ?;
    "#); 
    let last_tick_params = (last_tick_id, last_tick_timestamp, ticker);

    if let Err(_) = conn.exec_drop(last_tick_query, last_tick_params).await {
        return Err(DbError::QueryFailed(
            "Failed to fetch last tick".to_string()
        )); 
    };

    Ok(())
}

