pub use indicators::{self, *};
use serde::{Serialize, Deserialize};


pub enum StrategyInputError {
    MA(MaError)
}


pub enum StrategyComponentType<'a> {
    MA { ma_type: &'a str },
}


pub struct Strategy {
    pub name: String,
    pub inputs: StrategyInputs
}

impl Strategy {
    pub fn empty(name: String) -> Self {
        Self { 
            name, 
            inputs: StrategyInputs::empty()
        }
    }
}


#[derive(Serialize, Deserialize)]
pub struct StrategyInputs {
    pub moving_averages: Option<Vec<MaInputs>>
}

impl StrategyInputs {

    pub fn new(
        moving_averages: Option<Vec<MaInputs>>
    ) -> Self {
        
        Self {
            moving_averages
        }
    
    }

    pub fn empty() -> Self {
        
        Self {
            moving_averages: None
        }
    
    }

    pub fn add_new_default_component(&mut self, comp: StrategyComponentType) 
        -> Result<(), StrategyInputError> {
        
        match comp {
            StrategyComponentType::MA { ma_type } => {
                
                let inputs = match ma_type {
                    "sma" => {
                        MaInputs::SMA(indicators::SmaInputs::default())
                    },
                    _ => { 
                        return Err(StrategyInputError::MA(
                            MaError::InvalidType
                        ))
                    }
                };
                
                if let None = &self.moving_averages {
                    let mut vector = Vec::new();
                    vector.push(inputs);
                    self.moving_averages = Some(vector); 
                }
                else if let Some(vector) = &mut self.moving_averages {
                    vector.push(inputs)
                };
            }
        }

        Ok(())
    }

}


