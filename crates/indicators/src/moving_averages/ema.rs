use std::collections::VecDeque;

use serde::{Serialize, Deserialize};
use bigdecimal::BigDecimal;

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
    k_val: BigDecimal,
}

impl EmaInputs {
    
    pub fn new(period: u16, src: Option<String>) -> Self {

        let source = match src {
            Some(s) => s,
            None => "close".to_string()
        };

        let two: BigDecimal = BigDecimal::from(2u8);
        let denom: BigDecimal = BigDecimal::from(period + 1);
        let k_val: BigDecimal = two / denom;
        Self { period, source, k_val }

    }

    pub fn default() -> Self {
        Self::new(13, None)
    }
}

pub struct ExponentialMovingAverage {
    pub inputs: EmaInputs, 
    pub line: Vec<Option<BigDecimal>>,
    lookback_values: Option<VecDeque<BigDecimal>>,
}

impl ExponentialMovingAverage {

    pub fn empty(inputs: EmaInputs) -> Self {

        let line: Vec<Option<BigDecimal>> = Vec::new();
        let lookback_values = Some(VecDeque::new());

        Self {
            inputs, 
            line,
            lookback_values
        } 
    }

}

impl MovingAverage for ExponentialMovingAverage {
    
    fn calculate(&self, input_val: Option<&BigDecimal>) -> Option<BigDecimal> {
        
        match (self.line.len() as u16) < self.inputs.period {
            true => None,
            false => {
                
                // Initial SMA Value
                if (self.line.len() as u16) == self.inputs.period {
                    if let Some(vals) = &self.lookback_values {
                        let total: BigDecimal = vals 
                            .iter()
                            .sum();
                        let avg = total / self.inputs.period;
                        Some(avg)
                    }
                    else {
                        eprintln!(
                            "\x1b[1;31mReached an unreachable line\x1b[0m"
                        );
                        None // Should be unreachable
                    }
                }

                // EMA Value
                else {

                    // EMA=Price(t)×k+EMA(y)×(1−k)
                    // where:
                    // t=today
                    // y=yesterday
                    // N=number of days in EMA
                    // k=2÷(N+1)
                   
                    if let Some(price) = input_val {
                  
                        let i: usize = self.line.len();
                        if let Some(prev) = &self.line[i] {
                            let one = BigDecimal::from(1u8);
                            let kxp = price * &self.inputs.k_val;
                            Some(kxp + prev * (one - &self.inputs.k_val)) 
                        }
                        else {
                            None
                        }

                    }
                    else {
                        None 
                    }
                }
            }
        }

    }

    fn set_live_preview(&mut self, input_val: &BigDecimal) {
        let i = self.line.len() - 1;
        let val: Option<BigDecimal> = self.calculate(Some(input_val));
        self.line[i] = val; 
    }

    fn update(&mut self, input_val: &BigDecimal) {
       
        if let Some(ref mut vals) = self.lookback_values {
            vals.push_back(input_val.clone());
            if vals.len() as u16 > self.inputs.period {
                self.lookback_values = None;
            };
        }; 
            
        let val: Option<BigDecimal> = self.calculate(Some(input_val));
        self.line.push(val);

    }

    const SHORT_NAME: &'static str = "sma";

}



