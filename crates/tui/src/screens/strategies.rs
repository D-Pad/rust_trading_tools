use std::fmt::{self, Formatter, Display};

use tokio::{
    sync::{
        mpsc::{
            UnboundedSender
        }
    },
};
use ratatui::{
    Frame, crossterm::event::{
            KeyCode, KeyEvent
        }, layout::{
        Constraint, Direction, Layout, Rect
    }, style::{
        Color, Modifier, Style
    }, widgets::{
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
enum CreateMode {
    Move,
    Input,
}

#[derive(Clone)]
enum StrategyAction {
    CreateNew(CreateMode),
    ModifyExisting,
    Delete,
    None,
}

impl StrategyAction {
    fn to_title(&self) -> &'static str {
        match self {
            StrategyAction::CreateNew(_) => "Create New",
            StrategyAction::ModifyExisting => "Modify Existing",
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
                           
            StrategyAction::ModifyExisting => {
                blank_vec 
            },
          
            StrategyAction::Delete => {
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

        if let StrategyAction::CreateNew(_) = self.action {

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
                            
                            if i == self.focused_row {
                                text = text.style(Style::default()
                                    .yellow()
                                    .underlined());     
                            };

                            frame.render_widget(
                                text, 
                                cols[0]
                            );

                            let input = Paragraph::new(
                                if let FieldKind::Select(
                                    ref select
                                ) = row.kind {
                                    select.options[select.selected].to_string() 
                                }
                                else {
                                    row.value.clone()
                                }
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

        if let StrategyAction::CreateNew(ref mode) = self.action {

            if let CreateMode::Move = mode {

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
                                strat.modify_from_form_field(row);
                            }; 
                        }
                    }

                    _ => {}
                }
            }
            else if let CreateMode::Input = mode {
                
                match key.code {

                    KeyCode::Char(c) => {

                        let i = self.focused_row;
                        let active_row = &self.strategy_rows[i];

                        if let FormRow::InputRow(row) = active_row {
                            // row.value; 
                        }
                    },

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
                                Some(2) => Self::SCREEN_OPTIONS[2].clone(),
                                None | _ => StrategyAction::None,
                            
                            };

                            self.btm_state.select(Some(0));
                        },

                        StrategyFocus::Bottom => {
                            
                        }

                    };

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
        StrategyAction::CreateNew(CreateMode::Move),
        StrategyAction::ModifyExisting,
        StrategyAction::Delete,
    ];

}


