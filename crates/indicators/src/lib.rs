use bars::BarSeries;

pub mod moving_averages;
use moving_averages::*;


pub struct IndicatorInputValues {
    moving_averages: Option<Vec<MovingAverageInput>>
}

impl IndicatorInputValues {

    pub fn new() -> Self {
        Self {
            moving_averages: None,
        }
    }

    /// # Create a Set of Default Indicator Input Values
    ///
    /// For moving averages, pass a vector of string slices. This is requried
    /// because it's common to use many moving averages together. As such, the 
    /// 'IndicatorSet.moving_averages' is a vector of MovingAverage structs.
    pub fn defaults(ma_types: Option<Vec<&str>>) -> Self {
      
        let mut indicator_inputs = IndicatorInputValues::new(); 
        
        if let Some(t) = ma_types {
           
            let mut ma_inputs: Vec<MovingAverageInput> = Vec::new();

            for ma_type in t {
                
                let inputs = MovingAverageType::get_defaults_from_str(ma_type);
                
                if let Ok(inp) = inputs {
                    ma_inputs.push(inp);
                };

            };

            indicator_inputs.moving_averages = Some(ma_inputs);

        };

        indicator_inputs

    }
}


pub struct IndicatorSet {
    moving_averages: Option<Vec<MovingAverage>>
}

impl IndicatorSet {
    
    pub fn new(
        bar_series: &BarSeries,
        indicator_inputs: IndicatorInputValues,
    ) -> Self {

        let mut moving_averages: Option<Vec<MovingAverage>> = None;

        if let Some(inp) = indicator_inputs.moving_averages {
          
            let mut mas: Vec<MovingAverage> = Vec::new();

            for ma_input in inp {
             
                 
                
            };

        };

        Self {
            moving_averages
        }

    }

}

