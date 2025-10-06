use std::sync::Arc;
use tokio::sync::RwLock;

use deadpool_sqlite::Pool;

use crate::room::Hall;
use crate::txmanager::TxManager;
use crate::ws::{ClientMessage, ServerMessage};

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_pool: Arc<Pool>,
    pub hall: Arc<RwLock<Hall>>,
    pub tx2clients: Arc<RwLock<TxManager<u64, ServerMessage>>>,
    pub tx2games: Arc<RwLock<TxManager<usize, (u64, ClientMessage)>>>,
}

impl AppState {
    pub fn new(db_pool: Arc<Pool>) -> AppState {
        return AppState {
            db_pool,
            hall: Arc::new(RwLock::new(Hall::default())),
            tx2clients: Arc::new(RwLock::new(TxManager::default())),
            tx2games: Arc::new(RwLock::new(TxManager::default())),
        };
    }
}
