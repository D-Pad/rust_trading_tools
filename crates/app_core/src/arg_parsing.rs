use std::{collections::HashMap, env::args};


// --------------------------- COMMAND ENUMS ------------------------------- //
#[derive(Debug)]
pub enum Command {
    AddPair {
        exchange: String,
        pair: String
    },
    DropPair {
        exchange: String,
        pair: String
    },
    StartServer,
}

pub enum Response {
    Ok
}


// ----------------------------- STRUCTS ----------------------------------- //
#[derive(Debug)]
pub struct ParsedArgs {
    pub executable_path: String,
    pub executable_name: String,
    pub command: String,

    // Unique commands 
    pub start_server: bool,
    pub add_pairs: Option<HashMap<String, Vec<String>>>,
    pub remove_pairs: Option<HashMap<String, Vec<String>>>,

    // Errors
    pub parser_error: Option<ParserError>
}

impl ParsedArgs {
    
    fn new() -> Self {
        ParsedArgs {
            executable_path: String::new(),
            executable_name: String::new(),
            command: String::new(),

            start_server: false,
            add_pairs: None,
            remove_pairs: None,

            parser_error: None
        }     
    }

    pub fn is_ok(self) -> bool {
        self.parser_error.is_none()
    }

    pub fn to_commands(self) -> Vec<Command> {
        
        let mut commands: Vec<Command> = Vec::new();
        
        // Add pairs
        if let Some(additions) = self.add_pairs {
            for (exchange, pairs) in additions {
                for pair in pairs {
                    commands.push(Command::AddPair { 
                        exchange: exchange.clone(), 
                        pair: pair.clone() 
                    });
                }; 
            };
        };

        // Drop pairs
        if let Some(removals) = self.remove_pairs {
            for (exchange, pairs) in removals {
                for pair in pairs {
                    commands.push(Command::DropPair { 
                        exchange: exchange.clone(), 
                        pair: pair.clone() 
                    });
                }; 
            };
        };

        if self.start_server {
            commands.push(Command::StartServer);
        };

        commands

    }
}

impl std::fmt::Display for ParsedArgs {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\x1b[1;36mParsed Arguments: \x1b[0m\x1b[1m{{\x1b[0m")?;
        
        write!(f, "\n  \x1b[33mexecutable_path\x1b[0m: {}",
            self.executable_path)?;
        write!(f, "\n  \x1b[33mexecutable_name\x1b[0m: {}",
            self.executable_name)?;
        
        if self.command != "" { 
            write!(f, "\n  \x1b[33mcommand\x1b[0m: {}", self.command)?;
        };
        
        write!(f, "\n  \x1b[33mstart_server\x1b[0m: {}", self.start_server)?;
        write!(f, "\n  \x1b[33madd_pairs\x1b[0m: {:?}", self.add_pairs)?;
        write!(f, "\n  \x1b[33mremove_pairs\x1b[0m: {:?}", self.remove_pairs)?;
        write!(f, "\n  \x1b[33mparser_error\x1b[0m: {:?}\n\x1b[1m}}\x1b[0m", 
            self.parser_error)

    }
}


#[derive(Debug)]
pub enum ParserError {
    UnknownFlags(Vec<String>)
}


pub fn parse_args(passed_arguments: Option<Vec<String>>) -> ParsedArgs {

    // Initialization
    let mut arguments: Vec<String> = match passed_arguments {
        Some(a) => a, 
        None => args().collect()
    };
    
    let mut executable_path: String = String::new();   
    let mut executable_name: String = String::new();   
    
    let mut parsed_args: ParsedArgs = ParsedArgs::new();

    // Main program path and name
    if arguments.len() > 1 {
        executable_path = arguments.remove(0); 
        executable_name = arguments.remove(0);
    }
    else if arguments.len() > 0 {
        executable_path = arguments.remove(0);
    }
    else {
        return parsed_args;
    };

    parsed_args.executable_path = executable_path; 
    parsed_args.executable_name = executable_name;

    // Helper functions
    fn is_long_flag(arg: &str) -> bool {
        arg.len() >= 2 && arg.starts_with("--")
    }

    fn is_short_flag(arg: &str) -> bool {
        !is_long_flag(arg) && arg.starts_with("-") && arg.len() > 1
    }

    // Option tracking variables
    let mut invalid_flags: Vec<String> = Vec::new();
    let mut pairs_to_add: HashMap<String, Vec<String>> = HashMap::new();
    let mut pairs_to_rm: HashMap<String, Vec<String>> = HashMap::new();
    let mut exchange: String = String::new();
    let mut flag_name: String = String::new();

    let mut flag_found = false;
    let mut option_counter: u8 = 0;
    
    for arg in arguments {
  
        // ------------------- Long flag parsing --------------------- //
        if is_long_flag(&arg) {
           
            flag_found = true;
            option_counter = 0;
                
            match &arg[2..] {
                "start-server" => parsed_args.start_server = true,
                "add-pairs" | "rm-pairs" => flag_name = arg[2..].to_string(),
                _ => {
                    invalid_flags.push(arg.clone());
                }
            };

        }
        
        // ------------------- Short flag parsing -------------------- //
        else if is_short_flag(&arg) {
            
            flag_found = true;
            option_counter = 0;

            for ch in arg[1..].chars() {
             
                match ch {
                    's' => parsed_args.start_server = true,
                    'A' => flag_name = "add-pairs".to_string(),
                    'R' => flag_name = "rm-pairs".to_string(),
                    _ => {
                        invalid_flags.push(ch.to_string()); 
                    }
                }

            };
            
        }
        
        // ------------------- Initial command --------------------- //
        else if &parsed_args.command == "" && !flag_found {
            parsed_args.command = arg;
        }
        
        // ------------------- Option parsing ---------------------- //
        else {
            
            if flag_name == "add-pairs" || flag_name == "rm-pairs" {
                
                if option_counter == 0 {
                    exchange = arg.to_string();
                } 
                
                else {
                    if flag_name == "add-pairs" {
                        pairs_to_add.entry(exchange.to_string())
                            .or_insert(Vec::new())
                            .push(arg.to_string());
                    }
                    else if flag_name == "rm-pairs" {
                        pairs_to_rm.entry(exchange.to_string())
                            .or_insert(Vec::new())
                            .push(arg.to_string());
                    }
                }    
            };

            option_counter += 1;

        }

    }; 

    if invalid_flags.len() > 0 {
        parsed_args.parser_error = Some(
            ParserError::UnknownFlags(invalid_flags)
        )
    };

    if pairs_to_add.len() > 0 {
        parsed_args.add_pairs = Some(pairs_to_add);
    };

    if pairs_to_rm.len() > 0 {
        parsed_args.remove_pairs = Some(pairs_to_rm);
    };

    parsed_args

}


