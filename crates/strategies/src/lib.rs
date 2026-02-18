pub use indicators::{self, *};
use serde::{Serialize, Deserialize};


pub enum StrategyError {
    MaInput(MaError),
    ExportFailed,
}


// ------------------------------- STRATEGY -------------------------------- //
/// # Trading Strategy
///
/// A trading strategy stores a group of indicators, and uses their 
/// calculations to determine entry and exit trading signals.
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


// -------------------------- TEMPLATE RENDERING --------------------------- //
/// # Strategy Component 
///
/// Used to describe a strategy component. Can be used in the 
/// StrategyInputs.add_new_default_component() method to add an indicator to 
/// a strategy.
pub enum StrategyComponentType<'a> {
    MA { ma_type: &'a str },
}


#[derive(Serialize, Deserialize)]
/// # Strategy Input Rendering
///
/// Used for creating and saving strategy templates for later use. This struct 
/// only contains the inputs needed to initialize a Strategy, and does not 
/// perform any indicator calculations on it's own. See the 
/// 'add_new_default_component' method to see how components should be added
/// to a strategy template.
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

    /// ## Add Default Indicator Component
    ///
    /// New indicator components can be added by passing a 
    /// StrategyComponentType struct into this method, which is used to 
    /// generate a new strategy template, but shouldn't have much control over 
    /// the input values of the strategy. In other words, the default input 
    /// values are rendered by default, and should be changed later via the 
    /// TUI or web server.
    ///
    /// To add a component, a StrategyComponentType must be initialized first.
    /// ```
    /// let mut strat = Strategy::empty("My Strategy".to_string());
    /// let comp = StrategyComponentType::MA { ma_type: "sma" };
    /// strat.inputs.add_new_default_component(comp);
    /// ```
    pub fn add_new_default_component(&mut self, comp: StrategyComponentType) 
        -> Result<(), StrategyError> {
        
        match comp {
            StrategyComponentType::MA { ma_type } => {
                
                let inputs = match ma_type {
                    "sma" => {
                        MaInputs::SMA(indicators::SmaInputs::default())
                    },
                    _ => { 
                        return Err(StrategyError::MaInput(
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

// -------------------------- IMPORT & EXPORT ------------------------------ //
pub fn export_strategy_template(strategy: Strategy) 
    -> Result<(), StrategyError> {

    let mut file_name: String = strategy.name.replace(" ", "_");
    let num_chars: usize = file_name.len();  
    if num_chars > 5 && &file_name[&num_chars - 5..] != ".json" {
        file_name.push_str(".json");
    };

    if let Ok(o) = serde_json::to_string_pretty(&strategy.inputs) {
        
        Ok(())
    }
    else {
        Err(StrategyError::ExportFailed)
    }

}


pub fn load_strategy_template (strategy_name: &str) {
    
}

