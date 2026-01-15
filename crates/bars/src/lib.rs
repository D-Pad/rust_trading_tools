use std::fmt;
use chrono::{DateTime, Datelike, TimeZone, Utc, Duration};
use num_traits::{PrimInt, Unsigned};

pub enum BarBuildError {
    TickFetch,
    InvalidPeriod,
    NotEnoughData,
    BuildFailed,
    DateConversion
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
            return Err(BarBuildError::InvalidPeriod)
        };
    
        let period_key = match period.chars().last() {
            Some(c) => c, 
            None => { return Err(BarBuildError::InvalidPeriod) } 
        };
       
        if !VALID_PERIODS.contains(&period_key) {
            return Err(BarBuildError::InvalidPeriod)
        };
    
        let period_n: u64 = match period[0..period.len() - 1].parse::<u64>() {
            Ok(v) => v,
            Err(_) => return Err(BarBuildError::InvalidPeriod)
        };
        
        let mut bars: Vec<Bar> = Vec::new();
           
        // START PARSING DATA
        let (tick_indices, open_dates, close_dates) = 
            get_tick_indices_and_open_dates(&tick_data, period_n, period_key)?;
        
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
        
        Ok(
            BarSeries {
                tick_data,
                bars        
            }
        )
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

// -------------------------- DATE CONVERSION ------------------------------ //
fn micros_u64_to_datetime(
    microseconds: u64
) -> Result<DateTime<Utc>, BarBuildError> {
    
    let secs = (microseconds / 1_000_000) as i64;
    let nsecs = (microseconds % 1_000_000) as u32;
    
    match Utc.timestamp_opt(secs, nsecs) {
        chrono::LocalResult::Single(dt) => Ok(dt),

        chrono::LocalResult::Ambiguous(_, _) => {
            Err(BarBuildError::DateConversion)
        }

        chrono::LocalResult::None => {
            Err(BarBuildError::DateConversion)
        }
    }
}

fn unix_ts_i64_to_datetime(
    seconds: i64
) -> Result<DateTime<Utc>, BarBuildError> {
    
    match Utc.timestamp_opt(seconds, 0) {
        chrono::LocalResult::Single(dt) => Ok(dt),

        chrono::LocalResult::Ambiguous(_, _) => {
            Err(BarBuildError::DateConversion)
        }

        chrono::LocalResult::None => {
            Err(BarBuildError::DateConversion)
        }
    }
}


// --------------------------- CANDLE PERIOD ------------------------------- //
const VALID_PERIODS: &[char; 7] = &['s', 'm', 'h', 'd', 'w', 'M', 't'];

fn calculate_seconds_in_period(
    periods: u64, 
    symbol: char
) -> Result<u64, BarBuildError> {
    
    let num_seconds = match symbol {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        'd' => 86400,
        _ => return Err(BarBuildError::InvalidPeriod)
    };
    
    Ok(num_seconds * periods) 
}

