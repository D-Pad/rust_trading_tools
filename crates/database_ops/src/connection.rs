use mysql_async::{self, OptsBuilder, Pool, prelude::Queryable};
use std::env;
use crate::kraken;


pub const DATABASE_NAME: &'static str = "dpad_llc_trading_app";


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

pub enum DbError {
    ConnectionFailed,
    CredentialsMissing,
    TableCreationFailed,
    QueryFailed,
    ParseError,
    InitFailure,
}

pub enum FetchError {
    Db(DbError),
    MySql(mysql_async::Error),
    Api(kraken::RequestError),
    SystemError(String),
}

impl From<DbError> for FetchError {
    fn from(e: DbError) -> Self {
        FetchError::Db(e)
    }
}

impl From<mysql_async::Error> for FetchError {
    fn from(e: mysql_async::Error) -> Self {
        FetchError::MySql(e)
    }
}

impl From<kraken::RequestError> for FetchError {
    fn from(e: kraken::RequestError) -> Self {
        FetchError::Api(e)
    }
}

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


