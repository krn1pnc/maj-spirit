use std::sync::Arc;
use tokio::sync::RwLock;

use deadpool_sqlite::Pool;

use crate::{room::Hall, ws::ConnectionManager};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool>,
    pub hall: Arc<RwLock<Hall>>,
    pub conn_man: Arc<RwLock<ConnectionManager>>,
}

impl AppState {
    pub fn new(db_pool: Arc<Pool>) -> AppState {
        return AppState {
            db_pool,
            hall: Arc::new(RwLock::new(Hall::default())),
            conn_man: Arc::new(RwLock::new(ConnectionManager::default())),
        };
    }
}
