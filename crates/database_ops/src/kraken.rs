use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;


#[derive(Deserialize, Debug)]
pub struct KrakenResponse {
    pub error: Vec<String>,
    pub result: Option<KrakenResult>,
}

#[derive(Deserialize, Debug)]
struct KrakenResult {
    #[serde(flatten)]
    trades: HashMap<String, Vec<Trade>>,
    last: String,  // The 'since' value for pagination 
}

#[derive(Deserialize, Debug)]
struct Trade {
    price: String,         // Price as string
    volume: String,        // Volume as string
    time: f64,             // Unix timestamp (seconds with decimal)
    buy_sell: String,      // "b" or "s"
    market_limit: String,  // "m" or "l"
    miscellaneous: String, // Usually an empty string
    tick_id: u64,
}


pub enum RequestError {
    Http(reqwest::Error),
    BadStatus(reqwest::StatusCode),
    Deserialize(serde_json::Error),
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


pub async fn request_tick_data_from_kraken(
    ticker: &str, since_unix_timestamp: String 
) -> Result<KrakenResponse, RequestError> {
    
    let mut url = format!(
        "https://api.kraken.com/0/public/Trades?pair={}&since={}", 
        ticker,
        since_unix_timestamp
    );
   
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(RequestError::BadStatus(response.status()));
    }

    let raw_text = response.text().await?;

    let kraken_resp: KrakenResponse = serde_json::from_str(&raw_text)
        .map_err(|e| {
            println!("\x1b[1;31mDeserialization error:\n\x1b[0m{}", e);
            e
        })?;

    Ok(kraken_resp)

}


