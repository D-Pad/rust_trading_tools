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
    StrategyError, 
    delete_strategy, 
    fetch_available_templates, 
    load_strategy_template
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
    Select, 
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
enum Confirm {
    None,
    Deleting,
    AbortCreation,
}

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
    
    confirming: Confirm,
    width: u16,
    existing_strategies: Vec<String>
}

impl StrategyScreen {

    pub fn new(
        msg_sender: UnboundedSender<AppEvent>
    ) -> Self {
        
        let mut top_state = ListState::default();
        top_state.select(Some(0));

        let mut screen = StrategyScreen {
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
            confirming: Confirm::None,
            width: 30,
            existing_strategies: Vec::new()
        };

        screen.set_btm_item_data();
        screen.set_strategy_template_names();
        screen
    }

    pub fn get_btm_item_rows(data: &[String]) -> List<'_> {
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
      
        let width: u16 = nested_chunks[0].width;
        if width != self.width {
            self.width = width;
        };
        
        self.set_btm_item_data(); 

        match self.action { 
            
            StrategyAction::Create(_) |
            StrategyAction::Modify(EditMode::Move) |
            StrategyAction::Modify(EditMode::Input) =>  {

                self.render_template_rows(
                    frame, 
                    nested_chunks[1],
                );

            }
           
            _ => {
                
                let btm_list: List = Self::get_btm_item_rows(
                    &self.btm_item_data)
                    .block(
                        Block::default()
                            .title(Self::SCREEN_NAME)
                            .borders(Borders::ALL)
                    )
                    .highlight_style(
                        
                        match self.focus {
                            
                            StrategyFocus::Bottom => {
                                
                                let style = Style::default()
                                    .add_modifier(Modifier::REVERSED);
                                
                                if let Confirm::Deleting = self.confirming {
                                    style.yellow()
                                }
                                else {
                                    style.green() 
                                }
                            
                            },
                            
                            _ => Style::default()
                        }
                    );

                frame.render_stateful_widget(
                    btm_list,
                    nested_chunks[1],
                    &mut self.btm_state
                );
            }
        }
    }

    pub async fn handle_other(&mut self, key: KeyEvent) {
        
        let top_len = Self::SCREEN_OPTIONS.len().saturating_sub(1);
        
        match key.code {
        
            KeyCode::Up | KeyCode::Char('k') => {
              
                if let Confirm::Deleting = self.confirming { return };

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
                
                if let Confirm::Deleting = self.confirming { return };
            
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
            },

            KeyCode::Enter => {

                match &self.focus {

                    StrategyFocus::Top => {
                       
                        self.focus = StrategyFocus::Bottom;
                        
                        self.action = match &self.top_state.selected() {
                           
                            // Create mode
                            Some(0) => {

                                let strat = Some(
                                    StrategyConstructor::new()
                                );

                                self.focused_row = 1;
                                self.new_strategy = strat;
                                Self::SCREEN_OPTIONS[0].clone()
                            
                            },

                            // Modify mode 
                            Some(1) => {
                                self.focused_row = 1; 
                                Self::SCREEN_OPTIONS[1].clone()
                            }, 
                            
                            // Delete mode
                            Some(2) => {

                                self.set_strategy_template_names();
                                if self.existing_strategies.len() == 0 {

                                    self.focus = StrategyFocus::Top;
                                    
                                    let msg = AppEvent::Output(
                                        OutputMsg::new(
                                            String::from(
                                                "No strategies exist yet"
                                            ),
                                            Color::Red,
                                            true,
                                            None,
                                            None,
                                            None
                                        )
                                    );
                                    
                                    let _ = self.msg_sender.send(msg);
                                    StrategyAction::None 
                                
                                }
                                else {
                                    Self::SCREEN_OPTIONS[2].clone()
                                }
                            }, 
                            
                            // For making the compiler happy
                            None | _ => StrategyAction::None,
                        
                        };

                        self.btm_state.select(Some(0));
                    },

                    StrategyFocus::Bottom => {
        
                        let strat_name: String;
                        if let Some(i) = self.btm_state.selected() {
                            strat_name = self.btm_item_data[i].clone();
                        }
                        else { return }
                        
                        match self.action {

                            StrategyAction::Delete => {

                                if let Confirm::None = self.confirming {
                                    self.confirming = Confirm::Deleting;

                                    let _ = self.msg_sender.send(
                                        AppEvent::Output(
                                            OutputMsg::new(
                                                format!(
                                                    "Delete {}? (y/n)",
                                                    strat_name 
                                                ),
                                                Color::Yellow,
                                                false,
                                                None,
                                                None,
                                                None,
                                            )
                                        )
                                    );
                                };

                            },
                            
                            _ => {}

                        }

                    }

                };
               
            },

            KeyCode::Esc => {
                match self.confirming {
                    Confirm::None |
                    Confirm::AbortCreation => {
                        self.focus = StrategyFocus::Top;
                    },
                    _ => {} 
                }
            }

            KeyCode::Char('y') => {

                if let Confirm::Deleting = self.confirming {

                    let strat_name: String;
                    if let Some(i) = self.btm_state.selected() {
                        strat_name = self.btm_item_data[i].clone();
                    }
                    else { return }

                    let col: Color;
                    let msg = match delete_strategy(
                        &strat_name
                    )
                    {
                        Ok(_) => {
                            col = Color::Green;
                            let _ = self.msg_sender.send(
                                AppEvent::Clear
                            );
                            format!(
                                "Deleted {}", 
                                strat_name
                            )
                        },
                        Err(_) => {
                            col = Color::Red;
                            format!(
                                "Failed to delete {}", 
                                strat_name
                            )
                        }

                    };
                    
                    let _ = self.msg_sender.send(
                        AppEvent::Output(
                            OutputMsg::new(
                                msg,
                                col,
                                true,
                                None,
                                None,
                                None,
                            )
                        )
                    );

                    self.confirming = Confirm::None;
                    self.set_strategy_template_names();
                    if self.existing_strategies.len() == 0 {
                        self.focus = StrategyFocus::Top;
                    }
                }
            }

            _ => {
                self.confirming = Confirm::None;
                let _ = self.msg_sender.send(AppEvent::Clear);
            }
        }
    }

    pub async fn handle_edit_mode_input(&mut self, key: KeyEvent) {
        
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

    pub async fn handle_edit_mode_move(&mut self, key: KeyEvent) {
        
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

            KeyCode::Left | KeyCode::Char('h') => {
                let row = match self.strategy_rows.get_mut(self.focused_row) {
                    Some(r) => r,
                    None => return
                }; 
               
                if let FormRow::InputRow(r) = row {
                    
                    if let FieldKind::Select(_) = r.kind {
                       
                        r.rotate_selection_left();
                        
                        if let Some(strat) = &mut self.new_strategy {
                            if let Err(_) = strat.modify_from_form_field(r) {
                                let _ = self.msg_sender.send(AppEvent::Output(
                                    OutputMsg::new(
                                        "Failed to modify row".to_string(),
                                        Color::Red,
                                        true,
                                        None,
                                        None,
                                        None
                                    )
                                )); 
                            };
                        }
                    }
                }
            }

            KeyCode::Right | KeyCode::Char('l') => {
                let row = match self.strategy_rows.get_mut(self.focused_row) {
                    Some(r) => r,
                    None => return
                }; 
                
                if let FormRow::InputRow(r) = row {
                    
                    if let FieldKind::Select(_) = r.kind {
                        
                        r.rotate_selection_right();
                        
                        if let Some(strat) = &mut self.new_strategy {
                            if let Err(_) = strat.modify_from_form_field(r) {
                                let _ = self.msg_sender.send(AppEvent::Output(
                                    OutputMsg::new(
                                        "Failed to modify row".to_string(),
                                        Color::Red,
                                        true,
                                        None,
                                        None,
                                        None
                                    )
                                )); 
                            };
                        }
                    }
                }
            }

            KeyCode::Enter => {

                let i = self.focused_row;
                let active_row = &self.strategy_rows[i];
                
                if let FormRow::InputRow(row) = active_row {
                    
                    if let Some(ref mut strat) = 
                        self.new_strategy {
                        let _ = strat.modify_from_form_field(
                            row
                        );
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
                        
                        _ => {}
                    }
                } 
            }

            KeyCode::Esc => {

                if let Some(ref strat) = self.new_strategy {
                   
                    let mut msg = String::new();
                    let mut col = Color::Green;
                    
                    let mut modifying: bool = false;
                    if let StrategyAction::Modify(_) = self.action {
                        modifying = true;
                    };

                    match strat.strategy.export(modifying) {
                        
                        Ok(_) => {
                            msg.push_str(
                                "Strategy template saved."
                            );
                            self.focus = StrategyFocus::Top;
                            self.action = StrategyAction::None;
                        },
                        
                        Err(f) => { 
                            
                            if let Confirm::AbortCreation = 
                                self.confirming {
                                
                                let abort_msg = 
                                "Strategy creation aborted" 
                                    .to_string();
                                
                                self.focus = StrategyFocus::Top;
                                self.action = StrategyAction::None;
                                
                                let _ = self.msg_sender.send(
                                    AppEvent::Clear);
                                
                                let _ = self.msg_sender.send(
                                    AppEvent::Output(
                                    OutputMsg::new(
                                        abort_msg, 
                                        Color::Yellow,
                                        true,
                                        None,
                                        None,
                                        None
                                    )
                                ));
                                self.confirming = Confirm::None;
                                return 
                            };

                            msg.push_str("Failed to save template");
                            if let StrategyError::ExportFailed(e) = f { 
                                msg.push_str(&format!(": {}", e));
                            }
                            msg.push_str(
                                ". Press 'Esc' again to abort"
                            );
                            col = Color::Red;
                            self.confirming = 
                                Confirm::AbortCreation;
                        }
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

    pub async fn handle_key(&mut self, key: KeyEvent) {

        match self.action {

            StrategyAction::Create(ref mode) | 
            StrategyAction::Modify(ref mode) => {

                match mode {
                    
                    EditMode::Move => {
                        self.handle_edit_mode_move(key).await;
                    },
                    
                    // If we're in "create mode" and also trying to 
                    // modify an input value
                    EditMode::Input => {
                        self.handle_edit_mode_input(key).await;
                    },
                    
                    EditMode::Select => {
                        match key.code {

                            KeyCode::Char('j') | KeyCode::Down => {
                                move_down(
                                    &mut self.btm_state,
                                    self.btm_item_data.len(),
                                    1
                                );
                            },

                            KeyCode::Char('k') | KeyCode::Up => {
                                move_up(
                                    &mut self.btm_state,
                                    self.btm_item_data.len(),
                                    1
                                );
                            },

                            KeyCode::Enter => {
                           
                                let i = match self.btm_state.selected() {
                                    Some(x) => x,
                                    None => return
                                };

                                let name = match self.btm_item_data.get(i) {
                                    Some(x) => x.clone(),
                                    None => return
                                };
                                
                                let strategy = match load_strategy_template(
                                    &name) 
                                {
                                    Ok(i) => i,
                                    Err(_) => return
                                };
 
                                let strat_constructor = StrategyConstructor {
                                    strategy
                                };

                                self.new_strategy = Some(strat_constructor);
                                self.action = StrategyAction::Modify(
                                    EditMode::Move
                                );

                            },

                            KeyCode::Esc => {

                                self.action = StrategyAction::None;
                                self.focus = StrategyFocus::Top;

                            }

                            _ => {}

                        }    
                    }
                }
                
            },
            StrategyAction::Delete |
            StrategyAction::None => {
                self.handle_other(key).await;
            } 
        }
    }

    fn set_btm_item_data(&mut self) {

        let blank_vec = Vec::new();

        self.btm_item_data = match self.action {
                           
            StrategyAction::Delete |
            
            StrategyAction::Modify(EditMode::Select) => {
                self.set_strategy_template_names();
                if self.existing_strategies.len() > 0 { 
                    self.existing_strategies.clone()
                }
                else {
                    blank_vec
                }
            },

            StrategyAction::None => {
                if let Some(i) = self.top_state.selected() {
                    Vec::from([
                        multi_line_to_single_line(
                            INFO_STRINGS[i], 
                            self.width
                        ),
                    ])
                }
                else { 
                    blank_vec
                }
            },

            _ => { blank_vec }
        };

    }

    fn render_template_rows(
        &mut self, 
        frame: &mut Frame, 
        rect: Rect,
    ) {

        let strat = match &self.new_strategy {
            Some(s) => s,
            None => return
        };

        let mut is_new_strat: bool = true;
        let mode = match &self.action {
            StrategyAction::Create(m) => m,
            StrategyAction::Modify(m) => {
                is_new_strat = false;
                m
            },
            _ => { return }
        };

        self.strategy_rows = strat.get_form_rows(is_new_strat);

        let block = Block::default()
            .title("New Strategy Creation")
            .borders(Borders::ALL);

        frame.render_widget(block.clone(), rect);

        let inner = block.inner(rect);

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

                            let opt = select.options[select.selected]
                                .to_string();

                            if i == self.focused_row {
                                format!("◀ {} ▶", opt)
                            }
                            else {  
                                opt
                            }
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
                                _ => {
                                    // Not possible
                                    Style::default() 
                                }
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

    /// Fetches available strategy templates from ~/.config/dtrade/strategies
    fn set_strategy_template_names(&mut self) {

        let blank_vec = Vec::new();

        self.existing_strategies = match fetch_available_templates() {
            Ok(t) => {
                t
            },
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

    }

    // fn test_msg(&self, msg: &str) {

    //     let _ = self.msg_sender.send(AppEvent::Output(
    //         OutputMsg::new(
    //             msg.to_string(),
    //             Color::Red,
    //             true,
    //             Some(Color::Yellow),
    //             None,
    //             None
    //         )
    //     ));

    // }

    pub const SCREEN_NAME: &'static str = "Strategy Manager";

    const SCREEN_OPTIONS: [StrategyAction; 3] = [
        StrategyAction::Create(EditMode::Move),
        StrategyAction::Modify(EditMode::Select),
        StrategyAction::Delete,
    ];

}


