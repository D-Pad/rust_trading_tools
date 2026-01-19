use std::{
    collections::HashMap, 
    time::{SystemTime, UNIX_EPOCH}
};

use reqwest;
use serde::Deserialize;
use mysql_async::{prelude::Queryable, Conn};
use tokio::time::{sleep, Duration};

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
    db_connection: Option<Conn>
) -> Result<(), connection::FetchError> {
   
    let mut conn: Conn = match db_connection {
        Some(c) => c,
        None => {
            
            let db: connection::Db = connection::get_db_connection(
                None, "kraken"
            ).await?;
            
            let mut connection: Conn = match db.conn().await {
                Ok(c) => c,
                Err(_) => return Err(FetchError::Db(DbError::ConnectionFailed))
            };
            
            let existing_tables: Vec<String> = match connection.exec(
                "SHOW TABLES;", ()
            ).await {
                Ok(d) => d,
                Err(_) => return Err(
                    connection::FetchError::Db(
                        DbError::QueryFailed
                    )
                )
            };

            if existing_tables.contains(&ticker.to_string()) {
                println!("\x1b[1;31m{ticker} table already exists\x1b[0m");
                return Ok(())
            };

            connection
        }
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
    
    let query: String = format!(r#"
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
        ticker,
        price_digits_for_db_table,
        tick_info.pair_decimals,
        price_digits_for_db_table,
        tick_info.lot_decimals
    ); 
    
    if let Err(_) = conn.query_drop(query).await {
        return Err(FetchError::Db(DbError::QueryFailed)); 
    };

    // Initial table data population
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

    write_data_to_db_table(ticker, initial_data, Some(conn)).await;

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
    db_connection: Option<Conn>
) -> Result<(), DbError> {
    
    let mut query: String = format!(
        r#"INSERT INTO `{}` (
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
    
    let tick_value_result = match tick_data.result {
        Some(d) => d.trades.into_values().next().ok_or(DbError::ParseError),
        None => return Err(DbError::QueryFailed)
    };

    let tick_data = match tick_value_result {
        Ok(d) => d,
        Err(_) => return Err(DbError::ParseError) 
    };
   
    let max_index = tick_data.len() - 1;
    for (index, tick) in tick_data.iter().enumerate() {
        query.push_str(&tick.to_db_row());
        if index < max_index {
            query.push_str(",\n");
        };
    };
    
    query.push_str(";");
   
    let mut conn: Conn = match db_connection {
        Some(c) => c,
        None => {
            let db: connection::Db = connection::get_db_connection(
                None, "kraken"
            ).await?;
            
            let connection: Conn = match db.conn().await {
                Ok(c) => c,
                Err(_) => return Err(DbError::ConnectionFailed)
            };
            connection
        } 
    };

    if let Err(_) = conn.query_drop(query).await {
        println!("QUERY FAILED");
        return Err(DbError::QueryFailed); 
    };

    Ok(())
}

