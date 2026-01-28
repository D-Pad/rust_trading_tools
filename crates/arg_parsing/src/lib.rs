use std::env::args;


#[derive(Debug)]
pub struct ParsedArgs {
    pub executable_path: String,
    pub executable_name: String,
    pub command: String,

    // Unique commands 
    pub start_server: bool,

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

            parser_error: None
        }     
    }

    pub fn is_ok(self) -> bool {
        self.parser_error.is_none()
    }
}


#[derive(Debug)]
pub enum ParserError {
    UnknownFlags(Vec<String>)
}


pub fn parse_args() -> ParsedArgs {

    // Initialization
    let mut arguments: Vec<String> = args().collect();
    
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

    let mut flag_found = false;

    let mut invalid_flags: Vec<String> = Vec::new();
    
    for arg in arguments {
  
        // ------------------- Long flag parsing --------------------- //
        if is_long_flag(&arg) {
            flag_found = true;
                
            match &arg[2..] {
                "start_server" => parsed_args.start_server = true,
                _ => {
                    invalid_flags.push(arg.clone());
                }
            };

        }
        // ------------------- Short flag parsing -------------------- //
        else if is_short_flag(&arg) {
            flag_found = true;

            for ch in arg[1..].chars() {
             
                match ch {
                    's' => parsed_args.start_server = true,
                    _ => {
                        invalid_flags.push(ch.to_string()); 
                    }
                }

            };

        }
        // ------------------- Initial command --------------------- //
        else if &parsed_args.command == "" && !flag_found {
            parsed_args.command = arg;
        };

    }; 

    if invalid_flags.len() > 0 {
        parsed_args.parser_error = Some(
            ParserError::UnknownFlags(invalid_flags)
        )
    };

    println!("{:?}", parsed_args);
    parsed_args

}


