use std::fmt;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use database_ops::*;
use timestamp_tools::*;


#[derive(Debug)]
pub enum BarBuildError {
    TickFetch,
    BuildFailed(String),
    DateConversion,
    Period(TimePeriodError),
    TickIdCalculation(String),
    Db(DbError),
}

impl std::fmt::Display for BarBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BarBuildError::TickFetch => write!(f, "BarBuildError::TickFetch"),
            BarBuildError::BuildFailed(e) => write!(
                f, "BarBuildError::BuildFailed: {}", e),
            BarBuildError::DateConversion => write!(
                f, "BarBuildError::DateConversion"),
            BarBuildError::Period(e) => write!(
                f, "BarBuildError::Period::{}", e),
            BarBuildError::TickIdCalculation(e) => write!(
                f, "BarBuildError::TickIdCalculation: {}", e),
            BarBuildError::Db(e) => write!(
                f, "BarBuildError::Db::{}", e),
        }
    }
}

impl From<TimePeriodError> for BarBuildError {
    fn from(err: TimePeriodError) -> Self {
        BarBuildError::Period(err)
    }
}

#[derive(Debug)]
pub enum BarType {
    Candle
}

// ------------------------------ BAR TYPES -------------------------------- //
#[derive(Debug)]
pub struct Bar {
    open: f64, 
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    open_date: DateTime<Utc>,
    close_date: DateTime<Utc>,
    tick_data: Vec<(u64, u64, f64, f64)>
}

impl Bar {
    
    fn new(
        tick_data: Vec<(u64, u64, f64, f64)>,
        open_date: DateTime<Utc>,
        close_date: DateTime<Utc>
    ) -> Self {
      
        fn min_max_vol(data: &[(u64, u64, f64, f64)]) -> (f64, f64, f64) {
            
            let mut min: f64 = 0.0; 
            let mut max: f64 = 0.0; 
            let mut volume: f64 = 0.0; 
            
            for tick in data {
                
                if min == 0.0 { min = tick.2 } 
                else if tick.2 < min {
                    min = tick.2; 
                };
                
                if tick.2 > max { max = tick.2 };
                
                volume += tick.3;
            
            }
            (min, max, volume)
        }

        let open = tick_data[0].2;
        let close = tick_data[tick_data.len() - 1].2;
        let (low, high, volume) = min_max_vol(&tick_data);

        Bar { 
            open, 
            high, 
            low, 
            close, 
            volume, 
            open_date, 
            close_date, 
            tick_data 
        }
    }
}

impl fmt::Display for Bar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, 
            "[{}, {}, {}, {}, {}, {}]", 
            self.open_date,
            self.open, 
            self.high, 
            self.low, 
            self.close, 
            self.volume
        )
    }
} 


pub struct BarInfo {
    exchange: String,
    ticker: String,
    period: String,
    time_based: bool
}

impl BarInfo {
    pub fn new(exchange: String, ticker: String, period: String) 
        -> Result<Self, BarBuildError> 
    {
        let (period_key, _) = get_period_portions_from_string(&period)
            .map_err(|_| 
                BarBuildError::Period(TimePeriodError::InvalidPeriod)
            )?;

        let time_based = period_is_time_based(period_key)
            .map_err(|e| BarBuildError::Period(e))?;

        Ok(BarInfo { exchange, ticker, period, time_based })
    }
}


pub struct BarSeries {
    pub tick_data: Vec<(u64, u64, f64, f64)>,
    pub bars: Vec<Bar>,
    pub info: BarInfo
}

impl BarSeries {
    
