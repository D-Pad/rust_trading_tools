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
            KeyCode
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
        Color
    },
    Frame
};
use tokio::{
    task::JoinHandle,
    sync::mpsc::UnboundedSender,
};

use crate::{move_up, move_down, AppEvent, OutputMsg};
use timestamp_tools::{
    period_is_valid,
    VALID_PERIODS,
};
use string_helpers::multi_line_to_single_line;


// ---------------------------- INFO STRINGS ------------------------------- //
const INFO_STRINGS: [&'static str; 4] = [
    r#"Displays a list of available exchanges. Must choose an exchange before 
    choosing a ticker symbol."#,

    r#"Displays a list of available ticker symbols from the given exchange.
    Must choose an exchange before choosing a ticker so that available tickers
    can be looked up."#,

    r#"Press enter to begin typing a period length."#,

    r#"Builds a set of candles if all input values are provided. The candle
    data will exported as a CSV file."#
];


// -------------- CANDLE SCREEN ------------- //
#[derive(Clone)]
enum CandleAction {
    Exchange,
    Ticker,
    Period,
    Build,
    None,
}

impl CandleAction {
    
    fn title(&self) -> &'static str {
        match self {
            Self::Exchange => "Exchange Selection",
            Self::Ticker => "Asset Pair Selection",
            _ => "Info" // No other arms need a title 
        } 
    }

}

pub enum CandleFocus {
    Top,
    Bottom,
    InputMode,
}

pub struct CandleScreen {
    exchange: String,
    ticker: String,
    period: String,
    previous_period: String,

    step: CandleAction,
    pub focus: CandleFocus,
    top_state: ListState,
    btm_state: ListState,
    btm_item_data: Vec<String>,
    token_pairs: HashMap<String, Vec<String>>,
    task: Option<JoinHandle<()>>,
    pub transmitter: UnboundedSender<AppEvent>,
}

impl CandleScreen {

