use std::{collections::HashMap, fmt::Debug, hash::Hash};

use tokio::sync::mpsc;

use crate::error::AppError;

#[derive(Debug)]
pub struct TxManager<T: Eq + Hash, M: Debug> {
    conn: HashMap<T, mpsc::UnboundedSender<M>>,
}

impl<T: Eq + Hash, M: Debug> Default for TxManager<T, M> {
    fn default() -> Self {
        Self {
            conn: HashMap::default(),
        }
    }
}

impl<T: Eq + Hash, M: Debug> TxManager<T, M> {
    pub fn insert(&mut self, uid: T, tx: mpsc::UnboundedSender<M>) -> bool {
        if self.conn.contains_key(&uid) {
            return false;
        } else {
            self.conn.insert(uid, tx);
            return true;
        }
    }

    pub fn delete(&mut self, uid: &T) -> bool {
        if !self.conn.contains_key(uid) {
            return false;
        } else {
            self.conn.remove(uid);
            return true;
        }
    }

    pub fn send(&self, uid: &T, msg: M) -> Result<(), AppError> {
        if !self.conn.contains_key(uid) {
            return Err(AppError::TxNotExist);
        }
        match self.conn[uid].send(msg) {
            Ok(_) => return Ok(()),
            Err(e) => return Err(AppError::MpscSend(e.to_string())),
        }
    }
}
