use num_traits::{PrimInt, Unsigned};


pub enum TimePeriodError {
    InvalidPeriod,
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    
    }
}
