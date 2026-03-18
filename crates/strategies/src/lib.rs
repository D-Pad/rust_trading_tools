use std::{
    fs,
    fmt::{Display, Formatter, self},
    path::PathBuf
};

pub use indicators::{self, *};
use config::SystemPaths;

use serde::{Serialize, Deserialize};


pub enum StrategyError {
    MaInput(MaError),
    FileNotFound,
    DeleteFailed,
    ExportFailed(&'static str),
    ImportFailed,
    LookupFailed,
}


// ------------------------------- STRATEGY -------------------------------- //
/// # Trading Strategy
///
/// A trading strategy stores a group of indicators, and uses their 
/// calculations to determine entry and exit trading signals.
#[derive(Serialize, Deserialize)]
pub struct Strategy {
    pub general_settings: GeneralStrategySettings,
    pub inputs: StrategyInputs
}

impl Strategy {
    
    pub fn new(name: String) -> Self {
        Self { 
            general_settings: GeneralStrategySettings::new(name), 
            inputs: StrategyInputs::empty()
        }
    }

    pub fn empty() -> Self {
        Self { 
            general_settings: GeneralStrategySettings::new(String::new()), 
            inputs: StrategyInputs::empty()
        }
    }

    pub fn export(&self, modifying: bool) -> Result<(), StrategyError> {
        export_strategy_template(self, modifying) 
    }

}


// -------------------------- TEMPLATE RENDERING --------------------------- //
/// # Strategy Component 
///
/// Used to describe a strategy component. Can be used in the 
/// StrategyInputs.add_new_default_component() method to add an indicator to 
/// a strategy.
pub enum StrategyComponentType {
    MA,
}


#[derive(Serialize, Deserialize)]
/// # General Strategy Settings
///
/// A struct for managing high level strategy input values, outside of normal
/// indicator input values.
pub struct GeneralStrategySettings {
    pub name: String,
    pub inside_bar: bool,
}

impl GeneralStrategySettings {
    pub fn new(name: String) -> Self {
        GeneralStrategySettings { 
            name, 
            inside_bar: true 
        }
    }
}


#[derive(Serialize, Deserialize)]
/// # Strategy Input Rendering
///
/// Used for creating and saving strategy templates for later use. This struct 
/// only contains the inputs needed to initialize a Strategy, and does not 
/// perform any indicator calculations on it's own. See the 
/// 'add_new_default_component' method to see how components should be added
/// to a strategy template.
pub struct StrategyInputs {
    pub moving_average: Option<MaInputs>
}

impl StrategyInputs {

    pub fn new(
        moving_average: Option<MaInputs>
    ) -> Self {
        
        Self {
            moving_average
        }
    
    }

    pub fn empty() -> Self {
        
        Self {
            moving_average: None
        }
    
    }

    /// ## Add Default Indicator Component
    ///
    /// New indicator components can be added by passing a 
    /// StrategyComponentType struct into this method, which is used to 
    /// generate a new strategy template, but shouldn't have much control over 
    /// the input values of the strategy. In other words, the default input 
    /// values are rendered by default, and should be changed later via the 
    /// TUI or web server.
    ///
    /// To add a component, a StrategyComponentType must be initialized first.
    /// ```
    /// let mut strat = Strategy::empty("My Strategy".to_string());
    /// let comp = StrategyComponentType::MA { ma_type: "sma" };
    /// strat.inputs.add_new_default_component(comp);
    /// ```
    pub fn add_new_default_component(&mut self, comp: StrategyComponentType) 
        -> Result<(), StrategyError> {
        
        match comp {
            StrategyComponentType::MA => {
                if let None = &self.moving_average {
                    self.moving_average = Some(
                        MaInputs::SMA(indicators::SmaInputs::default())
                    ); 
                };
            }
        }

        Ok(())
    }

}

impl Display for StrategyInputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.moving_average {
            Some(v) => {
                write!(f, "Moving Average:\n")?;
                write!(f, "{}", match v {
                    MaInputs::SMA(sma) => format!(
                        "  SMA: {{ period: {}, source: {} }}",
                        sma.period,
                        sma.source
                    ),
                    MaInputs::EMA(ema) => format!(
                        "  EMA: {{ period: {}, source: {} }}",
                        ema.period,
                        ema.source
                    )
                })?;
                Ok(())
            },
            None => write!(f, "None")
        }
    }    
}

