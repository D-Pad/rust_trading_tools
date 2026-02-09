use ratatui::{
    widgets::{
        ListState
    },
    crossterm::{
        event::{
            KeyEvent,
        }
    }
};


// -------------- CANDLE SCREEN ------------- //
pub struct CandleScreen {
    step: CandleStep,
    exchange_state: ListState,
    pair_state: ListState,
    interval_state: ListState 
}

enum CandleStep {
    Exchange,
    Pair,
    Interval,
    Ready,
}

impl CandleScreen {

    pub fn new() -> Self {
        
        CandleScreen {
            step: CandleStep::Exchange,
            exchange_state: ListState::default(),
            pair_state: ListState::default(),
            interval_state: ListState::default()
        }
    
    }

    pub fn draw(&mut self) {

    }

    pub fn handle_key(&self, key: KeyEvent) {

    }

    pub const SCREEN_NAME: &'static str = "Candle Builder";

    pub const SCREEN_OPTIONS: [&'static str; 0] = [];
}


