use std::collections::{BTreeMap};

use app_core::app_state::{
    AppConfig
};

use ratatui::{
    crossterm::{
        event::{
            KeyEvent,
        }
    },
};


// pub struct AppConfig {
//     pub backtesting: BackTestSettings,
//     pub supported_exchanges: SupportedExchanges,
//     pub data_download: DataDownload, 
//     pub chart_parameters: ChartParams,
// }
// 
// 
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BackTestSettings {
//     pub inside_bar: bool,
// } 
// 
// 
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ChartParams {
//     pub num_bars: u16,
// }
// 
// 
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SupportedExchanges {
//     pub active: HashMap<String, bool>,
// }
// 
// 
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct DataDownload {
//     pub cache_size_units: u64,
//     pub cache_size_period: char,
// }


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

/// A ConfigForm is intended to be used as a way for the user to interface
/// with the system settings, and make changes to it. Used in the TUI crate
#[derive(Debug)]
pub struct ConfigForm {
    pub focused: usize,
    pub fields: BTreeMap<String, Vec<ConfigField>>,
}

impl ConfigForm {

    /// Takes an AppConfig reference and returns a ConfigForm
    ///
    /// Use this to build a  ConfigForm, to be used in a terminal user 
    /// interface. Intended to be used as a way for the user to edit system 
    /// settings from an interface.
    pub fn from_config(cfg: &AppConfig) -> Self {

        let mut supported_exchanges_vec: Vec<ConfigField> = Vec::new();

        for (exchange, enabled) in &cfg.supported_exchanges.active {
            supported_exchanges_vec.push(
                ConfigField {
                    label: exchange.clone(),
                    kind: FieldKind::Bool,
                    value: enabled.to_string(),
                    cursor: 0,
                }
            ); 
        };

        ConfigForm {
            focused: 0,
            fields: BTreeMap::from([
               
                (
                    "backtest".to_string(), 
                    Vec::from([
                        ConfigField {
                            label: "Inside Bar Testing".to_string(),
                            kind: FieldKind::Bool,
                            value: cfg.backtesting.inside_bar.to_string(),
                            cursor: 0,
                        },
                    ])
                ),

                // Chart params 
                (
                    "charts".to_string(),
                    Vec::from([
                        ConfigField {
                            label: "Max number of bars on chart".to_string(),
                            kind: FieldKind::Number,
                            value: cfg.chart_parameters.num_bars.to_string(),
                            cursor: 0,
                        }
                    ])
                ), 

                (
                    "exchanges".to_string(),
                    supported_exchanges_vec
                ),

                (
                    "downloads".to_string(),
                    Vec::from([
                        ConfigField {
                            label: "Initial download cache size".to_string(),
                            kind: FieldKind::Text,
                            value: cfg.data_download.cache_size.clone(),
                            cursor: 0,
                        }
                    ])
                )

            ])
        }

    }

    /// Converts a key into a human readable string 
    ///
    /// Takes a ConfigForm field BTreeMap key and turns it into a 
    /// human readable title
    pub fn key_to_title(key: &String) -> Result<&'static str, ConfigFormError> {
 
        match &key[..] {
            "backtest" => Ok("Backtest Settings"),
            "charts" => Ok("Chart Parameters"),
            "exchanges" => Ok("Supported Exchanges"),
            "downloads" => Ok("Data Download Parameters"),
            _ => Err(ConfigFormError::InvalidKey) 
        }

    } 

}


// ------------- SYSTEM SETTINGS -------------- //
pub struct SettingsScreen {
    config_form: ConfigForm 
}

impl SettingsScreen {

    pub fn new(app_config: &AppConfig) -> Self {
        SettingsScreen {
            config_form: ConfigForm::from_config(app_config) 
        } 
    }

    pub fn draw(&mut self) {

    }

    pub fn handle_key(&self, key: KeyEvent) {

    }

    pub const SCREEN_NAME: &'static str = "System Settings";

    pub const SCREEN_OPTIONS: [&'static str; 0] = [];

}



