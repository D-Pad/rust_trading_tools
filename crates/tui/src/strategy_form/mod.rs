use strategies::{
    Strategy,
    MaInputs,
};
use crate::{
    FormField
};



pub struct StrategyConstructor {
    pub strategy: Strategy,
}

impl StrategyConstructor {

    pub fn new() -> Self {
        Self {
            strategy: Strategy::empty()
        }
    }

    pub fn get_form_rows(&self) -> Vec<(String, FormField<StrategyKeys>)> {

        let mut rows: Vec<(String, FormField<StrategyKeys>)> = Vec::new();

        if let Some(ma) = &self.strategy.inputs.moving_average {
            match ma {
                MaInputs::SMA(inputs) => {
                    inputs;
                },
                _ => {}
            }
        };

        rows

    }



}



// ---------------- STRATEGY FORM INPUT STRUCTS AND ENUMS ------------------ //
pub enum StrategyKeys {
    MovingAverage(MovingAverageKeys)
}

pub enum MovingAverageKeys {
    MaType,
    Period,
    Phase,
    Power,
}



