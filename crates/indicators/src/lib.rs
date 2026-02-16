pub mod moving_averages;
use moving_averages::*;


pub struct IndicatorSet {
    moving_averages: Option<MaContainer>
}

impl IndicatorSet {

    pub fn new() -> Self {
        Self {
            moving_averages: None
        } 
    }

    pub fn set_moving_averages(&mut self, ma_inputs: Vec<MaInputs>) {

        if self.moving_averages.is_none() {
            let mut ma_container = MaContainer::new();
            for inputs in ma_inputs {
                ma_container.add_new_ma(inputs); 
            };
            self.moving_averages = Some(ma_container);
        };
            
    }

}

