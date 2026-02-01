use std::{cmp::{min, max}, fmt};

use reqwest;
use sqlx::{PgPool, pool::{PoolConnection}, types::BigDecimal};
use tokio::{sync::mpsc::UnboundedSender, task::JoinSet};

use timestamp_tools::db_timestamp_to_date_string;

pub mod connection;
pub use connection::{
    Db, 
    DbLogin, 
    DbError,
    DataDownloadStatus,
    FetchError, 
    get_table_name
};
pub mod kraken;


pub async fn add_new_pair(
    exchange: &str, 
    ticker: &str,
    time_offset: u64,
    db_pool: PgPool,
    client: &reqwest::Client
) -> Result<(), DbError> {
    
    match exchange {
        "kraken" => {
            kraken::add_new_db_table(
                ticker, 
                time_offset, 
                client, 
                db_pool.clone()
            ).await?;
        },
        _ => {

        }
    };

    Ok(())

}


pub async fn drop_pair(
    exchange: &str, 
    ticker: &str,
    db_pool: PgPool
) -> Result<(), DbError> {
    
    let query = format!(r#"
    DROP TABLE asset_{exchange}_{ticker} 
    "#);

    sqlx::query(&query)
        .execute(&db_pool)
        .await.map_err(|_| DbError::QueryFailed(query.to_string()))?;

    Ok(())
}


pub async fn download_new_data_to_db_table(
    exchange: &str, 
    ticker: &str,
    db_pool: PgPool,
    initial_unix_timestamp_offset: u64,
    client: &reqwest::Client,
    progress_tx: UnboundedSender<DataDownloadStatus>,
) -> Result<(), DbError> {
   
    if exchange == "kraken" {      
        kraken::download_new_data_to_db_table(
            ticker, 
            db_pool, 
            initial_unix_timestamp_offset,
            client,
            progress_tx,
        ).await?; 
    };

    Ok(())

}


pub async fn fetch_first_tick_by_time_column(
    exchange: &str,
    ticker: &str,
    timestamp: &u64,
    db_pool: PgPool
) -> Vec<(u64, u64, BigDecimal, BigDecimal)> {
    
    let query: String = format!(
        r#"
        SELECT id, time, price, volume FROM asset_{}_{}
        WHERE time >= {}
        LIMIT 1;
        "#,
        exchange,
        ticker,
        timestamp 
    );
    
    type Vrow = Vec<(u64, u64, BigDecimal, BigDecimal)>;
    let row: Vrow = match sqlx::query_as::
        <_, (i64, i64, BigDecimal, BigDecimal)>
        (&query)
            .fetch_all(&db_pool)
            .await 
    {
        Ok(rows) => rows
            .into_iter()
            .map(|(i, t, p, v)|(i as u64, t as u64, p, v))
            .collect()
        ,
        Err(_) => Vec::new()
    };
    
    row
}


pub async fn fetch_tables(
    db_pool: PgPool 
) -> Result<Vec<String>, DbError> {

    let table_query: String = format!(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
        "#
    );

    let tables: Vec<String> = match sqlx::query_scalar(&table_query)
        .fetch_all(&db_pool)
        .await 
    {
        Ok(d) => d,
        Err(_) => return Err(DbError::QueryFailed(
            "Failed to fetch table names".to_string() 
        ))
    };

    Ok(tables)

}


pub async fn fetch_first_or_last_row(
    exchange: &str, 
    ticker: &str,
    db_pool: PgPool,
    last_row: bool
) -> Result<Vec<(u64, u64, BigDecimal, BigDecimal)>, DbError> {

    let limit_str: &str = match last_row {
        true => "DESC ",
        false => ""
    };

    let query = format!(
        r#"SELECT id, time, price, volume 
        FROM asset_{exchange}_{ticker} 
        ORDER BY id {}LIMIT 1"#,
        limit_str
    );

    type TickRow = Vec<(u64, u64, BigDecimal, BigDecimal)>;
    let row: TickRow = match sqlx::query_as::<
        _, (i64, i64, BigDecimal, BigDecimal)
    >
        (&query)
        .fetch_all(&db_pool)
        .await 
    {
        Ok(d) => d
            .into_iter()
            .map(|(i, t, p, v)| (i as u64, t as u64, p, v))
            .collect() 
        ,
        Err(_) => {
            return Err(DbError::QueryFailed(query))
        }
    };

    Ok(row)

}


pub async fn fetch_rows(
    exchange: &str, 
    ticker: &str,
    limit: Option<u64>,
    db_pool: PgPool
) -> Result<Vec<(u64, u64, BigDecimal, BigDecimal)>, DbError> {

    let table_name = get_table_name(exchange, ticker);

    let mut conn: PoolConnection<sqlx::Postgres> = match db_pool
        .acquire()
        .await 
    {
        Ok(c) => c,
        Err(_) => return Err(DbError::ConnectionFailed)
    };

    let limit: u64 = match limit {
        Some(i) => i,
        None => 1_000
    };

    let last_id_query: &String = &format!(
        r#"SELECT id FROM {table_name} 
        ORDER BY id DESC LIMIT 1"#
    );
   
    let last_id: u64 = match sqlx::query_scalar::<_, i64>(last_id_query)
        .fetch_all(&mut *conn)
        .await 
    {
        Ok(d) => {
            if d.len() > 0 { 
                d[0] as u64
            }
            else {
                return Err(
                    DbError::QueryFailed(
                        "Last ID could not be fetched, table empty".to_string() 
                    )
                )
            }
        },
        Err(_) => return Err(
            DbError::QueryFailed(
                "Failed to fetch last tick ID".to_string() 
            )
        )
    };

    let tick_id: u64 = max(1, last_id - limit);
    
    let query: String = format!(
        r#"
        SELECT id, time, price, volume
        FROM {table_name} WHERE id >= {tick_id};
        "#,
    );

    type Drow = Vec<(u64, u64, BigDecimal, BigDecimal)>;
   
    let rows: Drow = match sqlx::query_as::<
        _, (i64, i64, BigDecimal, BigDecimal)
    >(&query)
        .fetch_all(&mut *conn)
        .await 
    {
        Ok(d) => d.into_iter()
            .map(|(i, t, p, vol)| (i as u64, t as u64, p, vol))
            .collect()
        ,
        Err(e) => return {
            Err(
                DbError::QueryFailed(
                    format!("Failed to fetch last tick ID: {}", e) 
                )
            )
        }
    };

    Ok(rows)
}


pub async fn first_time_setup(
    active_exchanges: &Vec<String>, 
    db_pool: PgPool 
) -> Result<(), DbError> {
   
    for exchange_name in active_exchanges {

        if exchange_name == "kraken" { 

            let mut conn: PoolConnection<sqlx::Postgres> = match db_pool
                .acquire()
                .await 
            {
                Ok(c) => c,
                Err(_) => return Err(DbError::ConnectionFailed)
            };

            let tables: Vec<String> = fetch_tables(db_pool.clone()).await?;

            if !tables.contains(&"_last_tick_history".to_string()) {

                let query: &'static str = r#"
                    CREATE TABLE IF NOT EXISTS _last_tick_history (
                        asset VARCHAR(12) NOT NULL PRIMARY KEY,
                        next_tick_id BIGINT NOT NULL,
                        time VARCHAR(20)
                    ); 
                "#;
                if let Err(_) = sqlx::query(&query)
                    .execute(&mut *conn)
                    .await 
                {
                    return Err(DbError::QueryFailed(
                            "Failed to create '_last_tick_history'".to_string()
                        )
                    ); 
                };
            };
        };
    };

    Ok(())
    
}


pub async fn initialize(active_exchanges: &Vec<String>) -> Result<Db, DbError> {

    let db_login: DbLogin = DbLogin::new(); 
 
    if !&db_login.is_valid() {
        return Err(DbError::CredentialsMissing)
    };
    
    let database = match Db::new().await {
        Ok(d) => d,
        Err(_) => return Err(DbError::ConnectionFailed)
    };

    let db_pool = database.get_pool();

    first_time_setup(&active_exchanges, db_pool.clone()).await?;

    Ok(database)

}


pub async fn update_database_tables(
    active_exchanges: &Vec<String>,
    time_offset: u64,
    client: &reqwest::Client,
    db_pool: PgPool,
    progress_tx: tokio::sync::mpsc::UnboundedSender<DataDownloadStatus>
) -> Result<(), DbError> {

    println!("\x1b[1;33mUpdating existing database tables\x1b[0m");
    
    let existing_tables = fetch_tables(db_pool.clone()).await?;

    let mut tasks: JoinSet<Result<(), DbError>> = JoinSet::new();
    
    for exchange_name in active_exchanges {
   
        let exchange_tables: Vec<&String> = existing_tables
            .iter() 
            .filter(|x| x.contains(exchange_name))
            .collect();

        if exchange_name == "kraken" {

            for table in &exchange_tables {
                
                let ticker: String = match table.split('_').last() {
                    Some(a) => a.to_uppercase(),
                    None => continue 
                };

                let task_db_pool = db_pool.clone();
                let task_tx = progress_tx.clone();
                let task_client = client.clone();

                tasks.spawn(async move {
                    kraken::download_new_data_to_db_table(
                        &ticker, 
                        task_db_pool, 
                        time_offset, 
                        &task_client, 
                        task_tx 
                    ).await 
                });
            };
        };
    };

    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(inner) => inner?,
            Err(join_err) => {
                return Err(DbError::TaskJoin(join_err))
            }
        } 
    };

    Ok(())

}


