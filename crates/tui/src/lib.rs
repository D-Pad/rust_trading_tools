use std::{
    collections::{HashMap, BTreeMap, VecDeque}, 
    io::{self}, 
    time::Duration,
    sync::Arc,
    cmp::min
};


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
            KeyEvent, KeyModifiers
        }, 
        execute,
        terminal::{
            EnterAlternateScreen, 
            LeaveAlternateScreen, 
            disable_raw_mode, 
            enable_raw_mode
        }
    }, layout::{
        Constraint,
        Direction,
        Layout,
        Rect
    }, style::{
        Color, 
        Modifier, 
        Style
    }, text::{Line, Text}, widgets::{
        Block, 
        Borders, 
        List, 
        ListItem, 
        ListState, 
        Paragraph, 
        Wrap 
    }
};
use tokio::{
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle, 
    time::interval
};


use app_core::{
    database_ops::{
        self, 
        DataDownloadStatus, 
        fetch_exchanges_and_pairs_from_db, 
        kraken::{
            AssetPairInfo, request_all_assets_from_kraken
        }, 
        update_database_tables
    }, 
    engine::Engine
};
use string_helpers::capitlize_first_letter;

pub mod system_settings;
pub use system_settings::{
    ConfigForm
};


fn move_up(state: &mut ListState, len: usize, step: usize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i > 0 => i - min(i, step),
        _ => 0,
    };
    state.select(Some(i));
}

fn move_down(state: &mut ListState, len: usize, step: usize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i + step < len => i + step,
        Some(i) if i + step >= len => len,
        _ => 0,
    };
    state.select(Some(i));
}


#[derive(Clone)]
enum Focus {
    Operations,
    Main,
    Quit,
}

enum AppEvent {
    Input(KeyEvent),
    Output(OutputMsg),
    Tick,
}

// ------------ SCREENS ------------- //
enum Screen<'a> {
    DatabaseManager(DatabaseScreen<'a>),
    CandleBuilder(CandleScreen),
    SystemSettings(SettingsScreen),
    Placeholder,
}


// -------------- MESSAGING ------------------ //
#[derive(Clone)]
struct OutputMsg {
    text: String,
    color: Color,
    bold: bool,
    bg_color: Option<Color>,
    exchange: Option<String>,
    ticker: Option<String>,
}

impl OutputMsg {
    fn new(
        text: String, 
        color: Color, 
        bold: bool, 
        bg_color: Option<Color>,
        exchange: Option<String>,
        ticker: Option<String>
    ) 
        -> Self {
        OutputMsg { text, color, bold, bg_color, exchange, ticker }
    }
}

impl From<DataDownloadStatus> for OutputMsg {
    
