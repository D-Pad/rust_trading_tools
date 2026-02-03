use std::{collections::BTreeMap, io::{self}};

use sqlx::PgPool;
use tokio::{sync::mpsc::Sender};
use ratatui::{
    Frame, 
    Terminal, 
    backend::CrosstermBackend, 
    crossterm::{
        event::{
            self, 
            Event, 
            KeyCode, 
            KeyEvent
        }, 
        execute,
        terminal::{
            EnterAlternateScreen, 
            LeaveAlternateScreen, 
            disable_raw_mode, 
            enable_raw_mode
        }
    }, 
    layout::{
        Constraint,
        Direction,
        Layout
    }, 
    style::{
        Modifier,
        Style
    }, 
    widgets::{
        Block,
        Borders,
        List,
        ListItem,
        ListState
    }
};

use app_core::{
    AppEvent, 
    database_ops::{
        fetch_exchanges_and_pairs_from_db
    }
};


enum Focus {
    Operations,
    Top,
    Bottom
}

// ------------ SCREENS ------------- //
enum Screen {
    DatabaseManager(DatabaseScreen),
    CandleBuilder(CandleScreen),
    SystemSettings(SettingsScreen)
}

// ------------ DATABASE SCREEN -------------- //
struct DatabaseScreen {
    focus: DbFocus,
    top_state: ListState,
    btm_state: ListState,
    selected_action: Option<DbAction>
}

impl DatabaseScreen {
    fn handle_key(
        &mut self,
        key: KeyEvent,
        options: &BTreeMap<&'static str, Vec<&'static str>>,
        selected_op: &str,
    ) {
        match key.code {
            KeyCode::Up => match self.focus {
                DbFocus::Top => {
                    let len = options.get(selected_op)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    
                    move_up(&mut self.top_state, len);
                }
                DbFocus::Bottom => {
                    // later
                }
            },

            KeyCode::Down => match self.focus {
                DbFocus::Top => {
                    let len = options.get(selected_op)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    
                    move_down(&mut self.top_state, len);
                }
                DbFocus::Bottom => {
                    // later
                }
            },

            KeyCode::Enter => match self.focus {
                DbFocus::Top => {
                    if let Some(i) = self.top_state.selected() {
                        self.selected_action = options
                            .get(selected_op)
                            .and_then(|v| v.get(i))
                            .and_then(|s| match *s {
                                "Add new pairs" => Some(DbAction::AddPairs),
                                "Remove pairs" => Some(DbAction::RemovePairs),
                                "Update data" => Some(DbAction::UpdateData),
                                _ => None,
                            });
                    }
                    self.focus = DbFocus::Bottom;
                    self.btm_state.select(Some(0));
                }

                DbFocus::Bottom => {
                    // execute action
                }
            },

            KeyCode::Esc => match self.focus {
                DbFocus::Bottom => {
                    self.focus = DbFocus::Top;
                    self.selected_action = None;
                }
                DbFocus::Top => {}
            },

            _ => {}
        }
    }
}

enum DbFocus {
    Top,
    Bottom
}

enum DbAction {
    AddPairs,
    RemovePairs,
    UpdateData
}


// -------------- CANDLE SCREEN ------------- //
struct CandleScreen {
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


// ------------- SYSTEM SETTINGS -------------- //
struct SettingsScreen {
    settings_state: ListState,
    dirty: bool,
}


// ---------------------------- TERMINAL INTERFACE ------------------------- //
pub struct TerminalInterface {
    transmitter: Sender<AppEvent>,
    db_pool: PgPool,
    options: BTreeMap<&'static str, Vec<&'static str>>,
    operation_state: ListState,
    screen: Screen,
    selected_op: &str,
}

impl TerminalInterface {
    
