use std::{cmp::min, env};
use mysql_async::{self, prelude::*, Conn};
pub mod connection;
pub use connection::Db;
pub mod kraken;


pub enum DbError {
    ConnectionFailed,
    CredentialsMissing,
    QueryFailed,
}

pub enum FetchError {
    Db(DbError),
    MySql(mysql_async::Error),
    Api(kraken::RequestError)
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

struct DbLogin {
    host: String,
    user: String,
    password: String
}

impl DbLogin {
    
    fn new() -> DbLogin {
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

    fn is_valid(&self) -> bool {
        let mut valid = true;
        let vals: [&str; 3] = [&self.user, &self.host, &self.password];
        for value in vals {
            if value == "" { valid = false }
        };
        valid 
    }
}


pub async fn download_new_data_to_db_table(
    exchange: &str, ticker: &str
) -> Result<(), FetchError> {
   
    let db: Db = get_db_connection(None, exchange).await?;

    let data = kraken::request_tick_data_from_kraken(
        ticker, 
        "1767850856".to_string()
    ).await?; 

    println!("DATA: {:?}", data);

    Ok(())
}


pub async fn fetch_last_row(
    exchange: &str, 
    ticker: &str,
    existing_db: Option<Db>
) -> Result<Vec<(u64, u64, f64, f64)>, FetchError> {

    let db_connector: Db = get_db_connection(existing_db, exchange).await?;
    
    let mut conn: Conn = db_connector.conn().await?;
   
    type TickRow = Vec<(u64, u64, f64, f64)>;
    let last_row: TickRow = conn.exec(
        &format!(
            r#"SELECT id, timestamp, price, volume FROM {ticker} 
            ORDER BY id DESC LIMIT 1"#
        ), ()
    ).await?;

    Ok(last_row) 

}

pub async fn fetch_rows(
    exchange: &str, 
    ticker: &str,
    limit: Option<u64>
) -> Result<Vec<(u64, u64, f64, f64)>, FetchError> {

    let db: Db = get_db_connection(None, exchange).await?;

    let limit: u64 = match limit {
        Some(i) => i,
        None => 1_000
    };

    let mut exchange_name = exchange.to_string();
    if !exchange_name.contains("_history") {
        exchange_name.push_str("_history");
    };

    let mut conn = db.conn().await?;

    let first_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM {ticker} 
            ORDER BY id LIMIT 1"#
        ), ()
    ).await {
        Ok(Some(d)) => d,
        Ok(None) | Err(_) => return Err(FetchError::Db(DbError::QueryFailed))
    };

    let last_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM {ticker} 
            ORDER BY id DESC LIMIT 1"#
        ), ()
    ).await {
        Ok(Some(d)) => d,
        Ok(None) | Err(_) => return Err(FetchError::Db(DbError::QueryFailed))
    };

    let mut query: String = String::from(
        "SELECT id, timestamp, price, volume" 
    );

    let tick_id: u64 = last_id - min(last_id - first_id, limit);

    query.push_str(&format!(
        " FROM {ticker} WHERE id >= {tick_id}"
    ));

    let rows: Vec<(u64, u64, f64, f64)> = conn.exec(query, ()).await?;

    Ok(rows)
}


async fn get_db_connection(
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


pub async fn initialize() -> Result<(), DbError> {
    
    let db: Db = get_db_connection(None, "kraken").await?;

    let mut conn = match db.conn().await {
        Ok(d) => d,
        Err(_) => { 
            return Err(DbError::ConnectionFailed)
        }
    };

    let table_request = conn.exec("SHOW TABLES;", ()).await;
    let existing_tables: Vec<(String)> = match table_request {
        Ok(d) => d,
        Err(_) => return Err(DbError::QueryFailed)
    };

    if !existing_tables.contains(&String::from("_last_tick_history")) {
        let query: &'static str = r#"
            CREATE TABLE IF NOT EXISTS _last_tick_history (
                asset VARCHAR(12) NOT NULL,
                id BIGINT NOT NULL,
                timestamp VARCHAR(20)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4; 
        "#;
        if let Err(_) = conn.query_drop(query).await {
            return Err(DbError::QueryFailed); 
        };
    };

    Ok(())
    
    // 1767850856060224
    // q = (
    //     f"UPDATE {HIST} SET "
    //     f"id = '{values['id']}', "
    //     f"timestamp = '{values['timestamp']}' "
    //     f"WHERE asset = '{values['asset']}';"
    // )

    // q = (
    //     f"CREATE TABLE IF NOT EXISTS {HIST} ("
    //     "asset VARCHAR(64), "
    //     "id VARCHAR(255), " 
    //     "timestamp VARCHAR(20));"
    // )

}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_test() {
        assert_eq!(2, 2);
    }
}

