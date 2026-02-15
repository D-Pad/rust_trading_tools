use std::fmt::{self, Formatter, Display};

use tokio::{
    sync::{
        mpsc::{
            UnboundedSender
        }
    },
};
use ratatui::{
    Frame,
    layout::{
        Rect,
        Layout,
        Direction,
        Constraint,
    },
    widgets::{
        Block,
        Borders,
        List,
        ListState,
        ListItem,
    },
    style::{
        Style,
        Modifier,
    },
    crossterm::{
        event::{
            KeyEvent,
            KeyCode,
        },
    },
};

use crate::{AppEvent, move_up, move_down};


pub enum StrategyFocus {
    Top,
    Bottom,
}

enum StrategyAction {
    CreateNew,
    ModifyExisting,
    None,
}

impl StrategyAction {
    fn to_title(&self) -> &'static str {
        match self {
            StrategyAction::CreateNew => "Create New",
            StrategyAction::ModifyExisting => "Modify Existing",
            StrategyAction::None => ""
        }
    }
}

impl Display for StrategyAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_title())
    }
}


pub struct StrategyScreen {
    pub msg_sender: UnboundedSender<AppEvent>,
    top_state: ListState,
    btm_state: ListState,
    btm_item_data: Vec<String>,
    pub focus: StrategyFocus,
    action: StrategyAction
}

impl StrategyScreen {

    pub fn new(
        msg_sender: UnboundedSender<AppEvent>
    ) -> Self {
        
        let mut top_state = ListState::default();
        top_state.select(Some(0));

        StrategyScreen {
            msg_sender,
            top_state,
            btm_state: ListState::default(),
            btm_item_data: Vec::new(),
            focus: StrategyFocus::Top,
            action: StrategyAction::None,
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
            .map(|v| ListItem::new(v.to_title()))
            .collect();

        let top_list = List::new(top_items)
            .block(
                Block::default()
                    .title(Self::SCREEN_NAME)
                    .borders(Borders::ALL)
            )
            .highlight_style(
                match self.focus {
                    StrategyFocus::Top => Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .green(),
                    _ => Style::default()
                }
            );
        
        frame.render_stateful_widget(
            top_list,
            nested_chunks[0],
            &mut self.top_state
        );

        self.btm_item_data = match self.action {
            
            StrategyAction::CreateNew => { 
                Vec::new() 
            },                 
            
            StrategyAction::ModifyExisting => { 
                Vec::new() 
            },
           
            _ => { Vec::new() } 
        };

        let btm_items: Vec<ListItem> = self.btm_item_data.iter()
            .map(|v| ListItem::new(&v[..]))
            .collect();

        let btm_list = List::new(btm_items)
            .block(
                Block::default()
                    // .title(self.focus.title())
                    .borders(Borders::ALL)
            )
            .highlight_style(
                if let StrategyFocus::Bottom = self.focus {
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

    pub async fn handle_key(&mut self, key: KeyEvent) {

        match key.code {
        
            KeyCode::Up | KeyCode::Char('k') => {
                
                match &self.focus {

                    StrategyFocus::Top => move_up(
                        &mut self.top_state, 
                        Self::SCREEN_OPTIONS.len(),
                        1
                    ),
                    
                    StrategyFocus::Bottom => move_up(
                        &mut self.btm_state, 
                        self.btm_item_data.len(),
                        1
                    ),

                    _ => {}
                
                }
            },

            KeyCode::Down | KeyCode::Char('j') => {
            
                match &self.focus {

                    StrategyFocus::Top => move_down(
                        &mut self.top_state, 
                        Self::SCREEN_OPTIONS.len(),
                        1
                    ),
                    
                    StrategyFocus::Bottom => move_down(
                        &mut self.btm_state, 
                        self.btm_item_data.len(),
                        1
                    )
                }
            }

            KeyCode::Enter => {

            }

            KeyCode::Esc => {

            }

            _ => {}
        }
    }

    pub const SCREEN_NAME: &'static str = "Strategy Manager";

    pub const SCREEN_OPTIONS: [StrategyAction; 2] = [
        StrategyAction::CreateNew,
        StrategyAction::ModifyExisting,
    ];

}


