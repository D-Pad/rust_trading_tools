use std::{collections::{HashMap, VecDeque}, io::{self}};

use sqlx::PgPool;
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
        Style,
        Color
    }, 
    widgets::{
        Block,
        Borders,
        List,
        ListItem,
        ListState,
        Wrap,
        Paragraph 
    },
    text::{Text, Line}
};
use app_core::{
    engine::Engine,
    database_ops::{
        fetch_exchanges_and_pairs_from_db
    }
};

use tokio::sync::mpsc::{channel, Sender, Receiver};


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
    Main, 
}

// ------------ SCREENS ------------- //
enum Screen<'a> {
    DatabaseManager(DatabaseScreen<'a>),
    CandleBuilder(CandleScreen),
    SystemSettings(SettingsScreen),
    Placeholder,
}


// ------------ DATABASE SCREEN -------------- //
struct DatabaseScreen<'a> {
    focus: DbFocus,
    top_state: ListState,
    btm_state: ListState,
    selected_action: Option<&'a DbAction>,
    token_pairs: HashMap<String, Vec<String>>,
    db_pool: PgPool,
}

impl<'a> DatabaseScreen<'a> {
 
    fn new(db_pool: PgPool) -> Self {
    
        let mut top_state = ListState::default();
        top_state.select(Some(0));
 
        DatabaseScreen {
            focus: DbFocus::Top,
            top_state,
            btm_state: ListState::default(),
            selected_action: None,
            token_pairs: HashMap::new(),
            db_pool,
        }

    }

