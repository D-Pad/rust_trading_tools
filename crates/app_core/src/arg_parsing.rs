use std::{env::args};
use bars::{BarSeries};


// --------------------------- COMMAND ENUMS ------------------------------- //
#[derive(Debug, Clone)]
pub enum Command {
    AddPair {
        exchange: String,
        ticker: String
    },
    DropPair {
        exchange: String,
        ticker: String
    },
    DbIntegrityCheck {
        exchange: String,
        ticker: String
    },
    StartServer,
    UpdatePairs,

    CandleBuilder {
        exchange: String,
        ticker: String,
        period: String,
        integrity_check: bool
    },
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::AddPair { exchange, ticker } => {
                write!(f, "AddPair: {}-{}", exchange, ticker)
            },
            Command::DropPair { exchange, ticker } => {
                write!(f, "DropPair: {}-{}", exchange, ticker)
            },
            Command::StartServer => {
                write!(f, "StartServer")
            },
            Command::UpdatePairs => {
                write!(f, "UpdatePairs")
            },
            Command::CandleBuilder { 
                exchange, ticker, period, integrity_check 
            } => {
                write!(f, 
                    "CandleBuilder: {} {} {} {}", 
                    exchange, 
                    ticker, 
                    period,
                    integrity_check
                )
            },
            Command::DbIntegrityCheck { exchange, ticker } => {
                write!(f, "DbIntegrityCheck: {} {}", exchange, ticker)
            },
        }
    }
}

pub enum DataResponse {
    Bars(BarSeries),
}

pub enum Response {
    Ok,
    Data(DataResponse),
}


// ----------------------------- STRUCTS ----------------------------------- //
pub struct ParsedArgs {
    pub executable_name: String,
    pub commands: Vec<Command>,
    pub parser_error: Option<ParserError>
}

impl ParsedArgs {
    
    fn new() -> Self {
        
        ParsedArgs {
            executable_name: String::new(),
            commands: Vec::new(),
            parser_error: None
        }     
    
    }

    pub fn is_ok(self) -> bool {
        self.parser_error.is_none()
    }
}

