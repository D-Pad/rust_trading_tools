use std::{
    collections::{BTreeMap, VecDeque}, 
    io::{self}, 
    time::Duration,
    sync::Arc,
};


use ratatui::{
    Frame, 
    Terminal, 
    backend::CrosstermBackend, 
    crossterm::{
        event::{
            self,
            Event,
            KeyCode, 
            KeyEvent, 
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
    }, 
    text::{
        Line, 
        Text
    }, 
    widgets::{
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
    sync::mpsc::{
        UnboundedSender, 
        unbounded_channel
    },
    time::interval
};


use app_core::{
    database_ops::{
        fetch_exchanges_and_pairs_from_db, kraken::{
            AssetPairInfo, 
            request_all_assets_from_kraken
        } 
    }, 
    engine::Engine,
    errors::{ConfigError},
};

mod screens;
use screens::{
    database::{
        DatabaseScreen, 
        DbFocus
    },
    settings::{
        SettingsScreen,
        FormMode, 
    },
    candles::{
        CandleScreen,
        CandleFocus,
    },
    AppEvent,
    Focus,
    OutputMsg,
    Screen,
    move_up,
    move_down,
};
use string_helpers::multi_line_to_single_line;


// ---------------------------- TERMINAL INTERFACE ------------------------- //
/// # Terminal User Interface (TUI)
///
/// When running, allows the user to add new token pairs to the database (or 
/// delete pairs from it), build and export candle data as CSV files, and
/// adjust global system settings. Create a TUI instance with the `new` 
/// method, then start it with `run().await`
/// ```
/// let engine = app_core::Engine::new(db_pool: PgPool);
/// let tui = TerminalUserInterface::new(engine);
/// tui.run().await;
/// ```
pub struct TerminalInterface {
    operation_state: ListState,
    screen: Screen,
    output_buffer: VecDeque<Line<'static>>,
    output_scroll: u16,
    output_area: Rect,
    asset_pairs: Arc<BTreeMap<String, BTreeMap<String, AssetPairInfo>>>,
    engine: Engine,
}

impl TerminalInterface {
    
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

    /// Adds lines of text to the output window
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

    /// Removes all lines from the output window
    fn clear_lines(&mut self) {
        self.output_buffer.clear();
        self.output_scroll = 0;
    }

    /// Draws the TUI.
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

        let hint_window: Paragraph = Paragraph::new(
                match self.screen {
                    Screen::Placeholder => format!("D-Trade:\n\n{}\n\n{}",
                        multi_line_to_single_line(
                            r#"Press 'Enter' to choose an option, and 'Esc' to 
                            return to the previous window. Up and down arrow
                            keys are used for navigation. Vim style navigation 
                            works as well ('j' key for down and 'k' for up)."#, 
                            main_area.width
                        ),
                        "Press 'q' to quit"
                    ),
                    _ => String::new()
                }
            )
            .block(main_block)
            .style(Style::default().fg(Color::White));

        frame.render_widget(hint_window, main_area);

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
                    Style::default().add_modifier(Modifier::REVERSED).green()
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
                screen.draw(frame, main_area);
            },

            Screen::SystemSettings(screen) => {
                screen.draw(frame, main_area);
            },

            Screen::Placeholder => {}
        }
    }

    /// Runs the TUI
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
                    },
                    AppEvent::Clear => self.clear_lines()
                }
            }

            match &mut self.screen {

                Screen::DatabaseManager(screen) => {
                    screen.pre_draw().await;
                },

                _ => {}
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

    /// Renders messages and stores then adds them to the output window
    /// with `self.add_line(msg)`
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

            _ => {
                msgs_to_render.push(msg);
            }
        
        }
               
        if clear_lines {
            self.clear_lines();
        } 
        for msg in msgs_to_render {
            self.add_line(&msg);
        };

    }

    /// Handles key inputs.
    ///
    /// After handling the key at the global level, passes the KeyEvent down 
    /// to the active screen for further KeyEvent handling.
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
                            1 => {
                                let pairs = fetch_exchanges_and_pairs_from_db(
                                    self.engine.database.get_pool()
                                ).await; 
                                Screen::CandleBuilder(
                                    CandleScreen::new(
                                        pairs,
                                        transmitter,
                                        self.engine.database.get_pool()
                                    )
                                )
                            },
                            2 => Screen::SystemSettings(
                                SettingsScreen::new(
                                    &self.engine.state.config,
                                    transmitter 
                                )
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

            let mut breakout: bool = false;
            
            match &mut self.screen {

                Screen::DatabaseManager(screen) => {

                    if let KeyCode::Esc = key.code {
                        if let DbFocus::Top = screen.focus {
                            new_focus = Focus::Operations;
                            breakout = true; 
                        };
                    };
                    screen.handle_key(key, &self.engine).await;

                },

                Screen::SystemSettings(screen) => {
                    
                    if let KeyCode::Esc = key.code {
                        
                        if let FormMode::Movement = screen.config_form.mode {
                            screen.active = false;
                            new_focus = Focus::Operations;
                            breakout = true; 
                        };

                        transmitter.send(AppEvent::Clear);
                        
                        match screen.config_form.save_input_values(
                            &self.engine.state.config,
                            &self.engine.state.paths
                        ) {
                            Ok(c) => {
                                transmitter.send(AppEvent::Output(
                                    OutputMsg { 
                                        text: "Settings saved!".to_string(), 
                                        color: Color::Green, 
                                        bold: true, 
                                        bg_color: None, 
                                        exchange: None, 
                                        ticker: None 
                                    }
                                ));

                                self.engine.state.config = c;
                            },

                            Err(e) => {
                                let mut msg: String = String::new();
                                let mut col: Color = Color::Red;
                                match e {
                                    ConfigError::NoChangesMade => {
                                        msg = String::from(
                                            "No changes detected. Not saved."
                                        );
                                        col = Color::Yellow;
                                    },
                                    _ => {
                                        msg = format!(
                                            "Settings save failed: {}", e
                                        );
                                    }
                                };
                                transmitter.send(AppEvent::Output(
                                    OutputMsg { 
                                        text: msg, 
                                        color: col, 
                                        bold: true, 
                                        bg_color: None, 
                                        exchange: None, 
                                        ticker: None 
                                    }
                                ));
                            }
                        };
                       
                    };
                    
                    screen.handle_key(key).await;
                },
                
                Screen::CandleBuilder(screen) => {
                    if let KeyCode::Esc = key.code {
                        if let CandleFocus::Top = screen.focus {
                            new_focus = Focus::Operations;
                            breakout = true; 
                        };
                    };
                    screen.handle_key(key).await;
                },

                _ => {}

            } 
                           
            if breakout {
                self.screen = Screen::Placeholder;
            };

        };

        new_focus 
    
    }
}



