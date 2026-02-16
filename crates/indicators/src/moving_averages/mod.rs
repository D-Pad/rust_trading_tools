use sqlx::{
    types::{BigDecimal},
};

pub mod sma;
use sma::SimpleMovingAverage;

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

struct UniqueMaInputs {
    jma_phase: Option<BigDecimal>,
    jma_power: Option<u8>,
    kama_fast: Option<u16>,
    kama_slow: Option<u16>
}

impl UniqueMaInputs {
    fn new() -> Self {
        Self {
            jma_phase: None,
            jma_power: None,
            kama_fast: None,
            kama_slow: None,
        }
    }
}

pub enum MaType {
    SMA
}

pub enum MA {
    SMA(SimpleMovingAverage)
}

impl MA {

    pub fn constructor(
        ma_type: MaType,
        period: u16,
        source: String,
        optional_inputs: Option<UniqueMaInputs>
    ) -> Self {

        match ma_type {
            
            MaType::SMA => {

                let ma = SimpleMovingAverage::empty(
                    period, 
                    Some(source.clone())
                );
                MA::SMA(ma)

            }

        }

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

}