fn get_tick_indices_and_open_dates<'a> (
    tick_data: &'a [(u64, u64, f64, f64)],
    period_number_portion: u64,
    period_symbol_portion: char
) -> Result<(Vec<usize>, Vec<DateTime<Utc>>, Vec<DateTime<Utc>>), BarBuildError> {

    fn err_msg(msg: &'static str) {
        println!("\x1b[1;31m{}\x1b[0m", msg);
    }

    fn this_week_or_month(
        ts: u64, 
        sym: &char
    ) -> Result<DateTime<Utc>, BarBuildError> {
      
        fn this_week_start(dt: DateTime<Utc>) -> DateTime<Utc> {
            let weekday = dt.weekday().num_days_from_sunday() as i64;
            
            let next_sunday = dt
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::days(7 - weekday);
        
            Utc.from_utc_datetime(&next_sunday)
        }
        
        fn this_month_start(dt: DateTime<Utc>) -> 
            Result<DateTime<Utc>, BarBuildError> {
            
            let year = dt.year();
            let month = dt.month();

            let (next_year, next_month) = if month == 12 {
                (year + 1, 1)
            } else {
                (year, month + 1)
            };
        
            let date_result = Utc.with_ymd_and_hms(
                next_year, next_month, 1, 0, 0, 0
            );
            
            match date_result.single() {
                Some(dt) => Ok(dt),
                None => Err(BarBuildError::DateConversion)
            }
        }

        let is_week: bool = sym == &'w';
        let dt: DateTime<Utc> = micros_u64_to_datetime(ts)?;
        let cut_date: DateTime<Utc> = match is_week {
            true => { 
                this_week_start(dt)
            },
            false => { 
                this_month_start(dt)?
            }
        };    

        Ok(cut_date) 
    }

    fn next_week_or_month(
        this: DateTime<Utc>, 
        sym: &char
    ) -> Result<DateTime<Utc>, BarBuildError> {
    
        if *sym == 'w' {
            Ok(this + Duration::days(7))
        } else {
            let year = this.year();
            let month = this.month();
    
            let (ny, nm) = if month == 12 {
                (year + 1, 1)
            } else {
                (year, month + 1)
            };
    
            Utc
                .with_ymd_and_hms(ny, nm, 1, 0, 0, 0)
                .single()
                .ok_or(BarBuildError::DateConversion)
        }
    }

    let p = period_number_portion;
    let sym = period_symbol_portion;
    let mut indices: Vec<usize> = Vec::new(); 
    let mut close_dates: Vec<DateTime<Utc>> = Vec::new(); 
    let mut open_dates: Vec<DateTime<Utc>> = Vec::new(); 
   
    if period_symbol_portion == 't' {  // is tick based
        
        let first_id = tick_data[0].0 / 1_000_000;
        let start_idx: usize = (p - (first_id % p as u64) - 1) as usize;
        
        if tick_data.len() < p as usize {
            return Err(BarBuildError::NotEnoughData)
        }

        let max_index = tick_data.len() - 1; 
        indices = (start_idx..=max_index)
            .step_by(p as usize)
            .collect(); 

        for &index in &indices {
            let open_row = tick_data[index];
            let open_date = micros_u64_to_datetime(open_row.1)?;
            open_dates.push(open_date);
           
            let mut close_index = index + (p as usize);
            if close_index > max_index { 
                close_index = max_index; 
            }; 
            let close_row = tick_data[close_index];
            let close_date = micros_u64_to_datetime(close_row.1)?;
            close_dates.push(close_date);

        };

    }
    else {  // is time based
      
        let num_seconds: u64 = match calculate_seconds_in_period(p, sym) {
            Ok(s) => s,
            Err(_) => 0 
        };
        
        let is_week_or_month = ['w', 'M'].contains(&period_symbol_portion);
        let first_ts: u64 = tick_data[0].1 / 1_000_000;

        let mut next_open_date = match is_week_or_month {
            
            true => {
                this_week_or_month(tick_data[0].1, &sym)?
            },
            
            false => {
                let open_ts = candle_open_timestamp(first_ts, num_seconds); 
                match unix_ts_i64_to_datetime(open_ts as i64) {
                    Ok(d) => d,
                    Err(e) => {
                        err_msg("Failed to create initial open date");
                        return Err(e)
                    }
                }
            }
        };
       
        let mut next_close_date = match is_week_or_month {
            
            true => {
                next_week_or_month(next_open_date, &sym)?
            },

            false => {
                let close_ts = candle_close_timestamp(first_ts, num_seconds);
                match unix_ts_i64_to_datetime(close_ts as i64) {
                    Ok(d) => d,
                    Err(e) => {
                        err_msg("Failed to create initial close date");
                        return Err(e)
                    }
                }       
            }
        };

        for (i, row) in tick_data.iter().enumerate() {
            
            let dt = micros_u64_to_datetime(row.1)?;
            
            if dt >= next_open_date { 
           
                open_dates.push(next_open_date); 
                close_dates.push(next_close_date); 
                indices.push(i);
                
                match is_week_or_month {
                    true => {
                        next_open_date = this_week_or_month(row.1, &sym)?;
                        next_close_date = next_week_or_month(
                            next_open_date, 
                            &sym
                        )?;
                    },
                    false => {
                        let norm_ts = (row.1 / 1_000_000) + num_seconds; 
                        next_open_date = { 
                            unix_ts_i64_to_datetime(
                                candle_open_timestamp(
                                    norm_ts as u64, num_seconds as u64
                                ) as i64
                            )?
                        };
                        next_close_date = {   
                            unix_ts_i64_to_datetime(
                                candle_close_timestamp(
                                    norm_ts as u64, num_seconds as u64
                                ) as i64
                            )?
                        }
                    }
                };
            };
        };
    }
    
    match indices.len() > 0 {
        true => Ok((indices, open_dates, close_dates)),
        false => Err(BarBuildError::NotEnoughData)
    }
}


fn candle_open_timestamp<T>(timestamp: T, num_seconds: T) -> T 
where 
    T: PrimInt + Unsigned
{
    timestamp - (timestamp % num_seconds)
}

fn candle_close_timestamp<T>(timestamp: T, num_seconds: T) -> T 
where 
    T: PrimInt + Unsigned
{
    candle_open_timestamp(timestamp, num_seconds) + num_seconds 
}


#[cfg(test)]
mod tests {
    
    use super::*;

    #[test]
    fn candle_test() {
        assert_eq!(4, 4);
    }
}