// -------------------------- IMPORT & EXPORT ------------------------------ //
pub fn fetch_available_templates() 
    -> Result<Vec<String>, StrategyError> {

    let sys_paths: SystemPaths = SystemPaths::new()
        .map_err(|_| StrategyError::LookupFailed)?;

    let files: Vec<String> = fs::read_dir(sys_paths.strategy_templates) 
        .map_err(|_| StrategyError::LookupFailed)? 
        .filter_map(|res| res.ok())
        .filter_map(|e| {
            e.path()
                .file_stem()?
                .to_str()?
                .to_owned()
                .into()
        })
        .collect();

    Ok(files)
}

pub fn export_strategy_template(strategy: &Strategy, modifying: bool) 
    -> Result<(), StrategyError> {

    let sys_paths: SystemPaths = SystemPaths::new()
        .map_err(|_| StrategyError::ExportFailed(
            "Failed to initialize system paths"
        ))?;

    let mut file_name: String = strategy
        .general_settings
        .name
        .replace(" ", "_")
        .to_lowercase();
    
    let num_chars: usize = file_name.len();  
 
    if file_name == "" {
        return Err(StrategyError::ExportFailed(
            "Strategy name not provided"
        ))
    }

    if num_chars <= 5 || 
        (num_chars > 5 && &file_name[&num_chars - 5..] != ".json") 
    {
        file_name.push_str(".json");
    };
    let file_path = sys_paths.strategy_templates.join(file_name);

    if let Ok(o) = serde_json::to_string_pretty(&strategy) {
      
        let exists: bool = file_path.exists();

        if !exists || (exists && modifying) {
            fs::write(file_path, o)
                .map_err(|_| StrategyError::ExportFailed(
                    "Failed to export strategy template"
                ))?;
            
            Ok(())
        }
        else {
            Err(StrategyError::ExportFailed(
                "Strategy template already exists. Try a different name"
            ))
        }

    }
    else {
        Err(StrategyError::ExportFailed(
            "Failed to convert strategy template to json format"
        ))
    }

}


pub fn load_strategy_template (strategy_name: &str) 
    -> Result<Strategy, StrategyError> {

    let sys_paths: SystemPaths = SystemPaths::new()
        .map_err(|_| StrategyError::ImportFailed)?;

    let mut file_name = strategy_name.to_lowercase();
    let num_chars: usize = file_name.len();  
    
    if num_chars <= 5 || 
        (num_chars > 5 && &file_name[&num_chars - 5..] != ".json") 
    {
        file_name.push_str(".json");
    }; 

    let expected_path: PathBuf = sys_paths.strategy_templates.join(file_name);
    
    if expected_path.exists() {
       
        let json = fs::read_to_string(&expected_path)
            .map_err(|_| StrategyError::ImportFailed)?;

        let inputs = serde_json::from_str::<Strategy>(&json)
            .map_err(|_| StrategyError::ImportFailed)?;
        
        Ok(inputs)
    
    }
    
    else {
        Err(StrategyError::FileNotFound)
    }

}


pub fn delete_strategy(strategy_name: &str) 
    -> Result<(), StrategyError> {

    let sys_paths: SystemPaths = SystemPaths::new()
        .map_err(|_| StrategyError::ImportFailed)?;

    let mut file_name = strategy_name.to_lowercase();
    if !file_name.contains(".json") {
        file_name.push_str(".json");
    }; 

    let expected_path: PathBuf = sys_paths.strategy_templates.join(file_name);
    
    if expected_path.exists() {
        fs::remove_file(expected_path)
            .map_err(|_| StrategyError::DeleteFailed)?;
        Ok(())
    }
    
    else {
        Err(StrategyError::DeleteFailed)
    }

}