pub struct DatabaseIntegrity {
    pub table_name: String,
    pub is_ok: bool,
    pub first_tick_id: u64,
    pub last_tick_id: u64,
    pub first_date: String, 
    pub last_date: String, 
    pub total_ticks: u64,
    pub missing_ticks: Vec<u64>,
    pub error: String 
}

impl fmt::Display for DatabaseIntegrity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       
        fn col(passes: bool) -> &'static str {
            match passes {
                true => "\x1b[32m",
                false => "\x1b[31m",
            }
        }
        
        write!(f, "\x1b[1;36mDatabase Integrity:\x1b[0m\n")?;
        write!(f, "  \x1b[33mtable_name   \x1b[0m: {}\n", 
            self.table_name)?;
        write!(f, "  \x1b[33mis_ok        \x1b[0m: {}{}\n", 
            col(self.is_ok), self.is_ok)?;
        write!(f, "  \x1b[33mfirst_tick_id\x1b[0m: {}\n", self.first_tick_id)?;
        write!(f, "  \x1b[33mlast_tick_id \x1b[0m: {}\n", self.last_tick_id)?;
        write!(f, "  \x1b[33mfirst_date   \x1b[0m: {}\n", self.first_date)?;
        write!(f, "  \x1b[33mlast_date    \x1b[0m: {}\n", self.last_date)?;
        write!(f, "  \x1b[33mtotal_ticks  \x1b[0m: {}\n", self.total_ticks)?;
        
        if self.missing_ticks.len() > 0 {
            write!(f, "  \x1b[33mmissing_ticks\x1b[0m: [\n\x1b[1;31m")?;
            for missing in &self.missing_ticks {
                write!(f, "    {}\n", missing)?;
            };
            write!(f, "\x1b[0m  ]\n")?;
        }
        else {
            write!(f, 
                "  \x1b[33mmissing_ticks\x1b[0m: \x1b[32mnone\x1b[0m\n")?;
        };

        if !self.is_ok {
            write!(f, "  \x1b[33merror\x1b[0m: \x1b[1:31m{}", self.error)?;
        };
        Ok(())
    }
} 


