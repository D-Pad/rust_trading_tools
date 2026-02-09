use std::collections::{BTreeMap};

use app_core::app_state::{
    AppConfig
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
pub struct ConfigField<'a> {
    pub label: &'a str,
    pub kind: FieldKind,
    pub value: String,
    pub cursor: usize,
}

/// A ConfigForm is intended to be used as a way for the user to interface
/// with the system settings, and make changes to it. Used in the TUI crate
#[derive(Debug)]
pub struct ConfigForm<'a> {
    pub focused: usize,
    pub fields: BTreeMap<&'a str, Vec<ConfigField<'a>>>,
}

impl<'a> ConfigForm<'a> {

    /// Takes an AppConfig reference and returns a ConfigForm
    ///
    /// Use this to build a  ConfigForm, to be used in a terminal user 
    /// interface. Intended to be used as a way for the user to edit system 
    /// settings from an interface.
    pub fn from_config(cfg: &'a AppConfig) -> Self {

        let mut supported_exchanges_vec: Vec<ConfigField> = Vec::new();

        for (exchange, enabled) in &cfg.supported_exchanges.active {
            supported_exchanges_vec.push(
                ConfigField {
                    label: exchange,
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
                    "backtest", 
                    Vec::from([
                        ConfigField {
                            label: "Inside Bar Testing",
                            kind: FieldKind::Bool,
                            value: cfg.backtesting.inside_bar.to_string(),
                            cursor: 0,
                        },
                    ])
                ),

                // Chart params 
                (
                    "charts",
                    Vec::from([
                        ConfigField {
                            label: "Max number of bars on chart",
                            kind: FieldKind::Number,
                            value: cfg.chart_parameters.num_bars.to_string(),
                            cursor: 0,
                        }
                    ])
                ), 

                (
                    "exchanges",
                    supported_exchanges_vec
                ),

                (
                    "downloads",
                    Vec::from([
                        ConfigField {
                            label: "Initial download cache size",
                            kind: FieldKind::Text,
                            value: format!(
                                "{}{}",
                                cfg.data_download.cache_size_units,
                                cfg.data_download.cache_size_period,
                            ),
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
    pub fn key_to_title(key: &str) -> Result<&'a str, ConfigFormError> {
 
        match key {
            "backtest" => Ok("Backtest Settings"),
            "charts" => Ok("Chart Parameters"),
            "exchanges" => Ok("Supported Exchanges"),
            "downloads" => Ok("Data Download Parameters"),
            _ => Err(ConfigFormError::InvalidKey) 
        }

    } 

}

