use app_core::{
    engine::{Engine},
    arg_parsing::{Command},
    AppEvent
};

use tokio::sync::mpsc::Receiver;


pub struct CliServer {
    pub engine: Engine, 
    pub receiver: Receiver<AppEvent>,
}

impl CliServer {
    
    pub fn new(engine: Engine, receiver: Receiver<AppEvent>) -> Self {
        CliServer { engine, receiver }
    }

    pub async fn run(&mut self) {
   
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                AppEvent::Tick => {
                    println!("TICK");
                },
                AppEvent::Quit => {
                    break;
                },
                AppEvent::Input(cmd_string) => {
                    println!("EXECUTING: {}", cmd_string);
                }
            };
        }

    }

}


