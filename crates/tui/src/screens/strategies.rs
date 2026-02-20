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
        Paragraph,
    },
    style::{
        Style,
        Modifier,
        Color,
    },
    crossterm::{
        event::{
            KeyEvent,
            KeyCode,
        },
    },
};

use crate::{
    AppEvent, 
    OutputMsg, 
    move_up, 
    move_down,
    strategy_form::NewStrategyConstructor,
};
use string_helpers::multi_line_to_single_line;
use strategies::{
    StrategyInputs, 
    load_strategy_template,
    export_strategy_template,
    fetch_available_templates,
    indicators::IndicatorTypes,
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
enum CreationAction {
    IndicatorSelect,
    ParameterSelect,
}

#[derive(Clone)]
enum StrategyAction {
    CreateNew(CreationAction),
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
    btm_option_state: ListState,
    btm_item_data: Vec<String>,
    pub focus: StrategyFocus,
    action: StrategyAction,

    new_strategy: Option<NewStrategyConstructor>,

    // Strategy Creation values
    indicator_choices: Vec<String>,
}

impl StrategyScreen {

    pub fn new(
        msg_sender: UnboundedSender<AppEvent>
    ) -> Self {
        
        let mut top_state = ListState::default();
        top_state.select(Some(0));

        let indicator_choices: Vec<String> = IndicatorTypes::list()
            .iter()
            .map(|v| v.to_string())
            .collect();
        // let indicator_choices: Vec<ListItem<'a>> = IndicatorTypes::list()
        //     .iter()
        //     .map(|v| ListItem::new(*v))
        //     .collect();

        StrategyScreen {
            msg_sender,
            top_state,
            btm_state: ListState::default(),
            btm_option_state: ListState::default(),
            btm_item_data: Vec::new(),
            focus: StrategyFocus::Top,
            action: StrategyAction::None,
            new_strategy: None,
            indicator_choices,
        } 
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
        
        let mut width = nested_chunks[0].width;
        let blank_vec: Vec<String> = Vec::new();  // Temporary
        
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

            StrategyAction::CreateNew(_) => { 
                self.indicator_choices.clone()
            }
        };

        if let StrategyAction::CreateNew(c_action) = &self.action {

            let block = Block::default()
                .title("Create New Strategy")
                .borders(Borders::ALL);

            frame.render_widget(block.clone(), nested_chunks[1]);

            let inner = block.inner(nested_chunks[1]);

            let creation_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(25),
                    Constraint::Percentage(100),
                ])
                .split(inner);
         
            let btm_list = List::new(self.btm_item_data.clone()) 
                .block(
                    Block::default()
                        .title("Indicator Selection")
                        .borders(Borders::ALL)
                )
                .highlight_style(
                    if let StrategyFocus::Bottom = self.focus {
                        let mut style = Style::default()
                            .add_modifier(Modifier::REVERSED);
                        match c_action {
                            CreationAction::IndicatorSelect => {
                                style = style.green();
                            },
                            CreationAction::ParameterSelect => {
                                style = style.yellow();
                            }
                        };
                        style
                    } else {
                        Style::default()
                    }
                );
 
            frame.render_stateful_widget(
                btm_list, 
                creation_chunks[0],
                &mut self.btm_state
            );
           
            width = creation_chunks[1].width;
            let options: Vec<ListItem> = self.get_indicator_options(width);
            let option_data = List::new(options)
                .block(
                    Block::default()
                        .title("Indicator Parameters")
                        .borders(Borders::ALL)
                );
            
            frame.render_widget(option_data, creation_chunks[1]);

        }
        else {

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
    }

    fn get_indicator_options(&self, width: u16) -> Vec<ListItem> {
        let mut options = Vec::from([
                ListItem::new(multi_line_to_single_line(
                    "When selected, indicator options are shown here",
                    width
                ))
            ]); 
        
        if let StrategyAction::CreateNew(c_action) = &self.action { 
            if let CreationAction::ParameterSelect = c_action {

            }
        };
        
        options
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {

        let top_len = Self::SCREEN_OPTIONS.len().saturating_sub(1);

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

                                let mut strat = NewStrategyConstructor::new();
                                self.new_strategy = Some(strat);
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

    pub const SCREEN_NAME: &'static str = "Strategy Manager";

    const SCREEN_OPTIONS: [StrategyAction; 3] = [
        StrategyAction::CreateNew(CreationAction::IndicatorSelect),
        StrategyAction::ModifyExisting,
        StrategyAction::Delete,
    ];

}


