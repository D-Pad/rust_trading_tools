use app_core::app_state::{
    AppConfig
};
use string_helpers::capitlize_first_letter;

use ratatui::{
    Frame,
    crossterm::event::KeyEvent,
    layout::{
        Constraint,
        Direction, 
        Layout,
        Rect,
    },
    widgets::{
        Paragraph,
        Block,
        Borders,
    }
};


#[derive(Debug, Clone)]
pub enum FieldKind {
    Bool,
    Number,
    Text
}

pub enum ConfigFormError {
    InvalidKey
}

#[derive(Debug)]
pub struct ConfigField {
    pub label: String,
    pub kind: FieldKind,
    pub value: String,
    pub cursor: usize,
}

#[derive(Debug)]
pub enum FormRow {
    SectionDivider(String),
    InputRow(ConfigField),
}

/// A ConfigForm is intended to be used as a way for the user to interface
/// with the system settings, and make changes to it. Used in the TUI crate
#[derive(Debug)]
pub struct ConfigForm {
    pub focused: usize,
    pub rows: Vec<FormRow>,
}

impl ConfigForm {

    /// Takes an AppConfig reference and returns a ConfigForm
    ///
    /// Use this to build a  ConfigForm, to be used in a terminal user 
    /// interface. Intended to be used as a way for the user to edit system 
    /// settings from an interface.
    pub fn from_config(cfg: &AppConfig) -> Self {

        let mut rows: Vec<FormRow> = Vec::new();
            
        rows.push(FormRow::SectionDivider(
            "Backtest Settings".to_string()
        ));
        rows.push(FormRow::InputRow(
            ConfigField {
                label: "Inside Bar Testing".to_string(),
                kind: FieldKind::Bool,
                value: cfg.backtesting.inside_bar.to_string(),
                cursor: 0,
            })
        );

        rows.push(FormRow::SectionDivider(
            "Chart Parameters".to_string()
        ));
        rows.push(FormRow::InputRow(
            ConfigField {
                label: "Max number of bars on chart".to_string(),
                kind: FieldKind::Number,
                value: cfg.chart_parameters.num_bars.to_string(),
                cursor: 0,
            })
        );

        rows.push(FormRow::SectionDivider(
            "Active Exchanges".to_string() 
        )); 
        for (exchange, enabled) in &cfg.supported_exchanges.active {
            rows.push(
                FormRow::InputRow(
                    ConfigField {
                        label: capitlize_first_letter(exchange),
                        kind: FieldKind::Bool,
                        value: enabled.to_string(),
                        cursor: 0,
                    }
                )
            ); 
        };

        rows.push(FormRow::SectionDivider(
            "Data Download Settings".to_string()
        ));
        rows.push(FormRow::InputRow(
            ConfigField {
                label: "Initial download cache size".to_string(),
                kind: FieldKind::Text,
                value: cfg.data_download.cache_size.clone(),
                cursor: 0,
            })
        );

        ConfigForm {
            focused: 0,
            rows 
        }

    }

    /// Converts a key into a human readable string 
    ///
    /// Takes a ConfigForm field BTreeMap key and turns it into a 
    /// human readable title
    pub fn key_to_title(key: &String) -> Result<&'static str, ConfigFormError> {
 
        match &key[..] {
            "charts" => Ok("Chart Parameters"),
            "exchanges" => Ok("Supported Exchanges"),
            "downloads" => Ok("Data Download Parameters"),
            _ => Err(ConfigFormError::InvalidKey) 
        }

    } 

}


// ------------- SYSTEM SETTINGS -------------- //
pub struct SettingsScreen {
    config_form: ConfigForm, 
}

impl SettingsScreen {

    pub fn new(app_config: &AppConfig) -> Self {
        SettingsScreen {
            config_form: ConfigForm::from_config(app_config),
        } 
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {

        fn divider_text(name: &String, row_width: u16) -> String {
            "".to_string() 
        }

        let block = Block::default()
            .title("System Settings")
            .borders(Borders::ALL);

        frame.render_widget(block.clone(), area);

        let inner = block.inner(area);

        let width: u16 = area.width.saturating_sub(2);

        let form_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(self.config_form.rows
                .iter()
                .map(|_| Constraint::Length(1))
                .collect::<Vec<Constraint>>()
            )
            .split(inner);

        for (i, row) in self.config_form.rows.iter().enumerate() {
        
            match row {
                FormRow::SectionDivider(s) => {
                    
                    let section_name = divider_text(s, width); 
                    frame.render_widget(
                        Paragraph::new(s.clone()),
                        form_rows[i] 
                    );
                
                },
                FormRow::InputRow(input_row) => {
                
                    let cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(30),
                            Constraint::Min(10)
                        ])
                        .split(form_rows[i]);
                    
                    frame.render_widget(
                        Paragraph::new(input_row.label.as_str()),
                        cols[0]
                    );

                    frame.render_widget(
                        Paragraph::new(input_row.value.as_str()),
                        cols[1]
                    );

                }
            };

        };

    }

    pub fn handle_key(&self, key: KeyEvent) {

    }

    pub const SCREEN_NAME: &'static str = "System Settings";

    pub const SCREEN_OPTIONS: [&'static str; 0] = [];

}



