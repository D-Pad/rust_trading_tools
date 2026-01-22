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
    NoData,
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


#[derive(Debug)]
pub enum FetchError {
    Api(RequestError),
    SystemError(String),
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


