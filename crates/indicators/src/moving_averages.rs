use sqlx::{
    types::{BigDecimal},
};


pub enum MaError {
    InvalidTypeStr
} 

// ----------------------------- MA INPUT VALUES --------------------------- //
pub enum MovingAverageType {
    SMA
}

impl MovingAverageType {
    
    pub fn get_default_inputs(&self) -> MovingAverageInput {
      
        let period: u16 = 13;
        let source: String = "close".to_string();

        match self {
            Self::SMA => {
                MovingAverageInput::SMA { period, source }
            }
        }
    
    }

    pub fn get_defaults_from_str(ma_type: &str) 
        -> Result<MovingAverageInput, MaError> {
        
        match ma_type {
            "sma" => {
                let ma = MovingAverageType::SMA; 
                Ok(ma.get_default_inputs()) 
            },

            _ => Err(MaError::InvalidTypeStr) 
        }
    
    }

}

pub enum MovingAverageInput {
    
    SMA {
        period: u16,
        source: String 
    },

}


// -------------------------- MOVING AVERAGE STRUCT ------------------------ //
pub struct MovingAverage {
    pub line: Vec<BigDecimal>,
    pub inputs: MovingAverageInput,
    pub ma_type: MovingAverageType,
}

impl MovingAverage {
    
    pub fn new(
        data_source: Vec<&BigDecimal>, 
        ma_type: MovingAverageType
    ) -> Self {
        
        Self {
            line: Vec::new(),
            inputs: ma_type.get_default_inputs(),
            ma_type: ma_type,
        } 
    
    }

}

