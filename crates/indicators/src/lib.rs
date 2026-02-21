pub mod moving_averages;
pub use moving_averages::*;
use bars::BarSeries;


pub enum IndicatorTypes {  // Use for strategy template creation
    MovingAverage
}

impl IndicatorTypes {

    pub fn list() -> [(IndicatorTypes, String); 1] {

        [
            (
                IndicatorTypes::MovingAverage, 
                "Moving Average".to_string()
            ),
        ]

    }

}


pub struct IndicatorSet {
    pub moving_average: Option<MA>
}

impl IndicatorSet {

    pub fn build_from_bar_set(&mut self, bar_set: &BarSeries) {
        
        for bar in &bar_set.bars {

            if let Some(ma) = &mut self.moving_average {
            
                match ma {

                    MA::SMA(sma) => {
                        sma.update(
                            bar.component_from_str(&sma.inputs.source)
                        )
                    }
                
                }
            
            }
        
        }
    
    }

    pub fn empty() -> Self {
        Self {
            moving_average: None
        } 
    }

    pub fn set_moving_average(&mut self, ma_inputs: MaInputs) {

        if self.moving_average.is_none() {
            self.moving_average = Some(
                match ma_inputs {
                    MaInputs::SMA(inputs) => MA::SMA(
                        SimpleMovingAverage::empty(inputs)
                    )
                }
            );
        };
            
    }

}

