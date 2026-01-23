use std::{
    collections::HashMap, 
    time::{SystemTime, UNIX_EPOCH},
    io::{self, Write}
};

use reqwest;
use serde::Deserialize;
use mysql_async::{prelude::Queryable, Pool, Conn};
use tokio::time::{sleep, Duration};

use timestamp_tools::{get_current_unix_timestamp};
use crate::connection::{DbError, FetchError, RequestError, get_table_name};
use super::fetch_tables;
pub use crate::connection;


// Tick data structs
#[derive(Deserialize, Debug)]
pub struct TickDataResponse {
    error: Vec<String>,
    result: Option<TickDataResult>,
}

impl TickDataResponse {
    
    fn len(&self) -> Option<usize> {
        if let Some(d) = &self.result {
            if let Some(v) = d.trades.values().next() {
                return Some(v.len())
            }
        };
        None
    }

    fn last_tick_id(&self) -> Option<u64> {
        if let Some(data) = &self.result {
            if let Some(vector) = data.trades.values().next() {
                if let Some(v) = vector.last() {
                    return Some(v.tick_id)
                }
            }
        }
        None
    }
    
    fn next_fetch_timestamp(&self) -> Option<String> {
        if let Some(d) = &self.result {
            Some(d.last.clone())
        }
        else {
            None
        }
    }

    fn timestamp_of_last_tick(&self) -> Option<f64> {
        if let Some(data) = &self.result {
            if let Some(vector) = data.trades.values().next() {
                if let Some(v) = vector.last() {
                    return Some(v.time.clone())
                }
            }
        }
        None
    }

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


pub async fn add_new_db_table(
    ticker: &str,
    start_date_unix_timestamp_offset: u64,
    http_client: Option<&reqwest::Client>,
    db_pool: Pool 
) -> Result<(), DbError> {

    let table_name: String = get_table_name("kraken", ticker);
           
    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };

    let existing_tables: Vec<String> = match fetch_tables(
        db_pool.clone()
    ).await {
        Ok(d) => d,
        Err(_) => return Err(
            connection:: DbError::QueryFailed(
                "Failed to fetch table names".to_string() 
            )
        )
    };

    if existing_tables.contains(&table_name) {
        return Err(
            connection::DbError::TableCreationFailed(
                format!("{} table already exists", ticker)
            )
        )
    };
    
    const INIT_TIME_OFFSET: u64 = 60 * 60 * 24 * 14;  // 2 weeks of seconds
    
    let current_ts = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(t) => t.as_secs(),
        Err(_) => return Err(
            connection::DbError::Fetch(
                FetchError::SystemError(
                    "Couldn't retrieve system time".to_string()
                )
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
                connection::DbError::Fetch(
                    FetchError::Api(
                        RequestError::RequestFailed(
                            "Could not fetch trade data".to_string()
                        )
                    )
                )
            })?;

            let trades_vec = result 
                .trades 
                .values() 
                .next() 
                .ok_or_else(|| {
                    connection::DbError::Fetch(
                        FetchError::SystemError(
                            "No trades detected in response".to_string()
                        )
                    )
                })?;

