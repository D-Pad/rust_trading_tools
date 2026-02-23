use std::collections::VecDeque;

use sqlx::{
    types::{BigDecimal},
};
use serde::{Serialize, Deserialize};

use crate::MovingAverage;


#[derive(Serialize, Deserialize, Clone, Debug)]
/// # Exponential Moving Average Inputs
///
/// Input values for a simple moving average.
///   - period: The number of periods to lookback on for each new calculation.
///   - source: Only applicable when calling the 'build_from_bar_set' method.
pub struct EmaInputs {
    pub period: u16,
    pub source: String,
}

impl EmaInputs {
    
    pub fn new(period: u16, src: Option<String>) -> Self {

        let source = match src {
            Some(s) => s,
            None => "close".to_string()
        };

        Self { period, source }

    }

    pub fn default() -> Self {
        Self { period: 13, source: "close".to_string() }
    }
}

pub struct ExponentialMovingAverage {
    pub inputs: EmaInputs, 
    pub line: Vec<Option<BigDecimal>>,
    lookback_values: VecDeque<BigDecimal>,
}

impl ExponentialMovingAverage {

    pub fn empty(inputs: EmaInputs) -> Self {

        let line: Vec<Option<BigDecimal>> = Vec::new();
        let lookback_values: VecDeque<BigDecimal> = VecDeque::new();

        Self {
            inputs, 
            line,
            lookback_values
        } 
    }

}

impl MovingAverage for ExponentialMovingAverage {
    
    fn calculate(&self, _input_val: Option<&BigDecimal>) -> Option<BigDecimal> {
        
        match (self.lookback_values.len() as u16) < self.inputs.period {
            true => None,
            false => {
                let total: BigDecimal = self.lookback_values.iter().sum();
                let avg = total / self.inputs.period;
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
        
        if self.lookback_values.len() as u16 > self.inputs.period {
            let _ = self.lookback_values.pop_front();
        };

        let val: Option<BigDecimal> = self.calculate(None);
        self.line.push(val);

    }

    const SHORT_NAME: &'static str = "sma";

}



