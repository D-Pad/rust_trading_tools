use std::{fmt::{self, Display, Formatter}};

use tokio::{
    sync::{
        mpsc::{
            UnboundedSender
        }
    },
};
use ratatui::{
    Frame, 
    crossterm::event::{
        KeyCode, 
        KeyEvent
    }, 
    layout::{
        Constraint, 
        Direction, 
        Layout, 
        Rect
    }, 
    style::{
        Color, 
        Modifier, 
        Style
    }, 
    widgets::{
        Block, 
        Borders, 
        List, 
        ListItem, 
        ListState, 
        Paragraph
    }
};

use crate::{
    AppEvent, 
    FieldKind, 
    FormRow, 
    OutputMsg, 
    move_down, 
    move_up, 
    strategy_form::{
        StrategyConstructor, 
        StrategyKeys
    }
};
use string_helpers::multi_line_to_single_line;
use strategies::{
    fetch_available_templates,
};


const INFO_STRINGS: [&'static str; 3] = [
    r#"Create a new strategy by choosing indicator components and entry 
    conditions."#,

    r#"Modify the input values of an existing strategy."#,

    r#"Remove any existing strategy templates. This action cannot be undone"#
];

// ------------------------------------------------------------------------- //
pub enum StrategyFocus {
    Top,
    Bottom,
}

#[derive(Clone)]
enum EditMode {
    Move,
    Input,
}

#[derive(Clone)]
enum StrategyAction {
    Create(EditMode),  // Create new or modify existing
    Modify(EditMode), 
    Delete,
    None,
}

impl StrategyAction {
    fn to_title(&self) -> &'static str {
        match self {
            StrategyAction::Create(_) => "Create New",
            StrategyAction::Modify(_) => "Modify Existing",
            StrategyAction::Delete => "Delete Existing",
            StrategyAction::None => ""
        }
    }
}

impl Display for StrategyAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_title())
    }
}


/// # Strategy Creation Screen
/// This screen is for creating and modifying trading strategies via the TUI
pub struct StrategyScreen {
    pub msg_sender: UnboundedSender<AppEvent>,
    top_state: ListState,
    btm_state: ListState,
    btm_item_data: Vec<String>,
    pub focus: StrategyFocus,
    action: StrategyAction,

    pub new_strategy: Option<StrategyConstructor>,

