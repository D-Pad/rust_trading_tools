use num_traits::{PrimInt, Unsigned};
use chrono::{DateTime, Datelike, TimeZone, Utc, Duration};


#[derive(Debug)]
pub enum TimePeriodError {
    InvalidPeriod,
    DateConversion,
    NotEnoughData
}

impl std::fmt::Display for TimePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimePeriodError::InvalidPeriod => write!(
                f, "TimePeriodError::InvalidPeriod"),
            TimePeriodError::DateConversion => write!(
                f, "TimePeriodError::DateConversion"),
            TimePeriodError::NotEnoughData => write!(
                f, "TimePeriodError::NotEnoughData"),
        }
    }
}


pub const VALID_PERIODS: &[char; 7] = &['s', 'm', 'h', 'd', 'w', 'M', 't'];


pub fn calculate_seconds_in_period(
    periods: u64, 
    symbol: char
) -> Result<u64, TimePeriodError> {
    
    let num_seconds = match symbol {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        'd' => 86400,
        'M' => 2592000,
        'Y' => 31536000, 
        _ => return Err(TimePeriodError::InvalidPeriod)
    };
    
    Ok(num_seconds * periods) 
}


pub fn get_current_unix_timestamp() -> u64 {
    Utc::now().timestamp() as u64 
}


pub fn get_period_portions_from_string(period: &str) 
    -> Result<(char, u64), TimePeriodError> {
    
    let period_key = match period.chars().last() {
        Some(c) => c, 
        None => { return Err(
                TimePeriodError::InvalidPeriod
            ) 
        } 
    };
    
    if !VALID_PERIODS.contains(&period_key) {
        return Err(TimePeriodError::InvalidPeriod) 
    };
    
    let period_n: u64 = match period[0..period.len() - 1].parse::<u64>() {
        Ok(v) => v,
        Err(_) => return Err(TimePeriodError::InvalidPeriod) 
    };

    Ok((period_key, period_n))

}


pub fn period_is_time_based(period_symbol: char) -> bool {
    if period_symbol == 't' { 
        false 
    }
    else { 
        true 
    }
}


// -------------------------- DATE CONVERSION ------------------------------ //
fn micros_u64_to_datetime(
    microseconds: u64
) -> Result<DateTime<Utc>, TimePeriodError> {
    
    let secs = (microseconds / 1_000_000) as i64;
    let nsecs = (microseconds % 1_000_000) as u32;
   
    match Utc.timestamp_opt(secs, nsecs) {
        chrono::LocalResult::Single(dt) => Ok(dt),

        chrono::LocalResult::Ambiguous(_, _) => {
            Err(TimePeriodError::DateConversion)
        }

        chrono::LocalResult::None => {
            Err(TimePeriodError::DateConversion)
        }
    }
}


fn unix_ts_i64_to_datetime(
    seconds: i64
) -> Result<DateTime<Utc>, TimePeriodError> {
    
    match Utc.timestamp_opt(seconds, 0) {
        chrono::LocalResult::Single(dt) => Ok(dt),

        chrono::LocalResult::Ambiguous(_, _) => {
            Err(TimePeriodError::DateConversion)
        }

        chrono::LocalResult::None => {
            Err(TimePeriodError::DateConversion)
        }
    }
}


pub fn db_timestamp_to_date_string(timestamp: u64) -> String {
    match micros_u64_to_datetime(timestamp) {
        Ok(v) => v.format("%Y-%m-%d %H:%M:%S").to_string(),
        Err(_) => "?".to_string()
    }
}


// --------------------------- CANDLE PERIOD ------------------------------- //
pub fn get_tick_indices_and_dates<'a> (
    tick_data: &'a [(u64, u64, f64, f64)],
    period_number: u64,
    period_symbol: char
) -> Result<
        (Vec<usize>, Vec<DateTime<Utc>>, Vec<DateTime<Utc>>), 
        TimePeriodError
    > 
{

    fn err_msg(msg: &'static str) {
        println!("\x1b[1;31m{}\x1b[0m", msg);
    }

    fn this_week_or_month(
        ts: u64, 
        sym: &char
    ) -> Result<DateTime<Utc>, TimePeriodError> {
      
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
            Result<DateTime<Utc>, TimePeriodError> {
            
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
                None => Err(TimePeriodError::DateConversion)
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
    ) -> Result<DateTime<Utc>, TimePeriodError> {
    
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
                .ok_or(TimePeriodError::DateConversion)
        }
    }

    let mut indices: Vec<usize> = Vec::new(); 
    let mut close_dates: Vec<DateTime<Utc>> = Vec::new(); 
    let mut open_dates: Vec<DateTime<Utc>> = Vec::new(); 
   
    if period_symbol == 't' {  // is tick based
        
        let first_id = tick_data[0].0 / 1_000_000;
        let start_idx: usize = (
            period_number - (first_id % period_number as u64) - 1
        ) as usize;
        
        if tick_data.len() < period_number as usize {
            return Err(TimePeriodError::NotEnoughData)
        }

        let max_index = tick_data.len() - 1; 
        indices = (start_idx..=max_index)
            .step_by(period_number as usize)
            .collect(); 

        for &index in &indices {
            let open_row = tick_data[index];
            let open_date = micros_u64_to_datetime(open_row.1)?;
            open_dates.push(open_date);
           
            let mut close_index = index + (period_number as usize);
            if close_index > max_index { 
                close_index = max_index; 
            }; 
            let close_row = tick_data[close_index];
            let close_date = micros_u64_to_datetime(close_row.1)?;
            close_dates.push(close_date);

        };

    }
    else {  // is time based
      
        let num_seconds: u64 = match calculate_seconds_in_period(
            period_number, period_symbol 
        ) {
            Ok(s) => s,
            Err(_) => 0 
        };
        
        let is_week_or_month = ['w', 'M'].contains(&period_symbol);
        let first_ts: u64 = tick_data[0].1 / 1_000_000;

        let mut next_open_date = match is_week_or_month {
            
            true => {
                this_week_or_month(tick_data[0].1, &period_symbol)?
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
                next_week_or_month(next_open_date, &period_symbol)?
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
                        next_open_date = this_week_or_month(
                            row.1, &period_symbol
                        )?;
                        next_close_date = next_week_or_month(
                            next_open_date, 
                            &period_symbol
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
        false => Err(TimePeriodError::NotEnoughData)
    }
}


pub fn candle_open_timestamp<T>(timestamp: T, num_seconds: T) -> T 
where 
    T: PrimInt + Unsigned
{
    timestamp - (timestamp % num_seconds)
}

pub fn candle_close_timestamp<T>(timestamp: T, num_seconds: T) -> T 
where 
    T: PrimInt + Unsigned
{
    candle_open_timestamp(timestamp, num_seconds) + num_seconds 
}


