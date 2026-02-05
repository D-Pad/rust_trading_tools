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
    database_ops::{
        DataDownloadStatus, 
        fetch_exchanges_and_pairs_from_db,
        update_database_tables
    }, engine::Engine
};

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};


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


// -------------- MESSAGING ------------------ //
struct OutputMsg {
    text: String,
    color: Color,
    bold: bool,
    bg_color: Option<Color>,
}

impl OutputMsg {
    fn new(text: String, color: Color, bold: bool, bg_color: Option<Color>) 
        -> Self {
        OutputMsg { text, color, bold, bg_color }
    }
}

impl From<DataDownloadStatus> for OutputMsg {
    
    fn from(status: DataDownloadStatus) -> Self {
        
        match status {
            DataDownloadStatus::Started { exchange, ticker } => {
                OutputMsg::new(
                    format!("Starting download: {exchange} / {ticker}"),
                    Color::Green,
                    true,
                    None,
                )
            }

            DataDownloadStatus::Progress {
                exchange,
                ticker,
                percent,
            } => {
                OutputMsg::new(
                    format!("Downloading {exchange} / {ticker}: {percent}%"),
                    Color::Yellow,
                    false,
                    None,
                )
            }

            DataDownloadStatus::Finished { exchange, ticker } => {
                OutputMsg::new(
                    format!("Finished download: {exchange} / {ticker}"),
                    Color::Green,
                    false,
                    None,
                )
            }

            DataDownloadStatus::Error { exchange, ticker } => {
                OutputMsg::new(
                    format!("ERROR downloading {exchange} / {ticker}"),
                    Color::Red,
                    true,
                    None,
                )
            }
        }
    }
}


// ------------ DATABASE SCREEN -------------- //
struct DatabaseScreen<'a> {
    focus: DbFocus,
    top_state: ListState,
    btm_state: ListState,
    btm_item_data: Vec<String>,
    selected_action: Option<&'a DbAction>,
    token_pairs: HashMap<String, Vec<String>>,
    db_pool: PgPool,
    transmitter: UnboundedSender<OutputMsg>,
}

impl<'a> DatabaseScreen<'a> {
 
    fn new(db_pool: PgPool, transmitter: UnboundedSender<OutputMsg>) -> Self {
    
        let mut top_state = ListState::default();
        top_state.select(Some(0));
 
        DatabaseScreen {
            focus: DbFocus::Top,
            top_state,
            btm_state: ListState::default(),
            btm_item_data: Vec::new(),
            selected_action: None,
            token_pairs: HashMap::new(),
            db_pool,
            transmitter
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

        self.btm_item_data = match self.selected_action {
            Some(DbAction::RemovePairs | DbAction::UpdateData) => {
                let mut items = Vec::new();
                for (key, vals) in &self.token_pairs {
                    for v in vals {
                        items.push(format!("{key} - {v}"))
                    }
                };
                items
            },
            Some(DbAction::AddPairs) | None => Vec::new(),
        };

        let btm_items: Vec<ListItem> = self.btm_item_data.iter()
            .map(|v| ListItem::new(v.clone()))
            .collect();

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
    
    async fn handle_key(&mut self, key: KeyEvent, engine: &Engine) {

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

                    if let Some(i) = self.btm_state.selected() {

                        let (prog_tx, mut prog_rx) = 
                            unbounded_channel::<DataDownloadStatus>();

                        let ui_tx = self.transmitter.clone();

                        tokio::spawn(async move {
                            while let Some(status) = prog_rx.recv().await {
                                let msg: OutputMsg = status.into();
                                let _ = ui_tx.send(msg); 
                            }
                        });
                  
                        let time_offset = engine.state.time_offset();
                        let client = &engine.request_client;
                        let db_pool = self.db_pool.clone();
                        
                        let tokens: Vec<&str> = self.btm_item_data[i]
                            .split(" - ")
                            .collect();

                        let exchange: &str = tokens[0];
                        let ticker: &str = tokens[1];

                        self.transmitter.send(
                            OutputMsg::new(
                                "Testing".to_string(), 
                                Color::Red, 
                                true, 
                                Some(Color::Cyan)
                            )
                        );
                        // update_database_tables(
                        //     &engine.state.get_active_exchanges(),
                        //     time_offset, 
                        //     client, 
                        //     db_pool, 
                        //     prog_tx, 
                        //     Some(&exchange), 
                        //     Some(&ticker)
                        // ).await;
                    }
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
}

impl<'a> TerminalInterface<'a> {
    
    pub async fn new(engine: Engine) -> Self {
        
        let mut operation_state = ListState::default();
        operation_state.select(Some(0));
        
        let screen: Screen = Screen::Placeholder;
        let output_buffer: VecDeque<Line<'static>> = VecDeque::new();

        TerminalInterface { 
            operation_state,
            screen,
            output_buffer,
            engine,
        }
    }

    fn add_line(&mut self, msg: OutputMsg) {
        
        let mut style = Style::default().fg(msg.color);
        if msg.bold {
            style = style.bold();
        };

        if let Some(col) = msg.bg_color {
            style = style.bg(col)
        };

        self.output_buffer.push_back(Line::styled(msg.text, style));
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

        let (transmitter, mut receiver) = unbounded_channel::<OutputMsg>();

        loop {

            while let Ok(msg) = receiver.try_recv() {
                self.add_line(msg);
            }

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
                                            self.engine
                                                .database
                                                .get_pool(),
                                            
                                            transmitter.clone()
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
                            screen.handle_key(key, &self.engine).await;

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



