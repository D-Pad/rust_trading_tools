pub use indicators::{self, *};


pub struct Strategy {
    name: String,
    inputs: StrategyInputs
}

pub struct StrategyInputs {
    moving_averages: Vec<MaInputs>
}

impl StrategyInputs {

    pub fn new(
        moving_averages: Vec<MaInputs>
    ) -> Self {
        Self {
            moving_averages
        }
    }

}


