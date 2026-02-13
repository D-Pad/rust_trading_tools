use std::collections::HashMap;

use ratatui::{
    widgets::{
        Block,
        Borders,
        List,
        ListState,
        ListItem,
    },
    crossterm::{
        event::{
            KeyEvent,
        }
    },
    layout::{
        Layout,
        Constraint,
        Direction,
        Rect,
    },
    style::{
        Style,
        Modifier,
    },
    Frame
};
use tokio::task::JoinHandle;


// -------------- CANDLE SCREEN ------------- //
#[derive(Clone)]
enum CandleAction {
    Exchange,
    Ticker,
    Period,
    Build,
}

impl CandleAction {
    
    fn title(&self) -> &'static str {
        match self {
            Self::Exchange => "Exchange Selection",
            Self::Ticker => "Asset Pair Selection",
            _ => "" // No other arms need a title 
        } 
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Exchange => "Exchange",
            Self::Ticker => "Ticker",
            Self::Period => "Period",
            Self::Build => "Build"
        } 
    }

}

enum CandleFocus {
    Top,
    Bottom
}

pub struct CandleScreen {
    step: CandleAction,
    exchange_state: ListState,
    pair_state: ListState,
    interval_state: String,
    focus: CandleFocus,
    top_state: ListState,
    btm_state: ListState,
    btm_item_data: Vec<String>,
    token_pairs: HashMap<String, Vec<String>>,
    task: Option<JoinHandle<()>>,
}

impl CandleScreen {

    pub fn new(token_pairs: HashMap<String, Vec<String>>) -> Self {
       
        let mut top_state = ListState::default();
        top_state.select(Some(0));
        let task: Option<JoinHandle<()>> = None;

        CandleScreen {
            step: CandleAction::Exchange,
            exchange_state: ListState::default(),
            pair_state: ListState::default(),
            interval_state: String::new(),
            focus: CandleFocus::Top,
            top_state,
            btm_state: ListState::default(),
            btm_item_data: Vec::new(),
            token_pairs,
            task,
        }
    
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {

        let nested_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(30),
                Constraint::Percentage(70),
            ])
            .split(area);

        let top_items: Vec<ListItem> = Self::SCREEN_OPTIONS
            .iter()
            .map(|v| ListItem::new(v.name()))
            .collect();

        let top_list = List::new(top_items)
            .block(
                Block::default()
                    .title(Self::SCREEN_NAME)
                    .borders(Borders::ALL)
            )
            .highlight_style(
                if let CandleFocus::Top = self.focus {
                    Style::default().add_modifier(Modifier::REVERSED).green()
                } else {
                    Style::default()
                }
            );
        
        frame.render_stateful_widget(
            top_list,
            nested_chunks[0],
            &mut self.top_state
        );

        if matches!(self.focus, CandleFocus::Bottom) {

            self.btm_item_data = match self.step {
                CandleAction::Exchange => { 
                    Vec::new() 
                },                 
                CandleAction::Ticker => { 
                    Vec::new() 
                }, 
                _ => { return } 
            };

            let btm_items: Vec<ListItem> = self.btm_item_data.iter()
                .map(|v| ListItem::new(&v[..]))
                .collect();

            let btm_list = List::new(btm_items)
                .block(
                    Block::default()
                        .title(self.step.title())
                        .borders(Borders::ALL)
                )
                .highlight_style(
                    if let CandleFocus::Bottom = self.focus {
                        Style::default()
                            .add_modifier(Modifier::REVERSED)
                            .green()
                    } else {
                        Style::default()
                    }
                );
            
            frame.render_stateful_widget(
                btm_list, 
                nested_chunks[1],
                &mut self.btm_state
            );
        }
    }

    pub fn handle_key(&self, key: KeyEvent) {

    }

    pub const SCREEN_NAME: &'static str = "Candle Builder";

    pub const SCREEN_OPTIONS: [CandleAction; 4] = [
        CandleAction::Exchange,
        CandleAction::Ticker,
        CandleAction::Period,
        CandleAction::Build,
    ];
}