            trades_vec.last().cloned().ok_or_else(|| {
                connection::DbError::Fetch(
                    FetchError::SystemError(
                        "Trades list was empty".to_string()
                    )
                )
            })?

        },
        Err(_) => return Err(
            connection::DbError::Fetch(
                FetchError::Api(
                    RequestError::RequestFailed(
                        "Could not fetch trade data".to_string()
                    )
                )
            )
        )
    };

    let price_string = initial_trade.price.to_string();
    let left_digits: usize = match price_string.split_once(".") {
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
            connection::DbError::Fetch(
                FetchError::Api(
                    RequestError::Http(e)
                )
            )
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
        return Err(DbError::TableCreationFailed(
            format!("Failed to create asset_kraken_{} table", ticker) 
        )); 
    };

    let initial_time_stamp_query: String = format!(r#"
        INSERT INTO _last_tick_history (asset, next_tick_id, time) 
        VALUES ('{}', 0, 0);"#, ticker);

    if let Err(_) = conn.query_drop(initial_time_stamp_query).await {
        return Err(
            DbError::QueryFailed(
                format!(
                    "Failed to fetch _last_tick_history for {}",
                    ticker
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
        Err(e) => return Err(DbError::Fetch(FetchError::Api(e)))
    };

    write_data_to_db_table(ticker, &initial_data, db_pool.clone(), None).await;
    
    Ok(())

}


pub async fn download_new_data_to_db_table(
    ticker: &str,
    db_pool: Pool,
    initial_unix_timestamp_offset: u64,
    http_client: Option<&reqwest::Client>
) -> Result<(), DbError> {
 
    let current_time: u64 = get_current_unix_timestamp();
    let start_timestamp: u64 = current_time - initial_unix_timestamp_offset;

    let client = match http_client {
        Some(c) => c,
        None => &reqwest::Client::new()
    }; 

    let mut conn: Conn = match db_pool.get_conn().await {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };

    let existing_tables: Vec<String> = match fetch_tables(
        db_pool.clone() 
    ).await {
        Ok(d) => d,
        Err(_) => return Err(
            DbError::QueryFailed(
                "Failed to fetch table names".to_string()
            )
        )
    };
    
    let table_name = get_table_name("kraken", ticker);
    if !existing_tables.contains(&table_name) {
        add_new_db_table(
            &ticker, 
            start_timestamp, 
            Some(&client),
            db_pool.clone()
        ).await?;
    };

    // Get the last recorded timestamp from _last_tick_history
    let last_tick_query = format!(
        r#"
        SELECT next_tick_id, time 
        FROM _last_tick_history
        WHERE asset = '{}' 
        "#,
        ticker
    ); 

    let valid_row: Vec<(u64, String)> = match conn.exec(
        last_tick_query, ()
    ).await {
        Ok(r) => r,
        Err(_) => return Err(DbError::QueryFailed(
            "Couldn't fetch last tick time from _last_tick_history".to_string()
        )) 
    };

    let (mut next_tick_id, mut next_timestamp) = match valid_row.len() > 0 {
        true => (valid_row[0].0, valid_row[0].1.clone()),
        false => return Err(DbError::QueryFailed(
            "Couldn't fetch last tick time from _last_tick_history".to_string()
        ))
    }; 

    let last_timestamp_in_db_vec: Vec<u64> = match conn.exec(
        format!(
            r#"
            SELECT time FROM {} ORDER BY id DESC LIMIT 1;
            "#, 
            &table_name
        ),
        ()
    ).await {
        Ok(d) => d,
        Err(_) => {
            return Err(DbError::QueryFailed(
                "Couldn't fetch last timestamp in table".to_string()
            ))
        }
    };

    let last_timestamp_in_db: u64 = match last_timestamp_in_db_vec.len() {
        0 => return Err(DbError::QueryFailed(
            "No timestamp detected in last_timestamp_in_db_vec".to_string()
        )),
        _ => last_timestamp_in_db_vec[0] / 1_000_000
    };

    let total_expected_seconds = current_time - last_timestamp_in_db;
    let mut num_seconds_left = total_expected_seconds.clone();
    let mut percent_complete: u8 = 0;

    fn get_percent_complete(curr: u64, target: u64) -> u8 {
        100 - ((curr * 100) / target) as u8
    }

    println!("\x1b[1;33mDownloading data from kraken\x1b[0m");
    loop {
        
        let new_data: TickDataResponse = match request_tick_data_from_kraken(
            ticker, 
            next_timestamp, 
            Some(client)
        ).await {
            Ok(d) => d,
            Err(e) => {
                return Err(DbError::Fetch(FetchError::Api(e)))
            }
        };

        let num_ticks: usize = match new_data.len() {
            Some(v) => v,
            None => { 
                return Err(DbError::Fetch(FetchError::SystemError(
                    "Failed to calculate length of trades".to_string()
                )))
            }
        };

        if let Err(e) = write_data_to_db_table(
            ticker, 
            &new_data, 
            db_pool.clone(), 
            Some(next_tick_id)
        ).await {
            return Err(e) 
        };

        next_tick_id = match &new_data.last_tick_id() {
            Some(v) => *v + 1,  // Expected first ID of next fetch
            None => return Err(DbError::Fetch(FetchError::SystemError(
                "Failed to fetch last tick ID from TickDataResponse"
                    .to_string()
            )))
        };

        next_timestamp = match &new_data.next_fetch_timestamp() {
            Some(v) => v.to_string(),
            None => return Err(DbError::Fetch(FetchError::SystemError(
                "Failed to fetch next fetch time from TickDataResponse"
                    .to_string()
            )))
        };

        if num_ticks < 1000 {
            println!(
                "\x1b[1;32mLess than 1000 ticks in set. Breaking loop\x1b[0m"
            ); 
            break
        };
    
        // Wait 1 sec to prevent rate limits
        sleep(Duration::from_secs(1)).await;
        
        let last_tick_time: u64 = match &new_data.timestamp_of_last_tick() {
            Some(v) => *v as u64,
            None => return Err(DbError::Fetch(FetchError::SystemError(
                "Failed to fetch last timestamp from TickDataResponse"
                    .to_string()
            )))
        };
        
        num_seconds_left = current_time - last_tick_time;
        percent_complete = get_percent_complete(
            num_seconds_left, total_expected_seconds
        );
       
        print!(
            "\r\x1b[0mDownload Progress: \x1b[1;32m{}%\x1b[0m", 
            &percent_complete
        );
        io::stdout().flush().ok();

    };

    println!("");

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
    tick_data: &TickDataResponse, 
    db_pool: Pool,
    next_tick_id: Option<u64>
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
  
    let trade_fetch_response = match &tick_data.result {
        Some(d) => d,
        None => return Err(DbError::ParseError)
    };

    let tick_data = match trade_fetch_response
        .trades
        .values()
        .next()
        .ok_or(DbError::ParseError)

    {
        Ok(d) => d,
        Err(_) => return Err(DbError::ParseError)
    };
 
    let max_index = tick_data.len() - 1;
    for (index, trade) in tick_data.iter().enumerate() {
       
        if let Some(next_id) = next_tick_id {
            if trade.tick_id < next_id {
                continue 
            };
        };

        data_insert_query.push_str(&trade.to_db_row());
        
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

    let last_tick_timestamp = trade_fetch_response.last.clone();
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

