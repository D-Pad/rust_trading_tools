use std::{
    collections::{HashMap, BTreeMap},
    time::{SystemTime, UNIX_EPOCH},
    cmp::{min, max}
};

use reqwest;
use serde::Deserialize;
use tokio::{time::{sleep, Duration}, sync::mpsc::UnboundedSender};
use sqlx::{PgPool, pool::{PoolConnection}};

use timestamp_tools::{get_current_unix_timestamp};
use connection::{
    DataDownloadStatus, 
    DbError, 
    FetchError, 
    RequestError, 
    get_table_name
};
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

    pub margin_call: Option<u32>,
    pub margin_stop: Option<u32>,

    pub ordermin: String,
    pub costmin: String,
    pub tick_size: String,

    pub status: String,

    pub long_position_limit: Option<u32>,
    pub short_position_limit: Option<u32>,
}


pub async fn add_new_db_table(
    ticker: &str,
    start_date_unix_timestamp_offset: u64,
    client: &reqwest::Client,
    db_pool: PgPool 
) -> Result<(), DbError> {

    let table_name: String = get_table_name("kraken", ticker);

    let existing_tables: Vec<String> = fetch_tables(db_pool.clone())
        .await
        .map_err(|_| 
            connection:: DbError::QueryFailed(
                "Failed to fetch table names".to_string() 
            )
        )?;

    if existing_tables.contains(&table_name) {
        return Err(
            connection::DbError::TableCreationFailed(
                format!("{} table already exists", ticker)
            )
        )
    };
    
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

    sleep(Duration::from_millis(500)).await;
    
    let tick_info = request_asset_info_from_kraken(&ticker, client)
        .await 
        .map_err(|e|  
            connection::DbError::Fetch(
                FetchError::Api(
                    RequestError::Http(e)
                )
            )
        )?;
    
    let create_table: String = format!(r#"
        CREATE TABLE IF NOT EXISTS {} (
            id BIGINT PRIMARY KEY,
            price DECIMAL({},{}) NOT NULL, 
            volume DECIMAL({},{}) NOT NULL, 
            time BIGINT NOT NULL, 
            buy_sell CHAR(1) NOT NULL, 
            market_limit CHAR(1) NOT NULL, 
            misc VARCHAR(16)
        );
        "#,
        table_name,
        max(24, tick_info.pair_decimals * 2),
        tick_info.pair_decimals,
        max(24, tick_info.lot_decimals * 2),
        tick_info.lot_decimals
    ); 

    let mut conn: PoolConnection<sqlx::Postgres> = db_pool
        .acquire() 
        .await 
        .map_err(|_| DbError::ConnectionFailed)?;

    if let Err(_) = sqlx::query(&create_table).execute(&mut *conn).await {
        return Err(DbError::TableCreationFailed(
            format!("Failed to create asset_kraken_{} table", ticker) 
        )); 
    };

    let initial_time_stamp_query: String = format!(r#"
        INSERT INTO _last_tick_history (asset, next_tick_id, time) 
        VALUES ('{}', 0, 0);"#, ticker);

    if let Err(_) = sqlx::query(&initial_time_stamp_query)
        .execute(&mut *conn)
        .await 
    {
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
   
    let initial_fetch_time = current_ts - start_date_unix_timestamp_offset;  

    let initial_data: TickDataResponse = request_tick_data_from_kraken(
        ticker, 
        initial_fetch_time.to_string(),
        client
    ).await.map_err(|e| DbError::Fetch(FetchError::Api(e)))?;

    write_data_to_db_table(ticker, &initial_data, db_pool.clone(), None)
        .await?;
    
    Ok(())

}


pub async fn download_new_data_to_db_table(
    ticker: &str,
    db_pool: PgPool,
    initial_unix_timestamp_offset: u64,
    client: &reqwest::Client,
    progress_tx: UnboundedSender<DataDownloadStatus>,
) -> Result<(), DbError> {

    const EXCHANGE: &'static str = "Kraken";
    let ex_name: String = EXCHANGE.to_string();

    let current_time: u64 = get_current_unix_timestamp();

    let mut conn = match db_pool
        .acquire()
        .await 
    {
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
            initial_unix_timestamp_offset, 
            &client,
            db_pool.clone()
        ).await?;
    };

    // Get the last recorded timestamp from _last_tick_history
    let ltq = format!(
        r#"
        SELECT next_tick_id, time 
        FROM _last_tick_history
        WHERE asset = '{}' 
        "#,
        ticker
    ); 

    type Vrow = Vec<(u64, String)>;
    let valid_row: Vrow = match sqlx::query_as::<_, (i64, String)>(&ltq)
        .fetch_all(&mut *conn)
        .await 
    {
        Ok(r) => r.into_iter().map(|(i, t)| (i as u64, t)).collect(),
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

    let tq = format!(
        "SELECT time FROM {} ORDER BY id DESC LIMIT 1;", 
        &table_name
    );
    
    let last_timestamp_in_db_vec: Vec<u64> = match sqlx::query_scalar(&tq)
        .fetch_all(&mut *conn)
        .await 
    {
        Ok(d) => d.into_iter().map(|v: i64| v as u64).collect(),
        Err(e) => {
            return Err(DbError::QueryFailed(format!(
                "Couldn't fetch last timestamp in table: {}", e
            )))
        }
    };

    let last_timestamp_in_db: u64 = match last_timestamp_in_db_vec.len() {
        0 => return Err(DbError::QueryFailed(
            "No timestamp detected in last_timestamp_in_db_vec".to_string()
        )),
        _ => last_timestamp_in_db_vec[0] / 1_000_000
    };

    let total_expected_seconds = current_time - last_timestamp_in_db;
    let mut num_seconds_left: u64;
    let mut percent_complete: u8;

    fn get_percent_complete(curr: u64, target: u64) -> u8 {
        100 - ((curr * 100) / target) as u8
    }

    fn send_failure_message(
        progress_tx: UnboundedSender<DataDownloadStatus>,
        sym: &str, 
    ) {
        let _ = progress_tx.send(DataDownloadStatus::Error { 
            exchange: "Kraken".to_string(), 
            ticker: sym.to_string(), 
        });
    }

    loop {
        
        let new_data: TickDataResponse = match request_tick_data_from_kraken(
            ticker, 
            next_timestamp, 
            client
        ).await {
            Ok(d) => d,
            Err(e) => {
                return Err(DbError::Fetch(FetchError::Api(e)))
            }
        };

        let num_ticks: usize = match new_data.len() {
            Some(v) => v,
            None => { 
                let msg = "Failed to calculate length of trades".to_string();
                send_failure_message(progress_tx.clone(), ticker);
                return Err(DbError::Fetch(FetchError::SystemError(msg)))
            }
        };

        if new_data.error.len() == 0 {

            if let Err(e) = write_data_to_db_table(
                ticker, 
                &new_data, 
                db_pool.clone(), 
                Some(next_tick_id)
            ).await {
                send_failure_message(progress_tx.clone(), ticker);
                return Err(e) 
            };

        }

        else {

            return Err(
                DbError::Fetch(
                    FetchError::Api(
                        RequestError::ErrorResponse(
                            new_data.error[0].clone() 
                        )
                    )
                )
            ) 

        };

        next_tick_id = match &new_data.last_tick_id() {
            Some(v) => *v + 1,  // Expected first ID of next fetch
            None => {
                let msg = "Failed to fetch last tick ID from TickDataResponse"
                    .to_string(); 
                send_failure_message(progress_tx.clone(), ticker); 
                return Err(DbError::Fetch(FetchError::SystemError(msg)))
            }
        };

        next_timestamp = match &new_data.next_fetch_timestamp() {
            Some(v) => v.to_string(),
            None => {
                let msg ="Failed to fetch next fetch time from TickDataResponse"
                    .to_string();
                send_failure_message(progress_tx.clone(), ticker);
                return Err(DbError::Fetch(FetchError::SystemError(msg)))
            }
        };
     
        let last_tick_time: u64 = match &new_data.timestamp_of_last_tick() {
            Some(v) => *v as u64,
            None => {
                let msg = "Failed to fetch last timestamp from TickDataResponse"
                    .to_string();
                return Err(DbError::Fetch(FetchError::SystemError(msg)))
            }
        };
        
        num_seconds_left = current_time - min(last_tick_time, current_time);
        percent_complete = get_percent_complete(
            num_seconds_left, total_expected_seconds
        );

        let _ = progress_tx.send(DataDownloadStatus::Progress { 
            exchange: ex_name.clone(), 
            ticker: ticker.to_string(), 
            percent: percent_complete 
        });

        if num_ticks < 1000 {

            let _ = progress_tx.send(DataDownloadStatus::Progress { 
                exchange: ex_name.clone(), 
                ticker: ticker.to_string(), 
                percent: 100 
            });

            let _ = progress_tx.send(DataDownloadStatus::Finished { 
                exchange: ex_name.clone(), 
                ticker: ticker.to_string(), 
            });
            
            break
        };
  
        // Wait 1 sec to prevent rate limits
        sleep(Duration::from_secs(1)).await;

    };

    Ok(())

}


pub async fn request_tick_data_from_kraken(
    ticker: &str, 
    since_unix_timestamp: String, 
    client: &reqwest::Client 
) -> Result<TickDataResponse, RequestError> {
    
    let url = format!(
        "https://api.kraken.com/0/public/Trades?pair={}&since={}", 
        ticker,
        since_unix_timestamp
    );
  
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


pub async fn request_all_asset_info_from_kraken(
    client: &reqwest::Client,
) -> Result<BTreeMap<String, AssetPairInfo>, reqwest::Error> {
    let url = "https://api.kraken.com/0/public/AssetPairs";

    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<AssetPairsResponse>()
        .await?;

    // Convert HashMap → BTreeMap for deterministic ordering
    let pairs: BTreeMap<String, AssetPairInfo> =
        response.result.into_iter().collect();

    Ok(pairs)
}


pub async fn request_asset_info_from_kraken(
    ticker: &str,
    client: &reqwest::Client 
) 
  -> Result<AssetPairInfo, reqwest::Error> {
    
    let url = format!(
        "https://api.kraken.com/0/public/AssetPairs?pair={}",
        ticker
    );

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
    db_pool: PgPool,
    next_tick_id: Option<u64>
) -> Result<(), DbError> {

    // Insert tick data first
    let mut data_insert_query: String = format!(
        r#"INSERT INTO asset_kraken_{} (
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

    if tick_data.len() == 0 {
        return Err(DbError::Fetch(FetchError::Api(RequestError::NoData)))
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

    if let Err(e) = sqlx::query(&data_insert_query)
        .execute(&db_pool)
        .await 
    {
        return Err(DbError::QueryFailed(
            format!(
                "Failed to insert tick data into database: {}: {}", 
                e,
                &data_insert_query
            )
        )); 
    };

    let last_tick_timestamp = trade_fetch_response.last.clone();
    let last_tick_id = match tick_data.iter().last() {
        Some(t) => t.tick_id + 1,
        None => return Err(DbError::ParseError) 
    };

    let last_tick_query: String = String::from(r#"
        UPDATE _last_tick_history
        SET next_tick_id = $1, time = $2
        WHERE asset = $3;
        "#
    );

    if let Err(_) = sqlx::query(&last_tick_query)
        .bind(last_tick_id as i64)
        .bind(last_tick_timestamp)
        .bind(ticker)
        .execute(&db_pool) 
        .await 
    {
        return Err(DbError::QueryFailed(
            "Failed to fetch update _last_tick_history".to_string()
        )); 
    };

    Ok(())

}

