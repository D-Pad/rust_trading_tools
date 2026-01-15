use std::{cmp::min, env};
use mysql_async::{prelude::*, Conn};
pub mod connection;
pub use connection::Db;
pub mod kraken;


pub enum DbError {
    TickFetch
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

pub async fn fetch_last_row(
    exchange: &str, 
    ticker: &str,
    existing_db_login: Option<Db>
) -> Vec<(u64, u64, f64, f64)> {

    let db_connector: Db = match existing_db_login {
        Some(db) => db,
        None => { 
            
            let db_login: DbLogin = DbLogin::new(); 
            if !&db_login.is_valid() {
                println!("\x1b[1;31mMissing DB credentials\x1b[0m"); 
                return Vec::new()
            };
            
            let mut exchange_name = exchange.to_string();
            if !exchange_name.contains("_history") {
                exchange_name.push_str("_history");
            };
            
            let db = Db::new(
                &db_login.host,
                3306,
                &db_login.user,
                &db_login.password,
                &exchange_name,
            ).await;
            
            match db {
                Ok(d) => d,
                Err(_) => return vec![]
            }
        }
    };

    let mut conn: Conn = match db_connector.conn().await {
        Ok(c) => c,
        Err(_) => return vec![]
    };
   
    type TickRow = Vec<(u64, u64, f64, f64)>;
    let last_row: TickRow = match conn.exec(
        &format!(
            r#"SELECT id, timestamp, price, volume FROM {ticker} 
            ORDER BY id DESC LIMIT 1"#
        ), ()
    ).await {
        Ok(s) => s,
        Err(_) => return vec![]
    };

    last_row 

}

pub async fn fetch_rows(
    exchange: &str, 
    ticker: &str,
    limit: Option<u64>
) -> mysql_async::Result<Vec<(u64, u64, f64, f64)>> {

    let db_login: DbLogin = DbLogin::new();
    if !&db_login.is_valid() {
        println!("\x1b[1;31mMissing DB credentials\x1b[0m"); 
        return Ok(Vec::new())
    };

    let limit: u64 = match limit {
        Some(i) => i,
        None => 1_000
    };

    let mut exchange_name = exchange.to_string();
    if !exchange_name.contains("_history") {
        exchange_name.push_str("_history");
    };

    let db = Db::new(
        &db_login.host,
        3306,
        &db_login.user,
        &db_login.password,
        &exchange_name,
    ).await?;

    let mut conn = db.conn().await?;

    let first_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM {ticker} 
            ORDER BY id LIMIT 1"#
        ), ()
    ).await? {
        Some(i) => i,
        None => return Ok(vec![]) 
    };

    let last_id: u64 = match conn.exec_first::<u64, _, _>(
        &format!(
            r#"SELECT id FROM {ticker} 
            ORDER BY id DESC LIMIT 1"#
        ), ()
    ).await? {
        Some(i) => i,
        None => return Ok(vec![]) 
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_test() {
        assert_eq!(2, 2);
    }
}

