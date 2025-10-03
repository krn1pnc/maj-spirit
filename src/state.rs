use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, mpsc};

use deadpool_sqlite::Pool;

use crate::room::Hall;
use crate::ws::ServerMessage;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool>,
    pub hall: Arc<RwLock<Hall>>,
    pub tx2clients: Arc<RwLock<HashMap<u64, mpsc::UnboundedSender<ServerMessage>>>>,
}

impl AppState {
    pub fn new(db_pool: Arc<Pool>) -> AppState {
        return AppState {
            db_pool,
            hall: Arc::new(RwLock::new(Hall::default())),
            tx2clients: Arc::new(RwLock::new(HashMap::new())),
        };
    }
}
