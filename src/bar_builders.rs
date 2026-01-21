use bars;
use app_state::{AppState};


pub async fn fetch_data_and_build_bars(
    exchange: &str,
    ticker: &str,
    period: &str,
    number_of_ticks: Option<u64>,
    app_state: &AppState 
) -> bars::BarSeries {
 
    let num_ticks = match number_of_ticks {
        Some(t) => Some(t),
        None => Some(1_000_000)
    };

    let tick_data: Vec<(u64, u64, f64, f64)> = match database_ops::fetch_rows(
        exchange, 
        ticker, 
        num_ticks,
        app_state.database.get_pool()
    ).await {
        Ok(d) => d,
        Err(_) => {
            println!("Failed to fetch ticks");
            return bars::BarSeries::empty(); 
        }
    };

    let bar_type = bars::BarType::Candle;
    
    match bars::BarSeries::new(tick_data, period, bar_type) {
        Ok(b) => b,
        Err(_) => bars::BarSeries::empty()
    } 

}