    focused_row: usize,
    strategy_rows: Vec<FormRow<StrategyKeys>>,
    user_input_buffer: String,
    previous_input_val: String,
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
            new_strategy: None,
            focused_row: 1,
            strategy_rows: Vec::new(),
            user_input_buffer: String::new(),
            previous_input_val: String::new(),
        } 
    }

    pub fn get_btm_item_rows(data: &[String]) -> List {
        data
            .iter()
            .map(|i| ListItem::new(i.clone()))
            .collect::<List>()
            .block(
                Block::default()
                    .title(Self::SCREEN_NAME)
                    .borders(Borders::ALL)
            )
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {

        let nested_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),  // 3 options + top and bottom borders
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
        
        let width = nested_chunks[0].width;
        let blank_vec = Vec::new();

        self.btm_item_data = match self.action {
                           
            StrategyAction::Delete |
            
            StrategyAction::Modify(_)=> {
                match fetch_available_templates() {
                    Ok(t) => t,
                    Err(_) => {
                        let _ = self.msg_sender.send(AppEvent::Output(
                            OutputMsg::new(
                                "Failed to fetch existing templates"
                                    .to_string(),
                                Color::Red,
                                true,
                                None,
                                None,
                                None,
                            )
                        ));
                        blank_vec 
                    }
                }
            },

            StrategyAction::None => {
                if let Some(i) = self.top_state.selected() {
                    Vec::from([
                        multi_line_to_single_line(
                            INFO_STRINGS[i], 
                            width
                        ),
                    ])
                }
                else { blank_vec }
            },

            _ => { blank_vec }
        };

        if let StrategyAction::Create(ref mode) = self.action {

            if let Some(strat) = &self.new_strategy {

                self.strategy_rows = strat.get_form_rows();

                let block = Block::default()
                    .title("New Strategy Creation")
                    .borders(Borders::ALL);

                frame.render_widget(block.clone(), nested_chunks[1]);

                let inner = block.inner(nested_chunks[1]);

                let form_rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(&self.strategy_rows
                        .iter()
                        .map(|_| Constraint::Length(1))
                        .collect::<Vec<Constraint>>()
                    )
                    .split(inner);

                for (i, r) in self.strategy_rows.iter().enumerate() {
                   
                    match r {

                        FormRow::SectionDivider(div) => {
                           
                            frame.render_widget(
                                Paragraph::new(
                                    format!("[{div}]"))
                                    .style(Style::default().red()),
                                form_rows[i] 
                            );
                        },

                        FormRow::InputRow(row) => {
                            let cols = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints([
                                    Constraint::Percentage(50),
                                    Constraint::Percentage(50),
                                ])
                                .split(form_rows[i]);

                            let mut text = Paragraph::new(
                                format!("  {}", row.label));
                            
                            let mut input = Paragraph::new(
                                if let FieldKind::Select(
                                    ref select
                                ) = row.kind {
                                    select.options[select.selected]
                                        .to_string() 
                                }
                                else {
                                    row.value.clone()
                                }
                            );

                            if i == self.focused_row {
                                
                                text = text.style(Style::default()
                                    .yellow()
                                    .underlined());
                              
                                if let StrategyAction::Create(
                                    EditMode::Input
                                ) = self.action {
                                    input = Paragraph::new(
                                        self.user_input_buffer.clone()
                                    );
                                };

                                input = input.style(
                                    match mode {
                                        EditMode::Move => {
                                            Style::default()
                                                .green()
                                                .underlined()
                                        },
                                        EditMode::Input => {
                                            Style::default()
                                                .add_modifier(
                                                    Modifier::REVERSED
                                                )
                                                .green()

                                        },
                                    } 
                                );
                            
                            };
                            
                            frame.render_widget(
                                text, 
                                cols[0]
                            );

                            frame.render_widget(input, cols[1]);
                        }
                    
                    }

                }

            }
        }
        else {
            
            let btm_list: List = Self::get_btm_item_rows(&self.btm_item_data);

            frame.render_stateful_widget(
                btm_list,
                nested_chunks[1],
                &mut self.btm_state
            );

        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {

        let top_len = Self::SCREEN_OPTIONS.len().saturating_sub(1);

        if let StrategyAction::Create(ref mode) = self.action {

            if let EditMode::Move = mode {

                match key.code {

                    KeyCode::Up | KeyCode::Char('k') => {
                       
                        let step: usize = {
                            
                            let min_i = 1;
                            let target = self.focused_row - 1;
                            let next_row = &self.strategy_rows[target];

                            match next_row {
                                FormRow::SectionDivider(_) => {
                                    if target > min_i { 2 }
                                    else { 0 }  // We're at the top
                                },
                                FormRow::InputRow(_) => 1
                            }
                        };

                        self.focused_row -= step;
                    }, 
                    
                    KeyCode::Down | KeyCode::Char('j') => {
                        
                        let max_i = self.strategy_rows.len() - 1;
                        let target = self.focused_row + 1;

                        if target <= max_i {
                        
                            let next_row = &self.strategy_rows[target];

                            let step = match next_row {
                                FormRow::SectionDivider(_) => {
                                    2 
                                },
                                FormRow::InputRow(_) => {
                                    1
                                }
                            };

                            self.focused_row += step;
                        
                        };
                    },

                    KeyCode::Enter => {

                        let i = self.focused_row;
                        let active_row = &self.strategy_rows[i];
                        
                        if let FormRow::InputRow(row) = active_row {
                            
                            if let Some(ref mut strat) = self.new_strategy {
                                let _ = strat.modify_from_form_field(row);
                            };
                            
                            match row.kind {
                                
                                FieldKind::Float |
                                FieldKind::Integer |
                                FieldKind::Text => {

                                    self.action = StrategyAction::Create(
                                        EditMode::Input);
                                    
                                    self.previous_input_val = row
                                        .value
                                        .clone();

                                    self.user_input_buffer = row
                                        .value
                                        .clone();

                                },
                                
                                FieldKind::Select(ref opts) => {
                                    println!("OPTS: {:?}", opts.options);
                                },
                                
                                _ => {}
                            }
                        } 
                    }

                    KeyCode::Esc => {

                        if let Some(ref strat) = self.new_strategy {
                           
                            let mut msg = String::new();
                            let mut col = Color::Green;

                            if let Ok(_) = strat.strategy.export() {
                                msg.push_str("Strategy template saved.");
                            }
                            else {
                                msg.push_str("Failed to save template");
                                col = Color::Red;
                            }

                            let _ = self.msg_sender.send(AppEvent::Output(
                                OutputMsg::new( 
                                    msg, 
                                    col, 
                                    false, 
                                    None, 
                                    None, 
                                    None 
                                )
                            ));

                        }; 

                    }

                    _ => {}
                }
            }
            
            // If we're in "create mode" and also trying to 
            // modify an input value
            else if let EditMode::Input = mode {

                let i = self.focused_row;
                let active_row = &mut self.strategy_rows[i];

                match key.code {

                    KeyCode::Char(c) => {

                        if let FormRow::InputRow(_) = active_row {
                            self.user_input_buffer.push_str(&c.to_string());  
                        }

                    },

                    KeyCode::Esc => {
                        
                        self.action = StrategyAction::Create(
                            EditMode::Move
                        );

                        if let FormRow::InputRow(row) = active_row {
                            row.value = self.previous_input_val.clone(); 
                        }

                        let _ = self.msg_sender.send(
                            AppEvent::Clear
                        );

                    },

                    KeyCode::Enter => {

                        if let FormRow::InputRow(row) = active_row {
                            
                            row.value = self.user_input_buffer.clone(); 
                            
                            if let Some(ref mut strat) = self.new_strategy {
                                
                                let r = strat.modify_from_form_field(row);
                                
                                match r {
                                    Ok(_) => {
                                        self.user_input_buffer = String::new();
                                        self.action = StrategyAction::Create(
                                            EditMode::Move
                                        );
                                        let _ = self.msg_sender.send(
                                            AppEvent::Clear
                                        );
                                    },
                                    Err(_) => {
                                        
                                        let mut err_msg = format!(
                                            "ERROR: Invalid input value: {}",
                                            row.value
                                        );

                                        err_msg.push_str(&format!(
                                            " | Expected: {}",
                                            row.kind.to_str()
                                        ));

                                        let _ = self.msg_sender.send(
                                            AppEvent::Output(
                                                OutputMsg::new(
                                                    err_msg,
                                                    Color::Red,
                                                    true,
                                                    None,
                                                    None,
                                                    None
                                                )
                                            )
                                        );
                                    } 
                                };
                            };     
                        }

                    },

                    KeyCode::Backspace => {

                        let l = self.user_input_buffer.len();
                        
                        if l > 1 {
                            self.user_input_buffer = self
                                .user_input_buffer[..l - 1].to_string(); 
                        }
                        else if l == 1 {
                            self.user_input_buffer = String::new();
                        } 
                    }
                    
                    _ => {}

                }

            }
        }
        
        else {
            
            match key.code {
            
                KeyCode::Up | KeyCode::Char('k') => {
                    
                    match &self.focus {

                        StrategyFocus::Top => move_up(
                            &mut self.top_state, 
                            top_len, 
                            1
                        ),
                        
                        StrategyFocus::Bottom => move_up(
                            &mut self.btm_state, 
                            self.btm_item_data.len(),
                            1
                        ),
                    
                    }
                },

                KeyCode::Down | KeyCode::Char('j') => {
                
                    match &self.focus {

                        StrategyFocus::Top => move_down(
                            &mut self.top_state, 
                            top_len, 
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

                    match &self.focus {

                        StrategyFocus::Top => {
                            
                            self.focus = StrategyFocus::Bottom;
                            
                            self.action = match &self.top_state.selected() {
                                
                                Some(0) => {

                                    let mut strat = Some(
                                        StrategyConstructor::new()
                                    );
                                    self.new_strategy = strat;
                                    Self::SCREEN_OPTIONS[0].clone()
                                
                                }, 
                                Some(1) => Self::SCREEN_OPTIONS[1].clone(), 
                                None | _ => StrategyAction::None,
                            
                            };

                            self.btm_state.select(Some(0));
                        },

                        StrategyFocus::Bottom => {
                            
                        }

                    };
                   
                    let files = match fetch_available_templates()
                    {
                        Ok(d) => d,
                        Err(_) => return
                    };
                    let _ = self.msg_sender.send(AppEvent::Output(
                        OutputMsg::new(
                            format!("{:?}", files),
                            Color::Red,
                            false,
                            None,
                            None,
                            None
                        )
                    ));


                }

                KeyCode::Esc => {
                    self.focus = StrategyFocus::Top;
                }

                _ => {}
            }
        }
    }

    pub const SCREEN_NAME: &'static str = "Strategy Manager";

    const SCREEN_OPTIONS: [StrategyAction; 3] = [
        StrategyAction::Create(EditMode::Move),
        StrategyAction::Modify(EditMode::Move),
        StrategyAction::Delete,
    ];

}


