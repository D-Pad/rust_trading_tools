

// ---------------------------- COMMANDS ----------------------------------- //
#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    DataBase(DataBaseCommand)
}

pub enum DataBaseCommand {
    AddPair {
        exchange: String,
        pair: String
    },
    DropPair {
        exchange: String,
        pair: String
    },
}


// ---------------------------- RESPONSES ---------------------------------- //
pub enum Response {
    Ok,
    Data(String),
    Error(String),
}