impl std::fmt::Display for ParsedArgs {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\x1b[1;36mParsed Arguments: \x1b[0m\x1b[1m{{\x1b[0m")?;
        
        // write!(f, "\n  \x1b[33mexecutable_path\x1b[0m: {}",
        //     self.executable_path)?;
        write!(f, "\n  \x1b[33mexecutable_name\x1b[0m: {}",
            self.executable_name)?;
        
        write!(f, "\n  \x1b[33mcommands\x1b[0m: [")?;
        for cmd in &self.commands {
            write!(f, "\n    {}", cmd)?;
        };
        write!(f, "\n  ]")?;
        
        write!(f, "\n  \x1b[33mparser_error\x1b[0m: {:?}\n\x1b[1m}}\x1b[0m", 
            self.parser_error)

    }
}


#[derive(Debug)]
pub enum ParserError {
    UnknownCommand(String),
    UnknownArg(String),
    UnknownFlags(Vec<String>),
    TooManyArgs(String),
    MissingArgs(String),
}


pub fn parse_args(passed_arguments: Option<Vec<String>>) -> ParsedArgs {

    // Initialization
    let mut arguments: Vec<String> = match passed_arguments {
        Some(a) => a, 
        None => args().skip(1).collect()
    };
    
    let executable_name: String;   
    
    let mut parsed_args: ParsedArgs = ParsedArgs::new();

    // Main program path and name
    if arguments.len() > 0 {
        executable_name = arguments.remove(0);
    }
    else {
        return parsed_args;
    };

    parsed_args.executable_name = executable_name;

    // Helper functions
    fn is_long_flag(arg: &str) -> bool {
        arg.len() >= 2 && arg.starts_with("--")
    }

    fn is_short_flag(arg: &str) -> bool {
        !is_long_flag(arg) && arg.starts_with("-") && arg.len() > 1
    }

    fn is_flag(arg: &str) -> bool {
        is_long_flag(arg) || is_short_flag(arg)
    }

    let mut command_buffer: Vec<String> = Vec::new();
    let mut op_mode: &String = &String::new();
    let mut flag_name: &String = &String::new();
    let mut unknown_flags: Vec<String> = Vec::new();

    // Specific option variables
    let mut exchange: String = String::new();
    let mut db_int_check_name: String = "all".to_string(); 
    let mut db_int_check_ticker: String = "all".to_string(); 
    let mut db_int_check: bool = false;

    for (i, arg) in arguments.iter().enumerate() {
     
        if i == 0 && !is_flag(&arg) {
            op_mode = arg;
        }
        
        else if op_mode != "" {

            match &op_mode[..] {
                
                "database" => {
                    if is_flag(arg) {
                        flag_name = arg;
                        exchange = String::new();
                        
                        if flag_name == "--update" {
                            parsed_args.commands.push(
                                Command::UpdatePairs
                            );                               
                        }
                        else if flag_name == "--integrity" {
                            db_int_check = true; 
                        };
                    }
                    else {  // Flag option parsing
                        
                        if flag_name == "--add-pairs" 
                        || flag_name == "--rm-pairs" {
                            
                            if exchange == "" {
                                match &arg[..] {
                                    "kraken" 
                                    // | other exchanges here
                                    => {
                                        exchange = arg.to_string();
                                    },
                                    _ => {
                                        parsed_args.parser_error = Some(
                                            ParserError::UnknownArg(
                                                format!(
                                                    "Invalid exchange: {}",
                                                    arg
                                                ) 
                                            ) 
                                        );
                                        return parsed_args
                                    }
                                }
                            } 
                            else {

                                if flag_name == "--add-pairs" {
                                    parsed_args.commands.push(
                                        Command::AddPair { 
                                            exchange: exchange.clone(), 
                                            ticker: arg.to_string() 
                                        }
                                    );
                                }
                                else if flag_name == "--rm-pairs" {
                                    parsed_args.commands.push(
                                        Command::DropPair { 
                                            exchange: exchange.clone(), 
                                            ticker: arg.to_string() 
                                        }
                                    );
                                };
                            }
                        }

                        else if flag_name == "--integrity" {
                            if db_int_check_name == "all" {
                                db_int_check_name = arg.to_string(); 
                            }
                            else if db_int_check_ticker == "all" {
                                db_int_check_ticker = arg.to_string(); 
                            };
                        }

                        else {
                            unknown_flags.push(
                                format!(
                                    "Invalid flag: {}",
                                    arg
                                )
                            ); 
                        }
                    }; 
                },

                "candles" => {
                    
                    if !is_flag(&arg) {
                        command_buffer.push(arg.to_string());
                    }
                    else if command_buffer.len() == 3 && is_flag(&arg) {
                        command_buffer.push(arg.to_string());
                    };

                },
                
                _ => {}
            }

        };
 
    }; 

    if unknown_flags.len() > 0 {
        parsed_args.parser_error = Some(
            ParserError::UnknownFlags(unknown_flags)
        )
    };

    match &op_mode[..] {
        "candles" => {

            let ex = command_buffer.remove(0);
            let sym = command_buffer.remove(0);
            let p = command_buffer.remove(0);
            let int_check = match command_buffer.len() {
                1 => {
                    let opt = command_buffer.remove(0);
                    if opt == "--integrity" || opt == "-i" {
                        true 
                    }
                    else {
                        false
                    }
                }, 
                _ => false 
            };

            parsed_args.commands.push(
                Command::CandleBuilder { 
                    exchange: ex, 
                    ticker: sym, 
                    period: p, 
                    integrity_check: int_check 
                }
            );
        },

        "database" => {
            if db_int_check {
                parsed_args.commands.push(
                    Command::DbIntegrityCheck { 
                        exchange: db_int_check_name, 
                        ticker: db_int_check_ticker 
                    }
                );
            };
        },

        "start" => {
            parsed_args.commands.push(Command::StartServer);
        },

        _ => {}
    };

    parsed_args

}