    pub async fn new (
        exchange: String,
        ticker: String,
        period: String,
        bar_type: BarType,
        db_pool: PgPool 
    ) -> Result<Self, BarBuildError> {
    
        let info: BarInfo = BarInfo::new(exchange, ticker, period)?; 

        let num_ticks: Option<u64> = Some(1_000_000);

        let tick_data: Vec<(u64, u64, f64, f64)> = match fetch_rows(
            &info.exchange, 
            &info.ticker, 
            num_ticks,
            db_pool 
        ).await {
            Ok(d) => d,
            Err(_) => {
                return Err(BarBuildError::TickFetch); 
            }
        };

        if info.period.len() < 2 {
            return Err(BarBuildError::Period(TimePeriodError::InvalidPeriod))
        };
           
        let mut bars: Vec<Bar> = Vec::new();
         
        let period_keys = match get_period_portions_from_string(&info.period) {
            Ok(d) => d,
            Err(e) => return Err(BarBuildError::Period(e))
        };

        let (period_char, period_n) = period_keys;

        // START PARSING DATA
        let (tick_indices, open_dates, close_dates) = 
            get_tick_indices_and_dates(&tick_data, period_n, period_char)?;
        
        let mut index: usize = 0;
   
        while index + 1 < tick_indices.len() {
            
            let start_idx = tick_indices[index];
            let end_idx = tick_indices[index + 1];
            let open_date: DateTime<Utc> = open_dates[index];
            let close_date: DateTime<Utc> = close_dates[index];
            let tick_slice = tick_data[start_idx..end_idx].to_vec(); 
            let new_bar: Bar = Bar::new(tick_slice, open_date, close_date);
            bars.push(new_bar);
    
            index += 1;
            
        }
        
        let start_idx = tick_indices[index];
        let open_date: DateTime<Utc> = open_dates[index];
        let close_date: DateTime<Utc> = close_dates[index];
        let tick_slice = tick_data[start_idx..].to_vec(); 
        bars.push(Bar::new(tick_slice, open_date, close_date));
       
        match bar_type {
            BarType::Candle =>  Ok(BarSeries { tick_data, bars, info })
        }

    }

}

impl<'a> IntoIterator for &'a BarSeries {
    type Item = &'a Bar;
    type IntoIter = std::slice::Iter<'a, Bar>;

    fn into_iter(self) -> Self::IntoIter {
        self.bars.iter()
    }
}


// ---------------- HELPER FUNCTIONS --------------- //
pub async fn calculate_first_tick_id(
    exchange: &str,
    ticker: &str,
    period: &str,
    db_pool: PgPool,
    num_bars: u16
) -> Result<u64, BarBuildError> {

    let (symbol, n_periods) = get_period_portions_from_string(period)
        .map_err(|_| BarBuildError::Period(TimePeriodError::InvalidPeriod))?;

    let last_tick = fetch_first_or_last_row(
        exchange, ticker, db_pool.clone(), true
    )
        .await 
        .map_err(|_| BarBuildError::TickIdCalculation(
            "Failed to fetch initial tick value".to_string()
        ))?
        .into_iter()
        .next()
        .ok_or_else(|| BarBuildError::TickIdCalculation(
            "Failed to fetch initial tick value".to_string()
        ))?;
        
    if period_is_time_based(symbol).map_err(|e| BarBuildError::Period(e))? {
        
        let last_tick_timestamp: u64 = last_tick.1 / 1_000_000;

        let num_secs = calculate_seconds_in_period(n_periods, symbol) 
            .map_err(|_| BarBuildError::TickIdCalculation(
                "Failed to calculate seconds in period".to_string()
            ))?;

        let first_tick_time: u64 = candle_open_timestamp(
            last_tick_timestamp - (num_secs * (num_bars as u64)), num_secs
        ) * 1_000_000;
     
        let tick = fetch_first_tick_by_time_column(
            exchange, 
            ticker, 
            &first_tick_time,
            db_pool 
        ).await;

        if tick.len() > 0 {
            Ok(tick[0].0)
        }
        else {
            Err(BarBuildError::TickIdCalculation(
                "Failed to fetch initial tick value".to_string()
            ))
        }

    } 
    else {

        let num_ticks: u64 = n_periods * (num_bars as u64);      
       
        let tick_id = last_tick.0 - num_ticks;
        
        Ok(tick_id)

    }

}



