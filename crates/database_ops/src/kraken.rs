use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;


#[derive(Deserialize, Debug)]
pub struct KrakenResponse {
    error: Vec<String>,
    result: Option<KrakenResult>,
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


pub async fn add_new_data_to_db_table(
    ticker: &str
) -> Result<KrakenResponse, String> {
    let resp = request_tick_data_from_kraken(ticker, 1767152112);
    match resp.await {
        Ok(s) => Ok(s),
        Err(_) => Err("Failed to fetch data".to_string())
    } 
}


pub async fn request_tick_data_from_kraken(
    ticker: &str, since_unix_timestamp: u64 
) -> Result<KrakenResponse, Box<dyn Error>> {
    
    let mut url = format!(
        "https://api.kraken.com/0/public/Trades?pair={}&since={}", 
        ticker,
        since_unix_timestamp
    );
   
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch data from Kraken").into());
    }

    let raw_text = response.text().await?;

    let kraken_resp: KrakenResponse = serde_json::from_str(&raw_text)
        .map_err(|e| {
            println!("\x1b[1;31mDeserialization error:\n\x1b[0m{}", e);
            e
        })?;

    Ok(kraken_resp)

}