    async fn pre_draw(&mut self) {
        let pool = self.db_pool.clone();
        self.token_pairs = fetch_exchanges_and_pairs_from_db(pool).await;
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
                    .title(Self::SCREEN_NAME)
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

        let btm_items: Vec<ListItem> = match self.selected_action {
            Some(DbAction::RemovePairs | DbAction::UpdateData) => {
                let mut items = Vec::new();
                for (key, vals) in &self.token_pairs {
                    for v in vals {
                        items.push(ListItem::new(format!("{key} - {v}")))
                    }
                };
                items
            },
            Some(DbAction::AddPairs) | None => Vec::new(),
        }; 

        let btm_list = List::new(btm_items)
            .block(
                Block::default()
                    .title(match self.selected_action {
                        Some(t) => t.name(),
                        None => ""
                    })
                    .borders(Borders::ALL)
            )
            .highlight_style(
                if let DbFocus::Bottom = self.focus {
                    Style::default().add_modifier(Modifier::REVERSED)
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

    fn pairs_to_vec(&self) -> Vec<(String, String, ListItem)> {
        
        let mut vals: Vec<(String, String, ListItem)> = Vec::new();
        
        for (key, val) in &self.token_pairs {
            for v in val {
                vals.push(
                    (   
                        key.clone(),
                        v.clone(),
                        ListItem::new(format!("{} - {}", key, v))
                    ),
                );
            };
        };
        vals 
    }

    fn handle_key(&mut self, key: KeyEvent) {

        let top_len = Self::SCREEN_OPTIONS.len();

        match key.code {
            
            KeyCode::Up | KeyCode::Char('k') => match self.focus {
                DbFocus::Top => {
                    move_up(&mut self.top_state, top_len);
                }
                DbFocus::Bottom => {
                    move_up(&mut self.btm_state, top_len);
                }
            },

            KeyCode::Down | KeyCode::Char('j') => match self.focus {
                DbFocus::Top => {
                    move_down(&mut self.top_state, top_len);
                }
                DbFocus::Bottom => {
                    move_down(&mut self.btm_state, top_len);
                }
            },

            KeyCode::Enter => match self.focus {
                DbFocus::Top => {
                    if let Some(i) = self.top_state.selected() {
                        self.selected_action = Some(&Self::SCREEN_OPTIONS[i]);
                    };

                    self.focus = DbFocus::Bottom;
                    self.btm_state.select(Some(0));
                }

                DbFocus::Bottom => {
                     
                }
            },

            KeyCode::Esc => match self.focus {
                DbFocus::Bottom => {
                    self.focus = DbFocus::Top;
                    self.selected_action = None;
                }
                DbFocus::Top => {
                    self.top_state.select(None);
                }
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
    operation_state: ListState,
    screen: Screen<'a>,
    output_buffer: VecDeque<Line<'static>>,
    engine: Engine,
    transmitter: Sender<String>,
    receiver: Receiver<String>,
}

impl<'a> TerminalInterface<'a> {
    
    pub async fn new(engine: Engine) -> Self {
        
        let mut operation_state = ListState::default();
        operation_state.select(Some(0));
        
        let screen: Screen = Screen::Placeholder;
        let output_buffer: VecDeque<Line<'static>> = VecDeque::new();

        let (transmitter, receiver) = channel(10); 

        TerminalInterface { 
            operation_state,
            screen,
            output_buffer,
            engine,
            transmitter,
            receiver
        }
    }

    fn add_line(
        &mut self, 
        msg: &'static str, 
        color: Color, 
        bold: bool,
        bg: Option<Color>
    ) {
        
        let mut style = Style::default().fg(color);
        if bold {
            style = style.bold();
        };

        if let Some(col) = bg {
            style = style.bg(col)
        };

        self.output_buffer.push_back(Line::styled(msg, style));
    }

    fn clear_lines(&mut self) {
        self.output_buffer.clear();
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

        // self.add_line("Testing", Color::Red, true, Some(Color::Cyan));

        loop {

            match &mut self.screen {

                Screen::DatabaseManager(screen) => {
                    screen.pre_draw().await;
                },

                Screen::CandleBuilder(screen) => {
                    // screen.draw();
                },

                Screen::SystemSettings(screen) => {
                    // screen.draw();
                },

                Screen::Placeholder => {}
            };

            terminal.draw(|frame| {
                
                let size = frame.area();


                let vertical_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(5),   
                        Constraint::Length(7),
                    ])
                    .split(size);

                // --------------------- OUTPUT WINDOW --------------------- //
                let text = Text::from(
                    self.output_buffer
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                );
                
                let output = Paragraph::new(text)
                    .block(
                        Block::default()
                        .title("Output")
                        .borders(Borders::ALL))
                    .wrap(Wrap { trim: false });

                frame.render_widget(output, vertical_chunks[1]);

                // --------------------- MAIN PANE ------------------------- //
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(21),
                        Constraint::Percentage(100),
                    ].as_ref())
                    .split(vertical_chunks[0]);

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

                    Screen::Placeholder => {}
                }

            })?;

            // Handle events
            if let Event::Key(key) = event::read()? {
            
                if let KeyCode::Char('q') = key.code {
                    break;
                }
                else if let Focus::Operations = focus {
                   
                    match key.code {
                    
                        KeyCode::Up | KeyCode::Char('k') => {
                            move_up(
                                &mut self.operation_state, 
                                operations.len()
                            );
                        }, 
                        
                        KeyCode::Down | KeyCode::Char('j') => {
                            move_down(
                                &mut self.operation_state, 
                                operations.len()
                            );
                        },
                        
                        KeyCode::Enter => {
                            if let Some(i) = self.operation_state.selected() {
                                self.screen = match i {
                                    0 => Screen::DatabaseManager(
                                        DatabaseScreen::new(
                                            self.engine.database.get_pool() 
                                        )
                                    ),
                                    1 => Screen::CandleBuilder(
                                        CandleScreen::new()
                                    ),
                                    2 => Screen::SystemSettings(
                                        SettingsScreen::new()
                                    ),
                                    _ => Screen::Placeholder 
                                };
                                focus = Focus::Main;
                            }
                        },

                        _ => {}
                    }
                }
                else {

                    match &mut self.screen {


                        Screen::DatabaseManager(screen) => {

                            if let KeyCode::Esc = key.code {
                                if let DbFocus::Top = screen.focus {
                                    focus = Focus::Operations;
                                };
                            };
                            screen.handle_key(key);

                        },

                        _ => {} 

                    } 

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



