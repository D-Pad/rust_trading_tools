use database_ops::*;
use bars::*;
pub mod config;


pub async fn fetch_data_and_build_bars(
    exchange: &str,
    ticker: &str,
    period: &str,
    number_of_ticks: Option<u64>
) -> BarSeries {
 
    let num_ticks = match number_of_ticks {
        Some(t) => Some(t),
        None => Some(1_000_000)
    };

    let tick_data: Vec<(u64, u64, f64, f64)> = 
        match fetch_rows(exchange, ticker, num_ticks).await {
            Ok(d) => d,
            Err(_) => {
                println!("Failed to fetch ticks");
                return BarSeries::empty(); 
            }
        };

    let bar_type = bars::BarType::Candle;
    
    match BarSeries::new(tick_data, period, bar_type) {
        Ok(b) => b,
        Err(_) => BarSeries::empty()
    } 

}


pub async fn dev_test() {
    let row = fetch_last_row("kraken", "SOLUSD", None).await;
    println!("{:?}", row);
}



