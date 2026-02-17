pub use indicators::{self, *};


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

    pub fn add_new_default_component(&mut self, comp: StrategyComponentType) {
        
        match comp {
            StrategyComponentType::MA { ma_type } => {
                match ma_type {
                    "sma" => {
                        let inputs = MaInputs::SMA(
                            indicators::SmaInputs::default()
                        );
                        if let None = &self.moving_averages {
                            let mut vector = Vec::new();
                            vector.push(inputs);
                            self.moving_averages = Some(vector); 
                        }
                        else if let Some(vector) = &mut self.moving_averages {
                            vector.push(inputs)
                        }
                    },
                    _ => {}
                }
            }
        }

    }

}


