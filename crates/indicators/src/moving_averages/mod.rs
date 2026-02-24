use std::{
    fmt::{
        Display,
        Formatter,
        self
    }
};

use serde::{Deserialize, Serialize};
use bigdecimal::BigDecimal;

pub mod sma;
pub use sma::{SimpleMovingAverage, SmaInputs};
pub mod ema;
pub use ema::{ExponentialMovingAverage, EmaInputs};


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

// struct UniqueMaInputs {
//     jma_phase: Option<BigDecimal>,
//     jma_power: Option<u8>,
//     kama_fast: Option<u16>,
//     kama_slow: Option<u16>
// }
// 
// impl UniqueMaInputs {
//     fn default() -> Self {
//         Self {
//             jma_phase: None,
//             jma_power: None,
//             kama_fast: None,
//             kama_slow: None,
//         }
//     }
// }

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "inputs")]
/// # Moving average inputs
///
/// Some moving average types have a unique set of input values only relevant
/// to themselves. Input values are as follows.
///
/// ## SMA & EMA
///   - period: The number of periods in the lookback values.
///   - source: The candle component that the moving average will be built 
///             from. Only applicable when using the `build_from_bar_set()` 
///             method on the MovingAverage trait.
pub enum MaInputs {
    SMA(SmaInputs),
    EMA(EmaInputs),
}

impl MaInputs {

    pub fn set_period(&mut self, period: u16) {

        match self {

            MaInputs::SMA(sma) => {
                sma.period = period;
            },

            MaInputs::EMA(ema) => {
                ema.period = period;
            },

        }

    }

    pub const MA_TYPES: [&'static str; 2] = [
        "sma", 
        "ema",
    ];

}

impl Display for MaInputs {
    
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MaInputs::SMA(_) => write!(f, "MaInputs::SMA"),
            MaInputs::EMA(_) => write!(f, "MaInputs::EMA")
        }
    }     

}


pub enum MA {
    SMA(SimpleMovingAverage),
    EMA(ExponentialMovingAverage),
}

impl MA {

    pub fn constructor(ma_inputs: MaInputs) -> Self {

        match ma_inputs {
            
            MaInputs::SMA(sma) => {
                let ma = SimpleMovingAverage::empty(sma);
                MA::SMA(ma)
            },

            MaInputs::EMA(ema) => {
                let ma = ExponentialMovingAverage::empty(ema);
                MA::EMA(ma)
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
            },
            MA::EMA(ema) => {
                &ema.line
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



