use std::collections::{BTreeMap};

use app_core::app_state::{
    AppConfig,
    BackTestSettings,
    ChartParams,
    SupportedExchanges,
    DataDownload,
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

#[derive(Debug)]
pub struct ConfigField<'a> {
    pub label: &'a str,
    pub kind: FieldKind,
    pub value: String,
    pub cursor: usize,
}

#[derive(Debug)]
pub struct ConfigForm<'a> {
    pub focused: usize,
    pub fields: BTreeMap<&'a str, Vec<ConfigField<'a>>>,
}

impl<'a> ConfigForm<'a> {

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
                    "Backtest Settings", 
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
                    "Chart Parameters",
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
                    "Supported Exchanges",
                    supported_exchanges_vec
                ),

                (
                    "Data Download Parameters",
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

}

