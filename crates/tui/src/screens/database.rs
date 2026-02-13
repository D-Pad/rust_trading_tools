// Local imports
use std::{
    collections::{
        BTreeMap,
        HashMap
    },
    sync::Arc,
};

// Third party imports
use sqlx::PgPool;
use tokio::{
    task::{
        JoinHandle
    },
    sync::mpsc::{
        UnboundedSender, 
        unbounded_channel
    } 
};
use ratatui::{
    Frame,
    crossterm::{
        event::{
            KeyCode,
            KeyEvent,
            KeyModifiers
        },
    },
    widgets::{
        Block,
        Borders,
        ListState,
        ListItem,
        List,
    },
    layout::{
        Rect,
        Layout,
        Direction,
        Constraint,
    },
    style::{
        Style,
        Modifier,
        Color,
    },
};

// Local imports
use super::{
    AppEvent,
    OutputMsg,
    move_up,
    move_down
};
use app_core::{
    database_ops::{
        self,
        kraken::{
            AssetPairInfo,
        },
        fetch_exchanges_and_pairs_from_db,
        DataDownloadStatus, 
        update_database_tables,
    },
    engine::Engine,
};
use string_helpers::capitlize_first_letter;


// ------------ DATABASE SCREEN -------------- //
pub struct DatabaseUpdateMsgs {
    pub msgs: BTreeMap<String, BTreeMap<String, OutputMsg>>,
}

impl DatabaseUpdateMsgs {
    fn new() -> Self {
        DatabaseUpdateMsgs { msgs: BTreeMap::new() } 
    }
}

pub struct DatabaseScreen {
    pub focus: DbFocus,
    pub top_state: ListState,
    pub btm_state: ListState,
    pub btm_item_data: Vec<String>,
    pub selected_action: Option<DbAction>,
    pub token_pairs: HashMap<String, Vec<String>>,
    pub asset_pairs: Arc<BTreeMap<String, BTreeMap<String, AssetPairInfo>>>,
    pub db_pool: PgPool,
    pub transmitter: UnboundedSender<AppEvent>,
    pub is_busy: bool,
    pub task_handle: Option<JoinHandle<()>>,
    pub db_update_msgs: DatabaseUpdateMsgs, 
}

impl DatabaseScreen {
 
    pub fn new(
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

    pub async fn pre_draw(&mut self) {
        let pool = self.db_pool.clone();
        self.token_pairs = fetch_exchanges_and_pairs_from_db(pool).await;
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {

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
                    Style::default().add_modifier(Modifier::REVERSED).green()
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
                let mut items = Vec::from(["All Tables".to_string()]);
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
                    .title(match self.selected_action.clone() {
                        Some(t) => t.name(),
                        None => ""
                    })
                    .borders(Borders::ALL)
            )
            .highlight_style(
                if let DbFocus::Bottom = self.focus {
                    Style::default().add_modifier(Modifier::REVERSED).green()
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

    pub async fn handle_btm_action(&mut self, engine: &Engine) {
 
        let ACTION = match &self.selected_action {
            Some(a) => a.clone(),
            None => Self::SCREEN_OPTIONS[3].clone()
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
              
                let active_exchanges = engine.state
                    .get_active_exchanges();

                let pair = if self.btm_item_data[i] != "All Tables" {
                    
                    let tokens: Vec<&str> = self.btm_item_data[i]
                        .split(" - ")
                        .collect();

                    (
                        Some(tokens[0].to_lowercase()),
                        Some(tokens[1].to_uppercase())
                    )
                
                }
                else {
                    (None, None)
                };

                let (exchange, ticker) = pair;

                self.task_handle = Some(tokio::spawn(async move {
                    update_database_tables(
                        &active_exchanges,
                        time_offset, 
                        &client, 
                        db_pool, 
                        prog_tx, 
                        exchange.as_deref(), 
                        ticker.as_deref()
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

    pub async fn handle_key(&mut self, key: KeyEvent, engine: &Engine) {

        self.check_and_modify_task_state();
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
                        self.selected_action = Some(
                            Self::SCREEN_OPTIONS[i].clone()
                        );
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

    /// Sets the 'is_busy' task state
    pub fn check_and_modify_task_state(&mut self) {
      
        if let Some(handle) = &self.task_handle {
            
            if handle.is_finished() { 
                self.is_busy = false;
                self.task_handle = None;
            }
            
            else {
                self.is_busy = true;
            };
        };
    }

    pub const SCREEN_NAME: &'static str = "Database Management";

    pub const SCREEN_OPTIONS: [DbAction; 4] = [
        DbAction::AddPairs, 
        DbAction::RemovePairs, 
        DbAction::UpdateData,
        DbAction::None
    ];

}

pub enum DbFocus {
    Top,
    Bottom
}

#[derive(Clone)]
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


