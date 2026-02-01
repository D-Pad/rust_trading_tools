use app_core::engine::{Engine};


pub struct CliServer {
    pub engine: Engine, 
}

impl CliServer {
    
    pub fn new(engine: Engine) -> Self {
        CliServer { engine }
    }

    pub fn start(&self) {
        println!("Running..."); 
    }

}


