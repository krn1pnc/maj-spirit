use std::collections::HashMap;

use axum::body::Body;
use axum::extract::ws::{self, WebSocket, WebSocketUpgrade};
use axum::extract::{Extension, State};
use axum::http::Response;
use futures_util::SinkExt;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {}

pub struct Connection {
    tx: mpsc::UnboundedSender<ServerMessage>,
    rx: mpsc::UnboundedReceiver<ClientMessage>,
}

impl Connection {
    pub fn new(
        tx: mpsc::UnboundedSender<ServerMessage>,
        rx: mpsc::UnboundedReceiver<ClientMessage>,
    ) -> Connection {
        return Connection { tx, rx };
    }
}

#[derive(Default)]
pub struct ConnectionManager {
    connections: HashMap<u64, Connection>,
}

impl ConnectionManager {
    pub fn is_connected(&self, uid: u64) -> bool {
        return self.connections.contains_key(&uid);
    }
    pub fn add_client(&mut self, uid: u64, client: Connection) -> Result<(), AppError> {
        if self.is_connected(uid) {
            return Err(AppError::UserAlreadyConnected);
        } else {
            self.connections.insert(uid, client);
            return Ok(());
        }
    }
    pub fn remove_client(&mut self, uid: u64) -> Result<(), AppError> {
        if let Some(_) = self.connections.remove(&uid) {
            return Ok(());
        } else {
            return Err(AppError::UserNotConnected);
        }
    }
    pub fn send(&self, uid: u64, msg: ServerMessage) -> Result<(), AppError> {
        if let Some(conn) = self.connections.get(&uid) {
            conn.tx.send(msg)?;
            return Ok(());
        } else {
            return Err(AppError::UserNotConnected);
        }
    }
    pub async fn recv(&mut self, uid: u64) -> Result<Option<ClientMessage>, AppError> {
        if let Some(conn) = self.connections.get_mut(&uid) {
            return Ok(conn.rx.recv().await);
        } else {
            return Err(AppError::UserNotConnected);
        }
    }
}

async fn handle_socket(socket: WebSocket, state: AppState, uid: u64) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (server_tx, mut server_rx) = mpsc::unbounded_channel::<ServerMessage>();
    let (client_tx, client_rx) = mpsc::unbounded_channel::<ClientMessage>();

    let conn = Connection::new(server_tx, client_rx);

    let mut conn_man = state.conn_man.write().await;
    conn_man.add_client(uid, conn).unwrap();
    drop(conn_man);

    let recv_handle = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(ws::Message::Text(json_text)) => {
                    if let Ok(msg) = serde_json::from_str(&json_text) {
                        client_tx.send(msg).unwrap();
                    }
                }
                Ok(ws::Message::Close(_)) | Err(_) => break,
                _ => (),
            }
        }
    });

    let send_handle = tokio::spawn(async move {
        while let Some(msg) = server_rx.recv().await {
            let msg = serde_json::to_string(&msg).unwrap();
            if ws_tx.send(ws::Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = recv_handle => {},
        _ = send_handle => {},
    }

    let mut conn_man = state.conn_man.write().await;
    conn_man.remove_client(uid).unwrap();
    drop(conn_man);
}

pub async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> Response<Body> {
    return ws.on_upgrade(move |socket| handle_socket(socket, state, uid));
}
