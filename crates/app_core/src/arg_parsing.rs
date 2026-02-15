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
    UpdatePairs,
    
    StartServer {
        http: bool
    },

    CandleBuilder {
        exchange: String,
        ticker: String,
        period: String,
        integrity_check: bool
    },

    Help,
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
            Command::StartServer { http } => {
                if *http {
                    write!(f, "StartServer: HTTP")
                }
                else {
                    write!(f, "StartServer: TUI")
                }
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
            Command::Help => {
                write!(f, "Help")
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
/// Argument Parser
///
/// Takes command line arguments and parses them into Command types, to be 
/// executed by the Engine.
pub struct ParsedArgs {
    pub commands: Vec<Command>,
    pub parser_error: Option<ParserError>,
    pub dev_mode: bool,
}

impl ParsedArgs {
    
    fn new() -> Self {
        
        ParsedArgs {
            commands: Vec::new(),
            parser_error: None,
            dev_mode: false,
        }     
    
    }

    pub fn is_ok(self) -> bool {
        self.parser_error.is_none()
    }
}

impl std::fmt::Display for ParsedArgs {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\x1b[1;36mParsed Arguments: \x1b[0m\x1b[1m{{\x1b[0m")?;
        
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

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::UnknownCommand(e) => {
                write!(f, "UnknownCommand: {}", e)
            },
            ParserError::UnknownArg(e) => {
                write!(f, "UnknownArg: {}", e)
            },
            ParserError::UnknownFlags(e) => {
                write!(f, "UnknownFlags: {:?}", e)
            },
            ParserError::TooManyArgs(e) => {
                write!(f, "TooManyArgs: {:?}", e)
            },
            ParserError::MissingArgs(e) => {
                write!(f, "MissingArgs: {:?}", e)
            },
        }
    }
}

const ARG_ERROR: &'static str = { 
    "\x1b[1;31mInvalid command: try --help for all options\x1b[0m"
};

/// Parses command line arguments into a ParsedArgs struct 
///
/// If 'None' is passed in as the argument, then commands are taken from 
/// std::env::args(). Otherwise, pass Some(a) where a is a vector of string 
/// values. A ParsedArgs struct is always returned no matter what. If any 
/// arguments were invalid, then `ParsedArgs.parser_error` will contain 
/// a specific error showing what went wrong. If `parser_error` is None,
/// then the argument parsing was successful.
pub fn parse_args(passed_arguments: Option<Vec<String>>) -> ParsedArgs {

    // Initialization
    let mut arguments: Vec<String> = match passed_arguments {
        Some(a) => a, 
        None => args().skip(2).collect()
    };
    
    let mut parsed_args: ParsedArgs = ParsedArgs::new();

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
    let mut server_start_http_mode: bool = false;

    if arguments.len() == 0 {
        println!("{ARG_ERROR}");
    };

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

                "start" => {

                    if arg == "--http" {
                        server_start_http_mode = true;
                    };

                },

                _ => {}
            }

        }

        else if is_flag(&arg) {
            
            if arg.len() < 2 { continue };
            
            match &arg[2..] {
                "help" => parsed_args.commands.push(Command::Help),
                "dev" => parsed_args.dev_mode = true,
                _ => { println!("{ARG_ERROR}") }
            }        
        }

        else {
            println!("{ARG_ERROR}");
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
            parsed_args.commands.push(Command::StartServer {
                http: server_start_http_mode
            });
        },

        _ => {}
    };

    parsed_args

}


