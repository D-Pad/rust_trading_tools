use mysql_async::{Pool, Conn, Result, OptsBuilder};

pub struct Db {
    pool: Pool,
}

impl Db {
    
    pub async fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        database: &str,
    ) -> Result<Self> {

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

    pub async fn conn(&self) -> Result<Conn> {
        self.pool.get_conn().await
    }

    pub async fn disconnect(self) {
        let _ = self.pool.disconnect().await;
    }
}


