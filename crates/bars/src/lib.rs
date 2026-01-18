use std::fmt;
use chrono::{DateTime, Utc};
use timestamp_tools::*;


pub enum BarBuildError {
    TickFetch,
    BuildFailed,
    DateConversion,
    Period(TimePeriodError),
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

pub struct BarSeries {
    pub tick_data: Vec<(u64, u64, f64, f64)>,
    pub bars: Vec<Bar>
}

impl BarSeries {
    
    pub fn new (
        tick_data: Vec<(u64, u64, f64, f64)>,
        period: &str,
        bar_type: BarType
    ) -> Result<Self, BarBuildError> {
    
        if period.len() < 2 {
            return Err(BarBuildError::Period(TimePeriodError::InvalidPeriod))
        };
           
        let mut bars: Vec<Bar> = Vec::new();
         
        let period_keys = match get_period_portions_from_string(period) {
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
            BarType::Candle =>  Ok(BarSeries { tick_data, bars })
        }

    }

    pub fn empty() -> Self {
        BarSeries {
            tick_data: Vec::new(),
            bars: Vec::new()
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


#[cfg(test)]
mod tests {
    
    use super::*;

    #[test]
    fn candle_test() {
        assert_eq!(4, 4);
    }
}
