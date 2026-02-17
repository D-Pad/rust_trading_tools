use std::{
    collections::HashMap, 
    fmt::{
        Display,
        Formatter,
        self
    }
};

use sqlx::{
    types::{BigDecimal},
};
use serde::{Deserialize, Serialize};

pub mod sma;
pub use sma::{SimpleMovingAverage, SmaInputs};


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

    const SHORT_NAME: &'static str;

}


// ---------------------------- MOVING AVERAGES ---------------------------- //
pub enum MaError {
    InvalidType,
    JsonParseFailed,
}

struct UniqueMaInputs {
    jma_phase: Option<BigDecimal>,
    jma_power: Option<u8>,
    kama_fast: Option<u16>,
    kama_slow: Option<u16>
}

impl UniqueMaInputs {
    fn default() -> Self {
        Self {
            jma_phase: None,
            jma_power: None,
            kama_fast: None,
            kama_slow: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "inputs")]
pub enum MaInputs {
    SMA(SmaInputs),
}

impl Display for MaInputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MaInputs::SMA(_) => write!(f, "MaInputs::SMA")
        }
    }     
}


pub enum MA {
    SMA(SimpleMovingAverage)
}

impl MA {

    pub fn constructor(ma_inputs: MaInputs) -> Self {

        match ma_inputs {
            
            MaInputs::SMA(sma) => {
                let ma = SimpleMovingAverage::empty(sma);
                MA::SMA(ma)
            }
        }
    }
}

impl Display for MA {
    
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
       
        let mut line_string: String = String::from("[");

        let line = match self {
            MA::SMA(sma) => {
                &sma.line  
            }
        };
        
        let length = line.len();
        
        if length > 6 {
            let indices: Vec<usize> = vec![
                0, 1, 2, length - 3, length - 2, length - 1
            ];
            for (i, index) in indices.iter().enumerate() {
                match &line[*index] {
                    Some(v) => line_string.push_str(
                        &format!("{:.4}", v)
                    ),
                    None => line_string.push_str("None")
                };

                if i < 5 { line_string.push_str(", ") };
                if i == 2 { line_string.push_str(" ... , ") }
            };
        }
        else {
            for (i, val) in line.iter().enumerate() {

                let sl = match val {
                    Some(v) => &format!("{:.4}", v),
                    None => "None"
                };

                line_string.push_str(sl);
                if i < length - 1 { line_string.push_str(", ") };
            }
        };
        write!(f, "{}]", line_string)
    }
}


// ------------------------------ CONTAINER -------------------------------- //
pub struct MaContainer {
    pub moving_averages: Vec<MA>
}

impl MaContainer {
    
    pub fn new() -> Self {
        Self {
            moving_averages: Vec::new()
        }
    }

    pub fn add_new_ma(
        &mut self,
        ma_inputs: MaInputs,
    ) {
        let ma = MA::constructor(ma_inputs);
        self.moving_averages.push(ma);
    }

    pub fn add_default_ma(&mut self, type_str: &str) -> Result<(), MaError> {
        match type_str {

            "sma" => {
                self.add_new_ma(MaInputs::SMA(SmaInputs::default()));
            },

            _ => return Err(MaError::InvalidType)

        }

        Ok(())
    }

    pub fn to_strategy_component(&self) 
        -> HashMap<&'static str, Vec<MaInputs>> {

        let mut data: HashMap<&'static str, Vec<MaInputs>> = HashMap::new();

        for ma in &self.moving_averages {
     
            match ma {

                MA::SMA(sma) => {

                    let name = SimpleMovingAverage::SHORT_NAME;
                    data.entry(name)
                        .or_insert(Vec::new())
                        .push(MaInputs::SMA(sma.inputs.clone()));

                } 

            }

        };
        
       data 

    }
}



