pub mod moving_averages;
pub use moving_averages::*;
use bars::BarSeries;


pub enum IndicatorTypes {  // Use for strategy template creation
    MovingAverages
}

impl IndicatorTypes {

    pub fn list() -> [(IndicatorTypes, String); 1] {

        [
            (
                IndicatorTypes::MovingAverages, 
                "Multi Moving Average".to_string()
            ),
        ]

    }

}


pub struct IndicatorSet {
    pub ma_container: Option<MaContainer>
}

impl IndicatorSet {

    pub fn build_from_bar_set(&mut self, bar_set: &BarSeries) {
        
        if let Some(mac) = &mut self.ma_container {

            for bar in &bar_set.bars {
            
                for ma in &mut mac.moving_averages {

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

    }

    pub fn empty() -> Self {
        Self {
            ma_container: None
        } 
    }

    pub fn set_moving_averages(&mut self, ma_inputs: Vec<MaInputs>) {

        if self.ma_container.is_none() {
            let mut ma_container = MaContainer::new();
            for inputs in ma_inputs {
                ma_container.add_new_ma(inputs); 
            };
            self.ma_container = Some(ma_container);
        };
            
    }

}

