use std::collections::VecDeque;

use sqlx::{
    types::{BigDecimal},
};


// ----------------------------- COMMON TRAITS ----------------------------- //
pub trait MovingAverage {
   
    /// # Calculat the Latest Moving Average Point
    ///
    /// This method only performs the calculation of the most recent moving
    /// average, and returns it. It does NOT mutate or modify any attributes.
    fn calculate(&self, input_val: Option<&BigDecimal>) -> Option<BigDecimal>;

    /// # Update With Real Time Price Data 
    ///
    /// This method is used to calculate the moving average value using live
    /// data. Useful for previewing a moving average as it's forming in real 
    /// time. In a live data situation, don't forget to call this method
    /// using the final candle state before calling update(), otherwise you'll
    /// lose data accuracy.
    fn set_live_preview(&mut self, input_val: &BigDecimal);

    /// # Update the Moving Average Line 
    ///
    /// Updates the moving average line. Call this method when a candle closes.
    /// Calling this method will automatically call self.calculation, and 
    /// append the line with the newly calculated value.
    fn update(&mut self, input_val: &BigDecimal);

}


// ---------------------------- MOVING AVERAGES ---------------------------- //
pub enum MaError {
    InvalidType
}

pub enum MA {
    SMA(SimpleMovingAverage)
}


pub struct SimpleMovingAverage {
    source: String,
    line: Vec<Option<BigDecimal>>,
    lookback_values: VecDeque<BigDecimal>,
    period: u16,
}

impl SimpleMovingAverage {

    fn empty(
        period: u16,
        src: Option<String>,
    ) -> Self {

        let source = match src {
            Some(s) => s,
            None => "close".to_string()
        };

        let mut line: Vec<Option<BigDecimal>> = Vec::new();
        let lookback_values: VecDeque<BigDecimal> = VecDeque::new();

        Self {
            source,
            line,
            period,
            lookback_values
        } 

    }

}

impl MovingAverage for SimpleMovingAverage {
    
    fn calculate(&self, input_val: Option<&BigDecimal>) -> Option<BigDecimal> {
        
        match (self.lookback_values.len() as u16) < self.period {
            true => None,
            false => {
                let total: BigDecimal = self.lookback_values.iter().sum();
                let avg = total / self.period;
                Some(avg)
            }
        }

    }

    fn set_live_preview(&mut self, input_val: &BigDecimal) {
        
        if self.lookback_values.len() > 0 {

            let mut i = self.lookback_values.len() - 1;
            self.lookback_values[i] = input_val.clone();
            
            let val: Option<BigDecimal> = self.calculate(None);
            i = self.line.len() - 1; 
            self.line[i] = val; 

        }; 

    }

    fn update(&mut self, input_val: &BigDecimal) {
        
        self.lookback_values.push_back(input_val.clone());
        
        if self.lookback_values.len() as u16 > self.period {
            let _ = self.lookback_values.pop_front();
        };

        let val: Option<BigDecimal> = self.calculate(None);
        self.line.push(val);

    }

}



// ------------------------------ CONTAINER -------------------------------- //
pub struct MaContainer {
    moving_averages: Vec<MA>
}

impl MaContainer {
    
    fn new() -> Self {
        Self {
            moving_averages: Vec::new()
        }
    }

    fn add_new_ma(
        &mut self, 
        period: u16, 
        ma_type: &str,
        src: Option<String>
    ) 
        -> Result<(), MaError> {

        let ma = match ma_type {

            "sma" => MA::SMA(
                SimpleMovingAverage::empty(period, src)
            ),

            _ => return Err(MaError::InvalidType)

        };

        self.moving_averages.push(ma);

        Ok(())

    }

}


