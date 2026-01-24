use mysql_async::{self, OptsBuilder, Pool, prelude::Queryable};
use std::env;


pub const DATABASE_NAME: &'static str = "dpad_llc_trading_app";


// ----------------------- ERROR ENUMS ----------------------------- //
#[derive(Debug)]
pub enum RequestError {
    Http(reqwest::Error),
    BadStatus(reqwest::StatusCode),
    Deserialize(serde_json::Error),
    RequestFailed(String),
    ErrorResponse(String),
    NoData,
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RequestError::Http(e) => write!(
                f, "RequestError::Http: {}", e
            ),
            RequestError::BadStatus(e) => write!(
                f, "RequestError::BadStatus: {}", e
            ),
            RequestError::Deserialize(e) => write!(
                f, "RequestError::Deserialize: {}", e
            ),
            RequestError::RequestFailed(e) => write!(
                f, "RequestError::RequestFailed: {}", e
            ),
            RequestError::ErrorResponse(e) => write!(
                f, "RequestError::ErrorResponse: {}", e
            ),
            RequestError::NoData => write!(
                f, "RequestError::RequestFailed: Request returned no data"
            )
        }
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(e: reqwest::Error) -> Self {
        RequestError::Http(e)
    }
}

impl From<serde_json::Error> for RequestError {
    fn from(e: serde_json::Error) -> Self {
        RequestError::Deserialize(e)
    }
}


#[derive(Debug)]
pub enum DbError {
    ConnectionFailed,
    CredentialsMissing,
    Fetch(FetchError),
    InitFailure,
    MySql(mysql_async::Error),
    ParseError,
    QueryFailed(String),
    TableCreationFailed(String),
}

impl From<FetchError> for DbError {
    fn from(e: FetchError) -> Self {
        DbError::Fetch(e)
    }
}

impl From<mysql_async::Error> for DbError {
    fn from(e: mysql_async::Error) -> Self {
        DbError::MySql(e)
    }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DbError::ConnectionFailed => write!(
                f, "DbError: Connection failed"
            ),
            DbError::CredentialsMissing => write!(
                f, "DbError: Database login credentials missing"
            ),
            DbError::Fetch(e) => write!(
                f, "DbError::FetchError: {}", e
            ),
            DbError::InitFailure => write!(
                f, "DbError: Could not initialize database connector struct"
            ),
            DbError::MySql(e) => write!(
                f, "DbError::MySqlAsync: {}", e
            ),
            DbError::ParseError => write!(
                f, "DbError: Failed to parse database data"
            ),
            DbError::QueryFailed(e) => write!(
                f, "DbError: Query Failed: {} ", e
            ),
            DbError::TableCreationFailed(e) => write!(
                f, "DbError: Failed to create new table: {} ", e
            )
        }
    }
}


#[derive(Debug)]
pub enum FetchError {
    Api(RequestError),
    SystemError(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FetchError::Api(e) => write!(
                f, "FetchError::Api: {} ", e
            ),
            FetchError::SystemError(e) => write!(
                f, "FetchError::SystemError: {} ", e
            )
        }
    }
}

impl From<RequestError> for FetchError {
    fn from(e: RequestError) -> Self {
        FetchError::Api(e)
    }
}


// ----------------------------- STRUCTS ----------------------------------- //
#[derive(Debug)]
pub struct Db {
    pub pool: Pool,
}

impl Db {
    
    pub async fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str
    ) -> mysql_async::Result<Self> {

        let init_opts: OptsBuilder = OptsBuilder::default()
            .ip_or_hostname(host)
            .tcp_port(port)
            .user(Some(user))
            .pass(Some(password))
            .into();

        let init_pool = Pool::new(init_opts);
        
        if let Ok(mut p) = init_pool.get_conn().await {
            let _ = p.exec_drop(
                format!("CREATE DATABASE IF NOT EXISTS {};", DATABASE_NAME),
                ()
            ).await;
        };

        let opts: OptsBuilder = OptsBuilder::default()
            .ip_or_hostname(host)
            .tcp_port(port)
            .user(Some(user))
            .pass(Some(password))
            .db_name(Some(DATABASE_NAME))
            .into();

        let pool = Pool::new(opts);

        Ok(Self { pool })
    }

    pub fn get_pool(&self) -> mysql_async::Pool {
        self.pool.clone()
    }

    pub async fn disconnect(self) {
        let _ = self.pool.disconnect().await;
    }

}


#[derive(Debug)]
pub struct DbLogin {
    pub host: String,
    pub user: String,
    pub password: String
}

impl DbLogin {
    
    pub fn new() -> DbLogin {
        let host: String = match env::var("DB_HOST") {
            Ok(s) => s,
            Err(_) => String::from("")
        };
        let user: String = match env::var("DB_USER_NAME") {
            Ok(s) => s,
            Err(_) => String::from("")
        };
        let password: String = match env::var("DB_PASSWORD") {
            Ok(s) => s,
            Err(_) => String::from("")
        };
        DbLogin { host, user, password } 
    }

    pub fn is_valid(&self) -> bool {
        let mut valid = true;
        let vals: [&str; 3] = [&self.user, &self.host, &self.password];
        for value in vals {
            if value == "" { valid = false }
        };
        valid 
    }
}


pub fn get_table_name(exchange: &str, ticker: &str) -> String {
    format!("asset_{exchange}_{ticker}")
}


