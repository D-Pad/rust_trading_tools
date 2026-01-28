use database_ops::*;

use crate::app_state::AppState;
use crate::errors::{RunTimeError};
use crate::arg_parsing::{ParsedArgs, ParserError, Command, parse_args};


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

        let args: ParsedArgs = parse_args();

        Ok(Engine { state, database, request_client, args })

    }

    // pub async fn handle(&mut self, cmd: Command) 
    //     -> Result<Response, RunTimeError> 
    // {
    //     match cmd {
    //         Command::AddPair { exchange, pair } => {
    //             self.database.add_pair(&exchange, &pair).await?;
    //             self.state.add_pair(exchange, pair);
    //             Ok(Response::Ok)
    //         }
    //         Command::DropPair { exchange, pair } => {
    //             self.database.drop_pair(&exchange, &pair).await?;
    //             Ok(Response::Ok)
    //         }
    //     }
    // }
}

