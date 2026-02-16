use std::collections::VecDeque;

use sqlx::{
    types::{BigDecimal},
};

use crate::MovingAverage;


pub struct SimpleMovingAverage {
    source: String,
    line: Vec<Option<BigDecimal>>,
    lookback_values: VecDeque<BigDecimal>,
    period: u16,
}

impl SimpleMovingAverage {

    pub fn empty(
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



