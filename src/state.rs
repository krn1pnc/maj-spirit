use std::sync::Arc;
use tokio::sync::RwLock;

use deadpool_sqlite::Pool;

use crate::room::Hall;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool>,
    pub hall: Arc<RwLock<Hall>>,
}

impl AppState {
    pub fn new(db_pool: Arc<Pool>) -> AppState {
        return AppState {
            db_pool,
            hall: Arc::new(RwLock::new(Hall::default())),
        };
    }
}
