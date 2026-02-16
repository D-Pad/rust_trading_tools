pub use bars::{self, *};
pub use indicators::{self, *};


pub struct Chart {
    pub bars: BarSeries,
    pub indicator_set: IndicatorSet,
}

impl Chart {
    
    pub fn new(bars: BarSeries) -> Self {
        let indicator_set = IndicatorSet::empty(); 
        Chart { bars, indicator_set }
    }

    pub fn populate_indicator_values(&mut self) {
        self.indicator_set.build_from_bar_set(&self.bars);
    }

    pub fn num_bars_on_chart(&self) -> usize {
        self.bars.bars.len()
    }

}


