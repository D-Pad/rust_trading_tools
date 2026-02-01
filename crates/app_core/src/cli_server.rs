use crate::engine::{Engine};


pub struct Server {
    pub engine: Engine, 
    pub cli_server: bool,
}

impl Server {
    pub fn new(engine: Engine, cli_server: bool) -> Self {
        Server { engine, cli_server }
    }

    pub fn start(&self) {
        if self.cli_server {
            println!("RUNNING IN CLI MODE");
        }
        else {
            println!("RUNNING IN HTTP MODE");
        }; 
    }
}


