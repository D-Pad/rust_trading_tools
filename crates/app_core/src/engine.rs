use database_ops::*;

use crate::app_state::AppState;
use crate::errors::{RunTimeError};
use crate::arg_parsing::{
    ParsedArgs,
    Response,
    Command, 
    parse_args
};


use reqwest::Client;


pub struct Engine {
    pub state: AppState,
    pub database: Db,
    pub request_client: Client,
    pub args: ParsedArgs
}

impl Engine {
   
    pub fn new(database: Db) -> Result<Self, RunTimeError> {

        let state: AppState = AppState::new()
            .map_err(|e| RunTimeError::Init(e))?;

        let request_client: Client = Client::new();

        let args: ParsedArgs = parse_args(None);

        Ok(Engine { state, database, request_client, args })

    }

    pub async fn handle(&mut self, cmd: Command) 
        -> Result<Response, RunTimeError> {
        match cmd {
            
            Command::AddPair { exchange, pair } => {
                
                add_new_pair(
                    &exchange, 
                    &pair, 
                    self.state.time_offset(),
                    self.database.get_pool(),
                    &self.request_client
                ).await.map_err(|e| RunTimeError::DataBase(e))?;

                Ok(Response::Ok)
            },

            Command::DropPair { exchange, pair } => {
                drop_pair(&exchange, &pair, self.database.get_pool())
                    .await 
                    .map_err(|e| RunTimeError::DataBase(e))?;

                Ok(Response::Ok)
            },

            Command::StartServer => { 
                // TODO: Add server starting logic 
                Ok(Response::Ok) 
            }
        }
    }
}

