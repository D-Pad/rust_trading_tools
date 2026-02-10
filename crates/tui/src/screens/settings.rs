use app_core::app_state::{
    AppConfig
};
use string_helpers::capitlize_first_letter;

use ratatui::{
    Frame,
    crossterm::{
        event::{
            KeyEvent,
            KeyCode
        },
    },
    style::{
        Modifier,
        Style,
    },
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


#[derive(Clone)]
pub enum FieldKind {
    Bool,
    Number,
    Text
}

pub enum ConfigFormError {
    InvalidKey
}

#[derive(Clone)]
pub struct ConfigField {
    pub label: String,
    pub kind: FieldKind,
    pub value: String,
}

pub enum FormRow {
    SectionDivider(String),
    InputRow(ConfigField),
}

pub enum FormMode {
    Movement,
    Input
}

/// A ConfigForm is intended to be used as a way for the user to interface
/// with the system settings, and make changes to it. Used in the TUI crate
pub struct ConfigForm {
    pub focused: usize,
    pub rows: Vec<FormRow>,
    pub mode: FormMode,
}

impl ConfigForm {

    /// Takes an AppConfig reference and returns a ConfigForm
    ///
    /// Use this to build a  ConfigForm, to be used in a terminal user 
    /// interface. Intended to be used as a way for the user to edit system 
    /// settings from an interface.
    pub fn from_config(cfg: &AppConfig) -> Self {

        let mut rows: Vec<FormRow> = Vec::new();
        let mode: FormMode = FormMode::Movement;           

        rows.push(FormRow::SectionDivider(
            "Backtest Settings".to_string()
        ));
        rows.push(FormRow::InputRow(
            ConfigField {
                label: "Inside Bar Testing".to_string(),
                kind: FieldKind::Bool,
                value: cfg.backtesting.inside_bar.to_string(),
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
            })
        );
        rows.push(FormRow::InputRow(
            ConfigField {
                label: "Logarithmic scale".to_string(),
                kind: FieldKind::Bool,
                value: cfg.chart_parameters.log_scale.to_string(),
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
            })
        );

        ConfigForm {
            focused: 1,
            rows,
            mode,
        }

    }

}


// ------------- SYSTEM SETTINGS -------------- //
pub struct SettingsScreen {
    pub config_form: ConfigForm,
    pub active: bool,
    pub previous_value: Option<String>
}

impl SettingsScreen {

    pub fn new(app_config: &AppConfig) -> Self {
        SettingsScreen {
            config_form: ConfigForm::from_config(app_config),
            active: true,
            previous_value: None
        } 
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {

        // fn divider_text(name: &String, row_width: usize) -> String {
        //   
        //     let sym = "—";

        //     let num_dashes: usize = row_width - name.len();
        //     let dashes = sym.repeat((num_dashes / 2) - 1);
        //     
        //     let mut div = format!("{} {} {}", dashes, name, dashes);
        //     
        //     let width_diff: usize = row_width.saturating_sub(div.len());
        //     if width_diff > 0 {
        //         div.push_str(&sym.repeat(width_diff).as_str())
        //     };

        //     div
        // }

        // let width: usize = area.width.saturating_sub(2) as usize;
        
        let block = Block::default()
            .title("System Settings")
            .borders(Borders::ALL);

        frame.render_widget(block.clone(), area);

        let inner = block.inner(area);

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
                    
                    // let section_name = divider_text(s, width); 
                    frame.render_widget(
                        Paragraph::new(format!("[{}]", s))
                            .style(Style::new().red()),
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
                   
                    let label = Paragraph::new(
                        format!(" {}:", input_row.label.as_str())
                    );
                    frame.render_widget(
                        if self.config_form.focused == i && self.active {
                            label.style(
                                Style::default()
                                    .yellow()
                                    .underlined()
                            )
                        }
                        else {
                            label
                        },
                        cols[0]
                    );

                    let input = Paragraph::new(
                        format!(":{}", input_row.value.as_str())
                    );
                    frame.render_widget(
                        if self.config_form.focused == i && self.active {
                            let mut input_style = Style::default()
                                .green()
                                .underlined();
                            if let FormMode::Input = self.config_form.mode {
                                input_style = input_style.add_modifier(
                                    Modifier::REVERSED
                                );
                            };
                            input.style(input_style)
                        }
                        else {
                            input 
                        },
                        cols[1]
                    );
                }
            };
        };
    }

    pub fn handle_key(&mut self, key: KeyEvent) {

        if let FormMode::Movement = self.config_form.mode {

            match key.code {
            
                KeyCode::Up | KeyCode::Char('k') => {
                   
                    let step: usize = {
                        
                        let min_i = 1;
                        let target = self.config_form.focused - 1;
                        let next_row = &self.config_form.rows[target];

                        match next_row {
                            FormRow::SectionDivider(_) => {
                                if target > min_i { 2 }
                                else { 0 }  // We're at the top
                            },
                            FormRow::InputRow(_) => 1
                        }
                    };

                    self.config_form.focused -= step;
                }, 
                
                KeyCode::Down | KeyCode::Char('j') => {
                    
                    let max_i = self.config_form.rows.len() - 1;
                    let target = self.config_form.focused + 1;
                    
                    if target < max_i {
                    
                        let next_row = &self.config_form.rows[target];

                        let step = match next_row {
                            FormRow::SectionDivider(_) => {
                                2 
                            },
                            FormRow::InputRow(_) => {
                                1
                            }
                        };
                        self.config_form.focused += step;
                    };
                },
                
                KeyCode::Enter => {

                    let i = self.config_form.focused;
                    let selected = &self.config_form.rows[i];

                    if let FormRow::InputRow(r) = selected {

                        let mut new_row = r.clone();
                        
                        match r.kind {

                            FieldKind::Bool => { 
                                
                                if r.value == "true" {
                                    new_row.value = "false".to_string();
                                }  
                                else if r.value == "false" {
                                    new_row.value = "true".to_string();
                                };
                                
                                self.config_form.rows[i] = FormRow::InputRow(
                                    new_row
                                );
                            },
                            
                            _ => { 
                               
                                let mode = &self.config_form.mode;
                                
                                self.config_form.mode = match mode {
                                    
                                    FormMode::Movement => {
                                        
                                        self.previous_value = Some(
                                            r.value.clone()
                                        );
                                        
                                        new_row.value = "".to_string();
                                        
                                        self.config_form
                                            .rows[i] = FormRow::InputRow(
                                                new_row
                                            );
                                        
                                        FormMode::Input
                                    },

                                    FormMode::Input => {
                                        FormMode::Movement
                                    }, 
                                }
                            }
                        }

                    };

                },

                KeyCode::Esc => {
                    
                    if matches!(self.config_form.mode, FormMode::Input) {
                        self.config_form.mode = FormMode::Movement; 
                    };
                
                } 

                _ => {}
            }

        }
        else if let FormMode::Input = &self.config_form.mode {

            let i = self.config_form.focused;
            
            match key.code {
                
                KeyCode::Char(c) => {
                    if let FormRow::InputRow(r) = &self.config_form.rows[i] {
                        let mut new_row = r.clone();
                        new_row.value.push(c);
                        self.config_form.rows[i] = FormRow::InputRow(new_row);
                    };
                },
                
                KeyCode::Enter => {
                    self.config_form.mode = FormMode::Movement;
                    self.previous_value = None;
                },
                
                KeyCode::Esc => {
                    if let FormRow::InputRow(r) = &self.config_form.rows[i] {
                        let mut new_row = r.clone();
                        if let Some(s) = &self.previous_value {
                            new_row.value = s.clone(); 
                        };
                        self.config_form.rows[i] = FormRow::InputRow(new_row);
                    };
                    self.config_form.mode = FormMode::Movement;
                    self.previous_value = None;
                },
               
                KeyCode::Backspace => {
                    if let FormRow::InputRow(r) = &self.config_form.rows[i] {
                        let mut new_row = r.clone();
                        let existing = new_row.value.clone();
                        let next_string: String = new_row
                            .value[..existing.len().saturating_sub(1)]
                            .to_string();
                        new_row.value = next_string;
                        self.config_form.rows[i] = FormRow::InputRow(new_row);
                    };                    
                },

                _ => {}
            }

        }
    }

    pub const SCREEN_NAME: &'static str = "System Settings";

    pub const SCREEN_OPTIONS: [&'static str; 0] = [];

}



