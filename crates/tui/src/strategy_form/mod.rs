use app_core::SmaInputs;
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
    ParseError(StringParseError)
}

pub enum StringParseError {
    U16,
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

    pub fn get_form_rows(&self) -> Vec<FormRow<StrategyKeys>> {

        let mut rows: Vec<FormRow<StrategyKeys>> = Vec::new();

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
           
            let ma_types = MaInputs::MA_TYPES;

            rows.push(
                FormRow::InputRow(FormField {
                    label: "MA Type".to_string(),
                    kind: FieldKind::Select(SelectOption::new(
                        Vec::from(ma_types))), 
                    value: ma_types[0].to_string(),
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
                _ => {}
            }
        };

        rows

    }
 
    pub fn modify_from_form_field(&mut self, field: &FormField<StrategyKeys>
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
        };

        Ok(())

    }

}



// ---------------- STRATEGY FORM INPUT STRUCTS AND ENUMS ------------------ //
pub enum StrategyKeys {
    MovingAverage(MovingAverageKeys)
}

pub enum MovingAverageKeys {
    Enabled,
    MaType,
    Period,
    Phase,
    Power,
}



