use std::collections::VecDeque;

use serde::{Serialize, Deserialize};
use bigdecimal::BigDecimal;

use crate::{
    MovingAverage,
    MaInputVals,
};


#[derive(Serialize, Deserialize, Clone, Debug)]
/// # Jurik Moving Average Inputs
///
/// Input values for a simple moving average.
///   - period: The number of periods to lookback on for each new calculation.
///   - source: Only applicable when calling the 'build_from_bar_set' method.
///   - power: Sets the exponent in the Jurik Adaptive MA calculation.
///   - phase: Used in the Jurik Adaptive MA calculation.
pub struct JmaInputs {
    pub period: u16,
    pub source: String,
    pub phase: f32,
    pub power: f32,
}

impl JmaInputs {
    
    pub fn new(
        period: u16, 
        src: Option<String>,
        phase: f32,
        power: f32
    ) -> Self {

        let source = match src {
            Some(s) => s,
            None => "close".to_string()
        };

        Self { period, source, phase, power }

    }

    pub fn default() -> Self {
        Self { 
            period: 13, 
            source: "close".to_string(), 
            phase: 3.0, 
            power: 2.0
        }
    }

}

pub struct JurikMovingAverage {
    pub inputs: JmaInputs, 
    pub line: Vec<Option<BigDecimal>>,
    lookback_values: VecDeque<BigDecimal>,
}

impl JurikMovingAverage {

    pub fn empty(inputs: JmaInputs) -> Self {

        let line: Vec<Option<BigDecimal>> = Vec::new();
        let lookback_values: VecDeque<BigDecimal> = VecDeque::new();

        Self {
            inputs, 
            line,
            lookback_values
        } 
    }

}

impl MovingAverage for JurikMovingAverage {
    
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

}


impl MaInputVals for JmaInputs {
    const SHORT_NAME: &'static str = "jma";
}


