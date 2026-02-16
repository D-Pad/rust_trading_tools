pub use bars::{self, *};
pub use indicators::{self, *};


pub struct Chart {
    pub bars: BarSeries,
    // pub indicators: IndicatorSet,
}

impl Chart {
    
    pub fn new(bars: BarSeries) -> Self {
        Chart { bars }
    }

    pub fn num_bars_on_chart(&self) -> usize {
        self.bars.bars.len()
    }

}