    pub fn new(
        token_pairs: HashMap<String, Vec<String>>,
        transmitter: UnboundedSender<AppEvent>,
    ) -> Self {
       
        let mut top_state = ListState::default();
        top_state.select(Some(0));
        let task: Option<JoinHandle<()>> = None;

        CandleScreen {
            exchange: String::new(),
            ticker: String::new(),
            period: String::new(),
            previous_period: String::new(),  // For error checking
            
            step: CandleAction::None,
            focus: CandleFocus::Top,
            top_state,
            btm_state: ListState::default(),
            btm_item_data: Vec::new(),
            token_pairs,
            task,
            transmitter,
        }
    
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {

        let nested_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),  // 4 options + top and bottom borders
                Constraint::Percentage(100),
            ])
            .split(area);

        let top_items: Vec<ListItem> = Self::SCREEN_OPTIONS
            .iter()
            .map(|v| ListItem::new(self.get_option_title(v)))
            .collect();

        let top_list = List::new(top_items)
            .block(
                Block::default()
                    .title(Self::SCREEN_NAME)
                    .borders(Borders::ALL)
            )
            .highlight_style(
                match self.focus {
                    CandleFocus::Top => Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .green(),
                    CandleFocus::InputMode => Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .yellow(),
                    _ => Style::default()
                }
            );
        
        frame.render_stateful_widget(
            top_list,
            nested_chunks[0],
            &mut self.top_state
        );

        self.btm_item_data = match self.step {
            
            CandleAction::Exchange => { 
                let mut exchanges: Vec<String> = Vec::new();
                for (ex, _) in &self.token_pairs {
                    exchanges.push(ex.clone());
                };
                exchanges
            },                 
            
            CandleAction::Ticker => { 
                let mut tickers: Vec<String> = Vec::new();
                let key = &self.exchange;

                if let Some(v) = self.token_pairs.get(key) {
                    for pair in v {
                        tickers.push(pair.clone());
                    };
                };

                tickers 
            },

            CandleAction::None => {
                let width: u16 = nested_chunks[0].width.saturating_sub(2);
                if let Some(n) = self.top_state.selected() { 
                    Vec::from([
                        multi_line_to_single_line(INFO_STRINGS[n], width)
                    ])
                }
                else {
                    Vec::new()
                }
            }
            
            _ => { Vec::new() } 
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

    fn get_option_title(&self, action: &CandleAction) -> String {
        
        let mut title = String::new(); 
        
        match action {
            
            CandleAction::Exchange => {
                title.push_str("Exchange");
                if self.exchange.len() > 0 {
                    title.push_str(&format!(": {}", self.exchange)) 
                };
            },

            CandleAction::Ticker => {
                title.push_str("Ticker");
                if self.ticker.len() > 0 {
                    title.push_str(&format!("  : {}", self.ticker)) 
                };
            },

            CandleAction::Period => {
                title.push_str("Period");
                
                if let CandleFocus::InputMode = self.focus {
                    if self.period.len() > 0 {
                        title.push_str(&format!("  : {}", self.period)) 
                    }
                    else {
                        title.push_str(
                            "  : Enter period (5m, 4h, 500t, etc.)"
                        )
                    };
                }
                else {
                    if self.period.len() > 0 {
                        title.push_str(&format!("  : {}", self.period)) 
                    };
                };
            },

            CandleAction::Build => { 
                title.push_str("Build") 
            },

            _ => {}

        }
                
        title
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {

        if let CandleFocus::InputMode = self.focus {
            
            if let CandleAction::Period = &self.step {
                
                match key.code {
                    KeyCode::Char(c) => {
                        self.period.push_str(&c.to_string());
                    },
                    KeyCode::Backspace => {
                        if self.period.len() > 0 {
                            let i = self.period.len().saturating_sub(1);
                            self.period = self.period[..i].to_string(); 
                        };
                    },
                    KeyCode::Enter => {
                        if period_is_valid(&self.period) {
                            self.previous_period = self.period.clone(); 
                            self.focus = CandleFocus::Top;
                            let _ = self.transmitter.send(AppEvent::Clear);
                        }
                        else {
                            let mut err_msg = String::new();
                            err_msg.push_str("Invalid period length: ");
                            err_msg.push_str(&format!(
                                "try integer + {:?}", VALID_PERIODS
                            ));
                            let _ = self.transmitter.send(AppEvent::Output(
                                OutputMsg { 
                                    text: err_msg, 
                                    color: Color::Red, 
                                    bold: true, 
                                    bg_color: None, 
                                    exchange: None, 
                                    ticker: None 
                                }
                            ));
                        };
                    },
                    KeyCode::Esc => {
                        self.period = self.previous_period.clone();
                        self.focus = CandleFocus::Top;
                        self.step = CandleAction::None;
                    }
                    _ => {}
                } 
            }
        } 
        else {
            
            match key.code {
            
                KeyCode::Up | KeyCode::Char('k') => {
                    
                    match &self.focus {

                        CandleFocus::Top => move_up(
                            &mut self.top_state, 
                            Self::SCREEN_OPTIONS.len(),
                            1
                        ),
                        
                        CandleFocus::Bottom => move_up(
                            &mut self.btm_state, 
                            self.btm_item_data.len(),
                            1
                        ),

                        _ => {}
                    
                    }
                },

                KeyCode::Down | KeyCode::Char('j') => {
                
                    match &self.focus {

                        CandleFocus::Top => move_down(
                            &mut self.top_state, 
                            Self::SCREEN_OPTIONS.len(),
                            1
                        ),
                        
                        CandleFocus::Bottom => move_down(
                            &mut self.btm_state, 
                            self.btm_item_data.len(),
                            1
                        ),

                        _ => {}
                    }
                }

                KeyCode::Enter => {
                
                    match &self.focus {

                        CandleFocus::Top => {
                            if let Some(n) = &self.top_state.selected() {
                                match n { 
                                    0 => {
                                        self.step = CandleAction::Exchange;
                                        self.focus = CandleFocus::Bottom;
                                        self.btm_state.select(Some(0));
                                    }, 
                                    1 => {
                                        if self.exchange.len() > 0 {
                                            self.step = CandleAction::Ticker;
                                            self.focus = CandleFocus::Bottom;
                                            self.btm_state.select(Some(0));
                                        }
                                        else {
                                            let msg = String::from(
                                                "Please choose an exchange"
                                            );
                                            self.transmitter.send(
                                                AppEvent::Output(
                                                    OutputMsg { 
                                                        text: msg, 
                                                        color: Color::Yellow, 
                                                        bold: false, 
                                                        bg_color: None, 
                                                        exchange: None, 
                                                        ticker: None 
                                                    }
                                                )
                                            );
                                        }
                                    }, 
                                    2 => self.step = {
                                        let msg = "Start typing..."
                                            .to_string();
                                        self.focus = CandleFocus::InputMode;
                                        self.transmitter.send(
                                            AppEvent::Output(OutputMsg { 
                                                text: msg, 
                                                color: Color::Yellow, 
                                                bold: true, 
                                                bg_color: None, 
                                                exchange: None, 
                                                ticker: None 
                                            })
                                        );
                                        CandleAction::Period
                                    }, 
                                    3 => {},
                                    _ => { return } 
                                } 
                            }
                        },
                        
                        CandleFocus::Bottom => {
                            
                            match &self.step {

                                CandleAction::Exchange => {
                                    if let Some(i) = self.btm_state
                                        .selected() 
                                    {
                                        self.exchange = self
                                            .btm_item_data[i].clone();
                                    };
                                }, 

                                CandleAction::Ticker => {
                                    if let Some(i) = self.btm_state
                                        .selected() 
                                    {
                                        self.ticker = self
                                            .btm_item_data[i].clone();

                                        let _ = self.transmitter
                                            .send(AppEvent::Clear);
                                    };
                                },

                                _ => {}
                            };
                            
                            self.focus = CandleFocus::Top;  
                            self.step = CandleAction::None;
                            self.btm_state.select(None);
                        },

                        _ => {}
                    }
                }

                KeyCode::Esc => {
                    match &self.focus {
                        CandleFocus::Top => {
                            self.top_state.select(None);
                        },
                        CandleFocus::Bottom => {
                            self.step = CandleAction::None;
                            self.btm_state.select(None);
                        },
                        CandleFocus::InputMode => { }
                    };
                    self.focus = CandleFocus::Top;
                }

                _ => {}
            
            }
   
        }
    }

    pub const SCREEN_NAME: &'static str = "Candle Builder";

    pub const SCREEN_OPTIONS: [CandleAction; 4] = [
        CandleAction::Exchange,
        CandleAction::Ticker,
        CandleAction::Period,
        CandleAction::Build,
    ];
}


