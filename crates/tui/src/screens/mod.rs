use std::cmp::min;


pub mod database;
pub mod candles;
pub mod settings;

use database::DatabaseScreen;
use settings::SettingsScreen;
use candles::CandleScreen;

use app_core::{
    database_ops::{
        DataDownloadStatus,
    }
};


use ratatui::{
    widgets::ListState,
    crossterm::{
        event::KeyEvent,
    },
    style::{
        Color
    },
};




pub fn move_up(state: &mut ListState, len: usize, step: usize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i > 0 => i - min(i, step),
        _ => 0,
    };
    state.select(Some(i));
}

pub fn move_down(state: &mut ListState, len: usize, step: usize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i + step < len => i + step,
        Some(i) if i + step >= len => len,
        _ => 0,
    };
    state.select(Some(i));
}



#[derive(Clone)]
pub enum Focus {
    Operations,
    Main,
    Quit,
}

pub enum AppEvent {
    Input(KeyEvent),
    Output(OutputMsg),
    Clear,
    Tick,
}

// ------------ SCREENS ------------- //
pub enum Screen {
    DatabaseManager(DatabaseScreen),
    CandleBuilder(CandleScreen),
    SystemSettings(SettingsScreen),
    Placeholder,
}


// -------------- MESSAGING ------------------ //
#[derive(Clone)]
pub struct OutputMsg {
    pub text: String,
    pub color: Color,
    pub bold: bool,
    pub bg_color: Option<Color>,
    pub exchange: Option<String>,
    pub ticker: Option<String>,
}

impl OutputMsg {
    pub fn new(
        text: String, 
        color: Color, 
        bold: bool, 
        bg_color: Option<Color>,
        exchange: Option<String>,
        ticker: Option<String>
    ) 
        -> Self {
        OutputMsg { text, color, bold, bg_color, exchange, ticker }
    }
}

impl From<DataDownloadStatus> for OutputMsg {
    
    fn from(status: DataDownloadStatus) -> Self {
        
        match status {
            DataDownloadStatus::Started { exchange, ticker } => {
                OutputMsg::new(
                    format!("  {ticker}: 0%"),
                    Color::Yellow,
                    true,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }

            DataDownloadStatus::Progress {
                exchange,
                ticker,
                percent,
            } => {
                OutputMsg::new(
                    format!("  {ticker}: {percent}%"),
                    Color::Yellow,
                    false,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }

            DataDownloadStatus::Finished { exchange, ticker } => {
                OutputMsg::new(
                    format!("  {ticker}: Finished"),
                    Color::Green,
                    false,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }

            DataDownloadStatus::Error { exchange, ticker } => {
                OutputMsg::new(
                    format!("  {ticker}: ERROR"),
                    Color::Red,
                    true,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }
        }
    }
}


