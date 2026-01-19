use mysql_async::{self, Pool, Conn, OptsBuilder};
use std::env;
use crate::kraken;


pub struct Db {
    pub pool: Pool,
}

impl Db {
    
    pub async fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        database: &str,
    ) -> mysql_async::Result<Self> {

        let opts: OptsBuilder = OptsBuilder::default()
            .ip_or_hostname(host)
            .tcp_port(port)
            .user(Some(user))
            .pass(Some(password))
            .db_name(Some(database))
            .into();

        let pool = Pool::new(opts);

        Ok(Self { pool })
    }

    pub async fn conn(&self) -> mysql_async::Result<Conn> {
        self.pool.get_conn().await
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

pub async fn get_db_connection(
    existing_db: Option<Db>,
    exchange: &str
) -> Result<Db, DbError> {
    
    let db_connector: Db = match existing_db {
        Some(db) => db,
        None => { 
            
            let db_login: DbLogin = DbLogin::new(); 
            if !&db_login.is_valid() {
                println!("\x1b[1;31mMissing DB credentials\x1b[0m"); 
                return Err(DbError::CredentialsMissing) 
            };
            
            let mut exchange_name = exchange.to_string();
            if !exchange_name.contains("_history") {
                exchange_name.push_str("_history");
            };
            
            let db = match Db::new(
                &db_login.host,
                3306,
                &db_login.user,
                &db_login.password,
                &exchange_name,
            ).await {
                Ok(d) => d,
                Err(_) => return Err(DbError::ConnectionFailed)
            };
            db
        }
    };

    Ok(db_connector)
}

