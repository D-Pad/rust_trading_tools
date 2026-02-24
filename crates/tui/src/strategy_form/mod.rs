use strategies::{
    Strategy,
    MaInputs,
};
use crate::{
    FormField,
    FieldKind,
    FormRow,
};



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
                value: "false".to_string(),
                key: StrategyKeys::MovingAverage(MovingAverageKeys::Enabled)
            })
        );
        if let Some(ma) = &self.strategy.inputs.moving_average {
            match ma {
                MaInputs::SMA(inputs) => {
                    inputs;
                },
                _ => {}
            }
        };

        rows

    }
 
    pub fn modify_from_form_field(&mut self, field: &FormField<StrategyKeys>) {

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



