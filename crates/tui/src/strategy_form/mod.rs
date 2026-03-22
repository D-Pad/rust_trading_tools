use app_core::{
    SmaInputs,
    EmaInputs
};
use strategies::{
    Strategy,
    MaInputs,
};
use crate::{
    FormField,
    FieldKind,
    FormRow,
    SelectOption,
};


pub enum ConstructorError {
    ParseError(StringParseError),
    InvalidType(String),
}


pub enum StringParseError {
    U16,
    Bool,
}


pub struct StrategyConstructor {
    pub strategy: Strategy,
}

impl StrategyConstructor {

    pub fn new() -> Self {
        Self {
            strategy: Strategy::empty()
        }
    }

    pub fn get_form_rows(&self, is_new: bool) -> Vec<FormRow<StrategyKeys>> {

        let mut rows: Vec<FormRow<StrategyKeys>> = Vec::new();

        // ---------------------- General Section ---------------------- //
        rows.push(FormRow::SectionDivider(
            "General".to_string()
        ));

        if is_new {
            rows.push(FormRow::InputRow(
                FormField { 
                    label: "Strategy Name".to_string(), 
                    kind: FieldKind::Text, 
                    value: self.strategy.general_settings.name.clone(), 
                    key: StrategyKeys::General(GeneralSettings::Name) 
                }
            ));
        }

        rows.push(FormRow::InputRow(
            FormField { 
                label: "Inside Bar Strategy".to_string(), 
                kind: FieldKind::Bool, 
                value: match &self.strategy.general_settings.inside_bar {
                    true => "true".to_string(),
                    false => "false".to_string()
                }, 
                key: StrategyKeys::General(GeneralSettings::InsideBar) 
            }
        ));

        // ----------------------- Moving Average --------------------- //
        rows.push(FormRow::SectionDivider(
            "Moving Average".to_string()
        ));
        
        rows.push(
            FormRow::InputRow(FormField {
                label: "Enabled".to_string(),
                kind: FieldKind::Bool,
                value: match &self.strategy.inputs.moving_average.is_some() {
                    true => "true".to_string(),
                    false => "false".to_string()
                },
                key: StrategyKeys::MovingAverage(MovingAverageKeys::Enabled)
            })
        );
        
        if let Some(ma) = &self.strategy.inputs.moving_average {
           
            let ma_type = ma.get_id();
            let options = Vec::from(MaInputs::MA_TYPES);
            let selected = options 
                .iter()
                .position(|&opt| opt == ma_type)
                .unwrap_or(0);

            rows.push(
                FormRow::InputRow(FormField {
                    label: "MA Type".to_string(),
                    kind: FieldKind::Select(SelectOption {
                        selected,
                        options,
                    }), 
                    value: ma_type.to_string(),
                    key: StrategyKeys::MovingAverage(
                        MovingAverageKeys::MaType
                    )
                })
            );
            
            match ma {

                MaInputs::SMA(inputs) => {
                    rows.push(
                        FormRow::InputRow(FormField {
                            label: "Period".to_string(),
                            kind: FieldKind::Integer, 
                            value: format!("{}", inputs.period),
                            key: StrategyKeys::MovingAverage(
                                MovingAverageKeys::Period
                            )
                        })
                    );
                },

                MaInputs::EMA(inputs) => {
                    rows.push(
                        FormRow::InputRow(FormField {
                            label: "Period".to_string(),
                            kind: FieldKind::Integer, 
                            value: format!("{}", inputs.period),
                            key: StrategyKeys::MovingAverage(
                                MovingAverageKeys::Period
                            )
                        })
                    );
                },

                MaInputs::JMA(inputs) => {
                    rows.push(
                        FormRow::InputRow(FormField {
                            label: "Period".to_string(),
                            kind: FieldKind::Integer, 
                            value: format!("{}", inputs.period),
                            key: StrategyKeys::MovingAverage(
                                MovingAverageKeys::Period
                            )
                        })
                    );
                },

            }
        };

        rows

    }
 
    pub fn modify_from_form_field(
        &mut self, 
        field: &FormField<StrategyKeys>
    ) -> Result<(), ConstructorError> {

        let inputs = &mut self.strategy.inputs;

        match &field.key {
            
            StrategyKeys::MovingAverage(ma_key) => {
                
                match ma_key {
                    
                    MovingAverageKeys::Enabled => {
                        if inputs.moving_average.is_none() {
                            inputs.moving_average = Some(MaInputs::SMA(
                                SmaInputs::default()
                            ));
                        }
                        else {
                            inputs.moving_average = None;
                        }
                    },

                    MovingAverageKeys::MaType => {
                       
                        let ma: MaInputs;

                        if let Some(s) = &self.strategy.inputs.moving_average {
                            
                            let p = s.get_period();
                            let source = s.get_source();

                            println!("{}", field.value);
                            if field.value == "sma" {
                                ma = MaInputs::SMA(SmaInputs::new(
                                    p, Some(source)
                                ));
                            }
                            else if field.value == "ema" {
                                ma = MaInputs::EMA(EmaInputs::new(
                                    p, Some(source)
                                ));
                            }
                            else {
                                return Err(ConstructorError::InvalidType(
                                    field.value.to_string() 
                                ))
                            }

                            self.strategy.inputs.moving_average = Some(ma);

                        }
                    
                    },

                    MovingAverageKeys::Period => {
                        if let Some(ref mut ma_inputs) = inputs.moving_average {
                            let period = field.value.parse::<u16>()
                                .map_err(|_| ConstructorError::ParseError(
                                    StringParseError::U16 
                                ))?; 
                            ma_inputs.set_period(period); 
                        };
                    },

                    MovingAverageKeys::Phase => {},

                    MovingAverageKeys::Power => {},

                };
            },

            StrategyKeys::General(gen_settings) => {
                
                match gen_settings {
                    
                    GeneralSettings::Name => {
                        self.strategy
                            .general_settings
                            .name = field.value.clone();
                    },

                    GeneralSettings::InsideBar => {
                        let old: bool = field.value.parse::<bool>()
                            .map_err(|_| ConstructorError::ParseError(
                                StringParseError::Bool
                            ))?;
                        self.strategy
                            .general_settings
                            .inside_bar = !old;
                    }
                
                }
            
            }
        };

        Ok(())

    }

}



// ---------------- STRATEGY FORM INPUT STRUCTS AND ENUMS ------------------ //
pub enum StrategyKeys {
    General(GeneralSettings),
    MovingAverage(MovingAverageKeys),
}

pub enum GeneralSettings {
    Name,
    InsideBar,
}

pub enum MovingAverageKeys {
    Enabled,
    MaType,
    Period,
    Phase,
    Power,
}


