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

    pub fn set_moving_averages(&self) {

        

    }

}

