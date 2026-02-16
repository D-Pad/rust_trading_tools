use std::{env, sync::Arc};

use axum::{
    Router, 
    routing::get,
    serve::serve,
};
use tokio::net::TcpListener;

use app_core::{
    engine::{Engine}
};

mod routes;
use routes::{
    generic::*, 
    database::{db_status, db_tables},
};

#[derive(Debug)]
pub enum ServerError {
    InitError(String),
    ServeFailed(String)
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ServerError::InitError(e) => write!(f, "InitError: {}", e),
            ServerError::ServeFailed(e) => write!(f, "ServeFailed: {}", e),
        }
    }
}


// -------------------------------- WEB SERVER ----------------------------- //
pub struct WebServer {
    router: Router,
    listener: TcpListener,
}

impl WebServer {

    pub async fn new(engine: Engine) -> Result<Self, ServerError> {

        let port: String = env::var("HTTP_PORT").unwrap_or_default(); 
        
        let router = Router::new()
            .route("/", get(home))
            .route("/database", get(db_status)) 
            .route("/database/tables", get(db_tables)) 
            .with_state(Arc::new(engine));

        let bind_addr = format!("0.0.0.0:{}", port); 
        
        let listener = TcpListener::bind(bind_addr).await
            .map_err(|_| ServerError::InitError(
                "Failed to bind listener".to_string()
            ))?;

        Ok(Self {
            router,
            listener,
        })

    }

    pub async fn serve(self) -> Result<(), ServerError> {

        serve(self.listener, self.router).await
            .map_err(|_| ServerError::ServeFailed(
                "Failed to start server".to_string()
            ))?;

        Ok(())
    }

}

