use database_ops::*;

use crate::app_state::AppState;
use crate::command_structs::{Command, Response};
use crate::errors::{RunTimeError};


pub struct Engine {
    pub state: AppState,
    pub database: Db,
}

impl Engine {
   
    pub fn new(database: Db) -> Result<Self, RunTimeError> {

        let state: AppState = AppState::new()
            .map_err(|e| RunTimeError::Init(e))?;

        Ok(Engine { state, database })

    }

    // pub async fn handle(&mut self, cmd: Command) 
    // -> Result<Response, RunTimeError> {
    //     match cmd {
    //         Command::AddPair { exchange, pair } => {
    //             self.db.add_pair(&exchange, &pair).await?;
    //             self.state.add_pair(exchange, pair);
    //             Ok(Response::Ok)
    //         }
    //         Command::DropPair { exchange, pair } => {
    //             self.db.drop_pair(&exchange, &pair).await?;
    //             Ok(Response::Ok)
    //         }
    //     }
    // }
}