    fn from(status: DataDownloadStatus) -> Self {
        
        match status {
            DataDownloadStatus::Started { exchange, ticker } => {
                OutputMsg::new(
                    format!("  {ticker}: 0%"),
                    Color::Yellow,
                    true,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }

            DataDownloadStatus::Progress {
                exchange,
                ticker,
                percent,
            } => {
                OutputMsg::new(
                    format!("  {ticker}: {percent}%"),
                    Color::Yellow,
                    false,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }

            DataDownloadStatus::Finished { exchange, ticker } => {
                OutputMsg::new(
                    format!("  {ticker}: Finished"),
                    Color::Green,
                    false,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }

            DataDownloadStatus::Error { exchange, ticker } => {
                OutputMsg::new(
                    format!("  {ticker}: ERROR"),
                    Color::Red,
                    true,
                    None,
                    Some(exchange),
                    Some(ticker),
                )
            }
        }
    }
}


// ------------ DATABASE SCREEN -------------- //
struct DatabaseUpdateMsgs {
    msgs: BTreeMap<String, BTreeMap<String, OutputMsg>>,
}

impl DatabaseUpdateMsgs {
    fn new() -> Self {
        DatabaseUpdateMsgs { msgs: BTreeMap::new() } 
    }
}

struct DatabaseScreen<'a> {
    focus: DbFocus,
    top_state: ListState,
    btm_state: ListState,
    btm_item_data: Vec<String>,
    selected_action: Option<&'a DbAction>,
    token_pairs: HashMap<String, Vec<String>>,
    asset_pairs: Arc<BTreeMap<String, BTreeMap<String, AssetPairInfo>>>,
    db_pool: PgPool,
    transmitter: UnboundedSender<AppEvent>,
    is_busy: bool,
    task_handle: Option<JoinHandle<()>>,
    db_update_msgs: DatabaseUpdateMsgs, 
}

impl<'a> DatabaseScreen<'a> {
 
    fn new(
        db_pool: PgPool, 
        transmitter: UnboundedSender<AppEvent>,
        asset_pairs: Arc<BTreeMap<String, BTreeMap<String, AssetPairInfo>>>, 
    ) -> Self {
    
        let mut top_state = ListState::default();
        top_state.select(Some(0));
        let is_busy: bool = false;
        let task_handle: Option<JoinHandle<()>> = None;

        DatabaseScreen {
            focus: DbFocus::Top,
            top_state,
            btm_state: ListState::default(),
            btm_item_data: Vec::new(),
            selected_action: None,
            token_pairs: HashMap::new(),
            asset_pairs,
            db_pool,
            transmitter,
            is_busy,
            task_handle,
            db_update_msgs: DatabaseUpdateMsgs::new(),
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
            Some(DbAction::AddPairs) => {
                let mut items = Vec::new();
                for (key, pairs) in self.asset_pairs.iter() {
                    let exchange_title: String = capitlize_first_letter(key); 
                    for (asset, _) in pairs.iter() {
                        items.push(format!("{} - {}", exchange_title, asset))
                    }
                };
                items
            },
            Some(DbAction::None) | None => Vec::new(),
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

    async fn handle_btm_action(&mut self, engine: &Engine) {
 
        let ACTION = match self.selected_action {
            Some(a) => a,
            None => &Self::SCREEN_OPTIONS[3]
        };

        if let Some(i) = self.btm_state.selected() {

            // Update option
            if let DbAction::UpdateData = ACTION { 
               
                let (prog_tx, mut prog_rx) = 
                    unbounded_channel::<DataDownloadStatus>();

                let ui_tx = self.transmitter.clone();

                tokio::spawn(async move {
                    while let Some(stat) = prog_rx.recv().await {
                        let msg: OutputMsg = stat.into();
                        let _ = ui_tx.send(AppEvent::Output(msg)); 
                    }
                });
        
                let time_offset = engine.state.time_offset();
                let client = engine.request_client.clone();
                let db_pool = self.db_pool.clone();
                
                let tokens: Vec<&str> = self.btm_item_data[i]
                    .split(" - ")
                    .collect();

                let exchange: String = tokens[0].to_lowercase();
                let ticker: String = tokens[1].to_uppercase();

                let active_exchanges = engine.state
                    .get_active_exchanges();

                self.task_handle = Some(tokio::spawn(async move {
                    update_database_tables(
                        &active_exchanges,
                        time_offset, 
                        &client, 
                        db_pool, 
                        prog_tx, 
                        Some(&exchange), 
                        Some(&ticker)
                    ).await;
                }));
            }

            else if let DbAction::AddPairs = ACTION {

                if self.btm_item_data.len() > 0 { 

                    let tokens: Vec<&str> = self.btm_item_data[i]
                        .split(" - ")
                        .collect();

                    let exchange: String = tokens[0].to_lowercase();
                    let ticker: String = tokens[1].to_uppercase();

                    let tx = self.transmitter.clone();

                    let time_offset = engine.state.time_offset();
                    let db_pool = engine.database.get_pool();
                    let client = engine.request_client.clone();
                    let asset_pairs = self.asset_pairs.clone();

                    self.task_handle = Some(tokio::spawn(async move {
                        
                        tx.send(AppEvent::Output(OutputMsg::new(
                            format!("Downloading seed data..."),
                            Color::Yellow,
                            false,
                            None,
                            None,
                            None
                        )));

                        database_ops::add_new_pair(
                            &exchange, 
                            &ticker, 
                            time_offset, 
                            db_pool, 
                            &client,
                            Some(&*asset_pairs)
                        ).await;
                        
                        tx.send(AppEvent::Output(OutputMsg::new(
                            format!("Added {} {}", exchange, ticker),
                            Color::Green,
                            true,
                            None,
                            None,
                            None
                        )));
                    }));
                };
            }

            else if let DbAction::RemovePairs = ACTION {

                if self.btm_item_data.len() > 0 { 

                    let tokens: Vec<&str> = self.btm_item_data[i]
                        .split(" - ")
                        .collect();

                    let exchange: String = tokens[0].to_lowercase();
                    let ticker: String = tokens[1].to_uppercase();
                    let tx = self.transmitter.clone();
                    let db_pool = engine.database.get_pool();

                    self.task_handle = Some(tokio::spawn(async move {

                        database_ops::drop_pair(
                            &exchange, 
                            &ticker, 
                            db_pool, 
                        ).await;
                        
                        tx.send(AppEvent::Output(OutputMsg::new(
                            format!("Deleted {} {}", exchange, ticker),
                            Color::Magenta,
                            true,
                            None,
                            None,
                            None
                        )));
                    }));
                };
            };
        }
    }

    async fn handle_key(&mut self, key: KeyEvent, engine: &Engine) {

        self.set_task_state_if_free();
        if self.is_busy { return };

        let top_len = Self::SCREEN_OPTIONS.len();
        let btm_len = self.btm_item_data.len();
        const PAGE_STEP: usize = 10;

        match (key.code, key.modifiers) {
           
            // -------------------- SINGLE STEP MOVEMENTS ------------------ //
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => match self.focus {
                
                DbFocus::Top => {
                    move_up(&mut self.top_state, top_len, 1);
                }
                
                DbFocus::Bottom => {
                    move_up(&mut self.btm_state, btm_len, 1);
                }
            },

            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => match self.focus {
                
                DbFocus::Top => {
                    move_down(&mut self.top_state, top_len, 1);
                }
                
                DbFocus::Bottom => {
                    move_down(&mut self.btm_state, btm_len, 1);
                }
            },

            // --------------------- FULL PAGE MOVEMENTS ------------------- //
            (KeyCode::Char('d'), mods) 
                if mods.contains(KeyModifiers::CONTROL) => match self.focus {
                
                    DbFocus::Top => {
                        move_down(&mut self.top_state, top_len, PAGE_STEP);
                    }
                    
                    DbFocus::Bottom => {
                        move_down(&mut self.btm_state, btm_len, PAGE_STEP);
                    }
            },
 
            (KeyCode::Char('u'), mods) 
                if mods.contains(KeyModifiers::CONTROL) => match self.focus {
                
                    DbFocus::Top => {
                        move_up(&mut self.top_state, top_len, PAGE_STEP);
                    }
                    
                    DbFocus::Bottom => {
                        move_up(&mut self.btm_state, btm_len, PAGE_STEP);
                    }
            },

            // ------------------------- ENTER & ESC ----------------------- //
            (KeyCode::Enter, _) => match self.focus {
                
                DbFocus::Top => {
                    if let Some(i) = self.top_state.selected() {
                        self.selected_action = Some(&Self::SCREEN_OPTIONS[i]);
                    };

                    self.focus = DbFocus::Bottom;
                    self.btm_state.select(Some(0));
                }

                DbFocus::Bottom => {

                    self.handle_btm_action(engine).await

                }
            },

            (KeyCode::Esc, _) => match self.focus {
                
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

    fn set_task_state_if_free(&mut self) -> bool {
        /// Returns true if task is busy, and false if free 
      
        let mut is_busy = false;
        
        if let Some(handle) = &self.task_handle {
            
            if handle.is_finished() { 
                self.is_busy = false;
                self.task_handle = None;
            }
            
            else {
                is_busy = true;
                self.is_busy = true;
            };
        };
        is_busy
    }

    const SCREEN_NAME: &'static str = "Database Management";

    const SCREEN_OPTIONS: [DbAction; 4] = [
        DbAction::AddPairs, 
        DbAction::RemovePairs, 
        DbAction::UpdateData,
        DbAction::None
    ];

}

enum DbFocus {
    Top,
    Bottom
}

enum DbAction {
    AddPairs,
    RemovePairs,
    UpdateData,
    None
}

impl DbAction {
    fn name(&self) -> &'static str {
        match self {
            DbAction::AddPairs => "Add new pairs",
            DbAction::RemovePairs => "Delete pairs",
            DbAction::UpdateData => "Update data",
            _ => ""
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
    output_scroll: u16,
    output_area: Rect,
    asset_pairs: Arc<BTreeMap<String, BTreeMap<String, AssetPairInfo>>>,
    engine: Engine,
}

impl<'a> TerminalInterface<'a> {
    
    pub async fn new(engine: Engine) -> Self {
        
        let mut operation_state = ListState::default();
        operation_state.select(Some(0));
        
        let screen: Screen = Screen::Placeholder;
        let output_buffer: VecDeque<Line<'static>> = VecDeque::new();

        let asset_pairs = Arc::new(BTreeMap::from([
            (
                "kraken".to_string(), 
                match request_all_assets_from_kraken(
                    &engine.request_client
                ).await {
                    Ok(d) => d,
                    Err(_) => BTreeMap::new()
                } 
            )
        ]));

        TerminalInterface { 
            operation_state,
            screen,
            output_buffer,
            output_scroll: 0,
            output_area: Rect::new(0, 0, 0, 0),
            asset_pairs,
            engine,
        }
    }

    fn add_line(&mut self, msg: &OutputMsg) {
        
        let mut style = Style::default().fg(msg.color);
        if msg.bold {
            style = style.bold();
        };

        if let Some(col) = msg.bg_color {
            style = style.bg(col)
        };

        let visible_height = self.output_area.height.saturating_sub(2);
        self.output_buffer.push_back(Line::styled(msg.text.clone(), style));
        self.output_scroll = self
            .output_buffer
            .len()
            .saturating_sub(visible_height as usize) 
            as u16;
    
    }

    fn clear_lines(&mut self) {
        self.output_buffer.clear();
        self.output_scroll = 0;
    }

    fn draw(
        &mut self, 
        frame: &mut Frame,
        operations: &[&'static str; 3],
        focus: &Focus
    ) {
 
        let size = frame.area();

        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),   
                Constraint::Length(10),
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
            .wrap(Wrap { trim: false })
            .scroll((self.output_scroll, 0));

        self.output_area = vertical_chunks[1];

        frame.render_widget(output, self.output_area);

        // --------------------- MAIN PANE ------------------------- //
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(21),
                Constraint::Percentage(100),
            ].as_ref())
            .split(vertical_chunks[0]);

        let main_area = main_chunks[1];                

        let main_block = Block::default()
            .borders(Borders::ALL);

        frame.render_widget(main_block, main_area);

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

        let (transmitter, mut receiver) = unbounded_channel::<AppEvent>();
        let listener_tx = transmitter.clone();
        let input_tx = transmitter.clone();

        let tick_listener = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(100)); // 10 FPS
            loop {
                ticker.tick().await;
                let _ = listener_tx.send(AppEvent::Tick); 
            }
        });

        let key_reader = tokio::spawn(async move {
            loop {
                if let Ok(_) = event::poll(Duration::from_millis(50)) {
                    if let Ok(e) = event::read() {
                        if let Event::Key(key) = e {
                            let _ = input_tx.send(AppEvent::Input(key));
                        }
                    }
                    else {
                        break;
                    }
                }
                else {
                    break;
                }
            }
        });
 
        loop {

            while let Ok(msg) = receiver.try_recv() {
                
                match msg {
                    
                    AppEvent::Input(key) => {
                        focus = self.handle_key(
                            key, 
                            &operations, 
                            focus, 
                            transmitter.clone()
                        ).await
                    },
                    
                    AppEvent::Tick => {}, // Nothing to do
                    
                    AppEvent::Output(msg) => {

                        self.render_messages(msg);

                    }
                }
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
                self.draw(frame, &operations, &focus);
            })?;

            if let Focus::Quit = focus { break };

        }

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        tick_listener.abort();
        key_reader.abort();

        Ok(())

    }

    fn render_messages(&mut self, msg: OutputMsg) {

        let mut msgs_to_render: Vec<OutputMsg> = Vec::new();
        let mut clear_lines: bool = false;

        match &mut self.screen {
            
            Screen::DatabaseManager(screen) => {
            
                // Handle database update messages here
                match (&msg.exchange, &msg.ticker) {
                    (Some(exchange), Some(ticker)) => {
                       
                        clear_lines = true;
                        &screen.db_update_msgs.msgs
                            .entry(exchange.to_string())
                            .or_insert_with(|| BTreeMap::new())
                            .insert(ticker.to_string(), msg);

                        for (ex, pairs) in &screen.db_update_msgs.msgs {
                            msgs_to_render.push(
                                OutputMsg::new(
                                    ex.to_string(),
                                    Color::Cyan,
                                    true,
                                    None,
                                    None,
                                    None
                                )
                            );
                            for (_, message) in pairs {
                                msgs_to_render.push(message.clone());
                            };
                        };
                    },
                    _ => {
                        msgs_to_render.push(msg)  
                    }

                }

            },
            _ => {}
        
        }
               
        if clear_lines {
            self.clear_lines();
        } 
        for msg in msgs_to_render {
            self.add_line(&msg);
        };

    }

    async fn handle_key(
        &mut self,
        key: KeyEvent, 
        operations: &[&'static str; 3],
        focus: Focus,
        transmitter: UnboundedSender<AppEvent>,
    ) -> Focus {
       
        let mut new_focus = focus.clone();

        if let KeyCode::Char('q') = key.code {
            return Focus::Quit;
        }
        
        else if let Focus::Operations = focus {
           
            match key.code {
            
                KeyCode::Up | KeyCode::Char('k') => {
                    move_up(
                        &mut self.operation_state, 
                        operations.len(),
                        1
                    );
                }, 
                
                KeyCode::Down | KeyCode::Char('j') => {
                    move_down(
                        &mut self.operation_state, 
                        operations.len(),
                        1
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
                                    
                                    transmitter,

                                    Arc::clone(&self.asset_pairs)
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
                        new_focus = Focus::Main;
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
                            new_focus = Focus::Operations;
                        };
                    };
                    screen.handle_key(key, &self.engine).await;

                },

                _ => {} 

            } 

        };

        new_focus 
    
    }
}



