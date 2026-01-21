use bars::{BarSeries}


pub struct Chart {
    pub bars: BarSeries
}

impl Chart {
    
    pub fn new(bars: BarSeries) {
        Chart { bars }
    }

    pub fn num_bars_on_chart(&self) -> usize {
        self.bars.len()
    }

}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    
    }
}

