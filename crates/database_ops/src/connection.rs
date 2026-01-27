use sqlx::{PgPool, postgres::PgPoolOptions};
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
    SQL(sqlx::Error),
    ParseError,
    QueryFailed(String),
    TableCreationFailed(String),
}

impl From<FetchError> for DbError {
    fn from(e: FetchError) -> Self {
        DbError::Fetch(e)
    }
}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        DbError::SQL(e)
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
            DbError::SQL(e) => write!(
                f, "DbError::SQL: {}", e
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


// ----------------------------- STATUS ENUMS ------------------------------ //
#[derive(Debug)]
pub enum DataDownloadStatus {
    Started {
        exchange: String,
        ticker: String,
    },
    Progress {
        exchange: String,
        ticker: String,
        percent: u8,
    },
    Finished {
        exchange: String,
        ticker: String,
    },
    Error {
        exchange: String,
        ticker: String,
    },
}

impl DataDownloadStatus {
    pub fn exchange_and_ticker(&self) -> (&str, &str) {
        match self {
            DataDownloadStatus::Started { exchange, ticker }
            | DataDownloadStatus::Progress { exchange, ticker, .. }
            | DataDownloadStatus::Finished { exchange, ticker }
            | DataDownloadStatus::Error { exchange, ticker, .. } => {
                (exchange.as_str(), ticker.as_str())
            }
        }
    }
}


// ----------------------------- STRUCTS ----------------------------------- //
#[derive(Debug)]
pub struct Db {
    pub pool: PgPool,
}

impl Db {
    
    pub async fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str
    ) -> Result<Self, DbError> {

        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            user,
            password,
            host,
            port,
            DATABASE_NAME
        );

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await
            .map_err(|_| DbError::InitFailure)?;

        Ok(Self { pool })

    }

    pub fn get_pool(&self) -> PgPool {
        self.pool.clone()
    }

    pub async fn disconnect(self) {
        self.pool.close().await;
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
        let host: String = env::var("DB_HOST").unwrap_or_default(); 
        let user: String = env::var("DB_USER_NAME").unwrap_or_default();
        let password: String = env::var("DB_PASSWORD").unwrap_or_default();
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
    format!("asset_{exchange}_{ticker}").to_lowercase()
}


