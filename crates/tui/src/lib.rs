use std::{collections::{BTreeMap, HashMap}, io::{self}};

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
        Layout,
        Rect
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


fn move_up(state: &mut ListState, len: usize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => 0,
    };
    state.select(Some(i));
}

fn move_down(state: &mut ListState, len: usize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i + 1 < len => i + 1,
        _ => 0,
    };
    state.select(Some(i));
}


enum Focus {
    Operations,
    Top,
    Bottom
}

// ------------ SCREENS ------------- //
enum Screen<'a> {
    DatabaseManager(DatabaseScreen<'a>),
    CandleBuilder(CandleScreen),
    SystemSettings(SettingsScreen)
}


// ------------ DATABASE SCREEN -------------- //
struct DatabaseScreen<'a> {
    focus: DbFocus,
    top_state: ListState,
    btm_state: ListState,
    selected_action: Option<&'a DbAction>,
}

impl<'a> DatabaseScreen<'a> {
 
    fn new() -> Self {
      
        DatabaseScreen {
            focus: DbFocus::Top,
            top_state: ListState::default(),
            btm_state: ListState::default(),
            selected_action: None,
        }

    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {

        let nested_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
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
                    .title(match self.selected_action {
                        Some(a) => {
                            a.name()
                        },
                        None => ""
                    })
                    .borders(Borders::ALL)
            )
            .highlight_style(
                if let DbFocus::Top = self.focus {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                }
            );
        
        frame.render_stateful_widget(
            top_list,
            nested_chunks[0],
            &mut self.top_state
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


    }

    fn handle_key(&mut self, key: KeyEvent) {

        let top_len = Self::SCREEN_OPTIONS.len();

        match key.code {
            
            KeyCode::Up => match self.focus {
                DbFocus::Top => {
                    move_up(&mut self.top_state, top_len);
                }
                DbFocus::Bottom => {
                    // later
                }
            },

            KeyCode::Down => match self.focus {
                DbFocus::Top => {
                    move_down(&mut self.top_state, top_len);
                }
                DbFocus::Bottom => {
                    // later
                }
            },

            KeyCode::Enter => match self.focus {
                DbFocus::Top => {
                    if let Some(i) = self.top_state.selected() {
                        self.selected_action = Some(
                            &Self::SCREEN_OPTIONS[i]
                        )
                    };

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

    const SCREEN_NAME: &'static str = "Database Management";

    const SCREEN_OPTIONS: [DbAction; 3] = [
        DbAction::AddPairs, 
        DbAction::RemovePairs, 
        DbAction::UpdateData
    ];

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

impl DbAction {
    fn name(&self) -> &'static str {
        match self {
            DbAction::AddPairs => "Add new pairs",
            DbAction::RemovePairs => "Delete pairs",
            DbAction::UpdateData => "Update data",
        }
    }
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

impl CandleScreen {

    fn new() -> Self {
        
        CandleScreen {
            step: CandleStep::Exchange,
            exchange_state: ListState::default(),
            pair_state: ListState::default(),
            interval_state: ListState::default()
        }
    
    }

    fn draw(&mut self) {

    }

    fn handle_key(&self, key: KeyEvent) {

    }

    const SCREEN_NAME: &'static str = "Candle Builder";

    const SCREEN_OPTIONS: [&'static str; 0] = [];
}


// ------------- SYSTEM SETTINGS -------------- //
struct SettingsScreen {
    settings_state: ListState,
    dirty: bool,
}

impl SettingsScreen {

    fn new() -> Self {
        SettingsScreen {
            settings_state: ListState::default(),
            dirty: false
        } 
    }

    fn draw(&mut self) {

    }

    fn handle_key(&self, key: KeyEvent) {

    }

    const SCREEN_NAME: &'static str = "System Settings";

    const SCREEN_OPTIONS: [&'static str; 0] = [];

}


// ---------------------------- TERMINAL INTERFACE ------------------------- //
pub struct TerminalInterface<'a> {
    transmitter: Sender<AppEvent>,
    db_pool: PgPool,
    operation_state: ListState,
    token_pairs: HashMap<String, Vec<String>>,
    screen: Screen<'a>,
}

impl<'a> TerminalInterface<'a> {
    
    pub async fn new(transmitter: Sender<AppEvent>, db_pool: PgPool) -> Self {
        
        let mut operation_state = ListState::default();
        operation_state.select(Some(0));
        
        let tokens = fetch_exchanges_and_pairs_from_db(db_pool.clone()).await;

        let screen: Screen = Screen::DatabaseManager(DatabaseScreen::new());

        TerminalInterface { 
            transmitter, 
            db_pool, 
            operation_state,
            token_pairs: tokens,
            screen 
        }
    }

    pub async fn run(&mut self) 
        -> io::Result<()> {

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?; 
 
        let mut focus = Focus::Operations;

        let operations: [&'static str; 3] = [
            DatabaseScreen::SCREEN_NAME,
            CandleScreen::SCREEN_NAME,
            SettingsScreen::SCREEN_NAME
        ];

        loop {
            
            terminal.draw(|frame| {
                
                let size = frame.area();

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

                let ops: Vec<ListItem> = operations 
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

                frame.render_stateful_widget(
                    op_list, 
                    main_chunks[0],
                    &mut self.operation_state
                );

                // ----------------------- Main Pane ----------------------- //
                let main_area = main_chunks[1];                

                let main_block = Block::default()
                    .borders(Borders::ALL);
 
                frame.render_widget(main_block, main_area);

                match &mut self.screen {

                    Screen::DatabaseManager(screen) => {
                        screen.draw(frame, main_area);
                    },

                    Screen::CandleBuilder(screen) => {
                        // screen.draw();
                    },

                    Screen::SystemSettings(screen) => {
                        // screen.draw();
                    },
                }

            })?;

            // Handle events
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    _ => {}
                };
                match &mut self.screen {
                    Screen::DatabaseManager(screen) => {
                        screen.handle_key(key);
                    },

                    Screen::CandleBuilder(screen) => {
                        screen.handle_key(key);
                    },

                    Screen::SystemSettings(screen) => {
                        screen.handle_key(key);
                    },                   
                };
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



