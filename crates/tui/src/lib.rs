use std::io::{self, Stdout};

use tokio::{sync::mpsc::Sender, time::{sleep, Duration}};
use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode}, 
        execute,
        terminal::{
            disable_raw_mode, 
            enable_raw_mode, 
            EnterAlternateScreen,
            LeaveAlternateScreen
        }
    },
    backend::{
        CrosstermBackend
    },
    layout::{
        Constraint,
        Direction,
        Layout
    },
    widgets::{
        Block,
        Borders,
        List,
        ListItem
    },
    Terminal
};

use app_core::AppEvent;


pub async fn tui_test(transmitter: Sender<AppEvent>) 
    -> io::Result<()> {
  
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
   
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    

    let tables = vec!["asset_kraken_btcusd".to_string()]; 

    loop {
        // Draw UI
        terminal.draw(|f| {
            let size = f.area();

            // Create a vertical layout
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(100)
                ].as_ref())
                .split(size);

            // Pane for database tables
            let block = Block::default()
                .title("Database Tables")
                .borders(Borders::ALL);

            let items: Vec<ListItem> = tables
                .iter()
                .map(|table| ListItem::new(table.as_str()))
                .collect();

            let list = List::new(items).block(block);

            f.render_widget(list, chunks[0]);
        })?;

        // Handle events
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
            // Add more key handlers as needed for your app
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