pub async fn integrity_check(
    exchange: &str, 
    ticker: &str,
    db_pool: PgPool,
    tick_step_value: Option<u16>
) -> DatabaseIntegrity {

    let table_name = get_table_name(exchange, ticker); 
    
    let mut dbi: DatabaseIntegrity = DatabaseIntegrity { 
        table_name: table_name.clone(), 
        is_ok: true, 
        first_tick_id: 0, 
        last_tick_id: 0,
        first_date: String::new(),
        last_date: String::new(),
        total_ticks: 0,
        missing_ticks: Vec::new(), 
        error: String::new() 
    };

    let mut conn = match db_pool
        .acquire()
        .await 
    {
        Ok(c) => c,
        Err(_) => {
            dbi.error.push_str("Failed to establish a Database Connection");
            return dbi 
        }
    };

    (dbi.first_tick_id, dbi.first_date) = match fetch_first_or_last_row(
        exchange, ticker, db_pool.clone(), false
    ).await {
        Ok(d) => (d[0].0, db_timestamp_to_date_string(d[0].1)),
        Err(e) => {
            dbi.error.push_str(&format!("Couldn't fetch first tick ID: {}", e));
            return dbi
        }
    };
     
    (dbi.last_tick_id, dbi.last_date) = match fetch_first_or_last_row(
        exchange, ticker, db_pool.clone(), true
    ).await {
        Ok(d) => (d[0].0, db_timestamp_to_date_string(d[0].1)),
        Err(e) => {
            dbi.error.push_str(&format!("Couldn't fetch last tick ID: {}", e)); 
            return dbi 
        }
    };

    const DEFAULT_STEP_VALUE: u16 = 10000;
    let step_val = match tick_step_value {
        Some(s) => s,
        None => DEFAULT_STEP_VALUE
    };

    let range_vals = dbi.first_tick_id..dbi.last_tick_id;
    let mut last_id = 0;
   
    for start in range_vals.step_by(step_val as usize) {
      
        let end = min(start + (step_val as u64) - 1, dbi.last_tick_id); 
        
        let query = format!(
            "SELECT id FROM {} WHERE id BETWEEN {} AND {}",
            table_name,
            start,
            end
        );
        
        let tick_slice: Vec<u64> = match sqlx::query_scalar(&query)
            .fetch_all(&mut *conn)
            .await 
        {
            Ok(d) => d.into_iter().map(|v: i64| v as u64).collect(),
            Err(_) => {
                dbi.error.push_str("Failed to fetch tick slice");
                return dbi
            }
        };

        dbi.total_ticks += tick_slice.len() as u64;

        for tick_id in tick_slice {
            if last_id != 0 && tick_id != last_id + 1 {
                for i in (last_id + 1)..tick_id {
                    dbi.missing_ticks.push(i);
                };
                dbi.is_ok = false;
            };
            last_id = tick_id;
        }; 
    
    }; 

    if dbi.error.len() > 0 { 
        dbi.is_ok = false 
    };
   
    // Extra layer of checking, even though the loop above wold cover this
    // particular scenario
    if (dbi.last_tick_id - dbi.first_tick_id) + 1 != dbi.total_ticks {
        dbi.is_ok = false
    };

    dbi

}