    pub fn new(transmitter: Sender<AppEvent>, db_pool: PgPool) -> Self {
        let options = BTreeMap::from([
            (
                "Database Manager", 
                vec![
                    "Add new pairs", 
                    "Remove pairs", 
                    "Update data"
                ]
            ),
            (
                "Candle Builder",
                vec![]
            ),
            (
                "System Settings",
                vec![]
            )
        ]);

        let mut operation_state = ListState::default();
        operation_state.select(Some(0));
      
        let mut selected_op = self.operations() 
            .get(0) 
            .cloned() 
            .ok_or_else(|| { 
                io::Error::new(
                    io::ErrorKind::Other, "No operations available") 
            })?;

        let db_screen = DatabaseScreen {
            focus: DbFocus::Top,
            top_state: ListState::default(),
            btm_state: ListState::default(),
            selected_action: None 
        };

        TerminalInterface { 
            transmitter, 
            db_pool, 
            options,
            operation_state,
            screen: Screen::DatabaseManager(db_screen)
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        
    }

    fn key_handle(&mut self, key: KeyEvent) {

        let operations = self.operations();
        match key.code {
            KeyCode::Char('q') => break,
           
            KeyCode::Down => match focus {
                Focus::Operations => {
                    let i = match self.operation_state.selected() {
                        Some(i) if i + 1 < operations.len() => i + 1,
                        Some(i) => i,
                        None => 0,
                    };
                    self.operation_state.select(Some(i));
                    selected_op = operations[i];
                },
            
                Focus::Top => {
                    let len = self.options.get(selected_op)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    
                    let i = match top_state.selected() {
                        Some(i) if i + 1 < len => i + 1,
                        Some(i) => i,
                        None => 0,
                    };
                    top_state.select(Some(i));
                },
            
                Focus::Bottom => {
                    // later
                }
            },

            KeyCode::Up => match focus {
                Focus::Operations => {
                    let i = match self.operation_state.selected() {
                        Some(i) if i > 0 => i - 1,
                        Some(i) => i,
                        None => 0,
                    };
                    self.operation_state.select(Some(i));
                    selected_op = operations[i];
                },
            
                Focus::Top => {
                    let len = self.options.get(selected_op)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    
                    let i = match top_state.selected() {
                        Some(i) if i > 0 => i - 1,
                        Some(i) => i,
                        None => 0,
                    };
                    top_state.select(Some(i));
                },
            
                Focus::Bottom => {
                    // later
                }                   
            },

            KeyCode::Enter => {
                match focus {
                    Focus::Operations => {
                        focus = Focus::Top;
                        top_state.select(Some(0));
                        self.selected_action = None;
                    }

                    Focus::Top => {
                        if let Some(i) = top_state.selected() { 
                            self.selected_action = self.options 
                                .get(selected_op)
                                .and_then(|v| v.get(i))
                                .copied();
                        };

                        focus = Focus::Bottom;
                        btm_state.select(Some(0));
                    }

                    Focus::Bottom => {
                        // Final selection action
                        // Example: send event
                    }
                };
                if let Some(i) = self.operation_state.selected() {
                    selected_op = operations[i];
                };
            },

            KeyCode::Esc => match focus {
                Focus::Operations => {},
                Focus::Top => {
                    focus = Focus::Operations
                },
                Focus::Bottom => {
                    focus = Focus::Top;
                    self.selected_action = None;
                }
            },

            _ => {}
        }
    } 

    fn operations(&self) -> Vec<&str> {
        self.options.keys().cloned().collect()
    } 

    pub async fn run(&mut self) 
        -> io::Result<()> {

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?; 
 
        let mut top_state = ListState::default();
        let mut btm_state = ListState::default();

        let mut focus = Focus::Operations;

        let exchanges_and_tables = fetch_exchanges_and_pairs_from_db(
            self.db_pool.clone()
        );

        loop {
            
            terminal.draw(|f| {
                
                let size = f.area();

                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(20),
                        Constraint::Percentage(100),
                    ].as_ref())
                    .split(size);

                // ---------------------- Operation Panes ------------------ // 
                let operations_block = Block::default()
                    .title("Operations")
                    .borders(Borders::ALL);

                let ops: Vec<ListItem> = self.operations() 
                    .iter()
                    .map(|table| ListItem::new(*table))
                    .collect();
                
                let op_list = List::new(ops)
                    .block(operations_block)
                    .highlight_style(
                        if let Focus::Operations = focus {
                            Style::default().add_modifier(Modifier::REVERSED)
                        } else {
                            Style::default()
                        }
                    );

                f.render_stateful_widget(
                    op_list, 
                    main_chunks[0],
                    &mut self.operation_state
                );

                // ----------------------- Main Pane ----------------------- //
                let main_area = main_chunks[1];                

                let nested_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ])
                    .split(main_area);

                let main_block = Block::default()
                    .borders(Borders::ALL);
 
                f.render_widget(main_block, main_area);

                let top_items: Vec<ListItem> = self.options
                    .get(selected_op)
                    .into_iter()
                    .flat_map(|v| v.iter())
                    .map(|s| ListItem::new(*s))
                    .collect();

                let top_list = List::new(top_items)
                    .block(
                        Block::default()
                            .title(selected_op)
                            .borders(Borders::ALL)
                    )
                    .highlight_style(
                        if let Focus::Top = focus {
                            Style::default().add_modifier(Modifier::REVERSED)
                        } else {
                            Style::default()
                        }
                    );
                
                f.render_stateful_widget(
                    top_list,
                    nested_chunks[0],
                    &mut top_state
                );

                // let btm_items: Vec<ListItem> = match selected_action {
                //     Some(action) => {
                //         match selected_op {
                //              
                //         }
                //     }, 
                //     None => Vec::new()
                // };

                // let btm_list = List::new(btm_items)
                //     .block(
                //         Block::default()
                //             .title(match selected_action {
                //                 Some(t) => t,
                //                 None => ""
                //             })
                //             .borders(Borders::ALL)
                //     );
                // 
                // f.render_widget(btm_list, nested_chunks[1]);

            })?;

            // Handle events
            if let Event::Key(key) = event::read()? {

            }
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())

        // loop {
        //     transmitter.send(AppEvent::Tick).await; 
        //     sleep(Duration::new(1, 0)).await; 
        // }

    }

}



