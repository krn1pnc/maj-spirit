use axum::body::Body;
use axum::extract::ws;
use axum::extract::{Extension, State};
use axum::http::Response;
use futures_util::SinkExt;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::game::Cards;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GameInfo {
    pub round_id: usize,
    pub players: [u64; 4],
    pub players_score: [i64; 4],
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(tag = "tag", content = "content")]
pub enum ServerMessage {
    GameNotStart,
    UserNotInRoom,
    NotCurrentPlayer,

    GameInfoSync(GameInfo),
    CardSync(Cards),

    GetCard(u8),
    Discard((u64, u8)),
    NotHaveCard,

    RoundStart(usize),
    WinAll(u64),
    WinOne((u64, u64)),
    Tie,

    GameEnd(usize),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "tag", content = "content")]
pub enum ClientMessage {
    RequestGameSync,
    RequestCardSync,
    Discard(u8),
}

async fn handle_socket(socket: ws::WebSocket, state: AppState, uid: u64) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    let mut tx2clients = state.tx2clients.write().await;
    if !tx2clients.insert(uid, tx.clone()) {
        match ws_tx.send(ws::Message::Close(None)).await {
            Err(e) => tracing::error!("{:?}", e),
            Ok(()) => (),
        }
        return;
    }
    let send_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracing::debug!("send {:?} to {}", msg, uid);

            let msg = serde_json::to_string(&msg).unwrap();
            if ws_tx.send(ws::Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });
    drop(tx2clients);

    let recv_handle = tokio::spawn(async move {
        let handle_message = async |json_text: &str| -> Result<(), AppError> {
            let msg = serde_json::from_str(json_text)?;
            let hall = state.hall.read().await;
            let tx2games = state.tx2games.read().await;
            if let Some(room_id) = hall.belongs.get(&uid) {
                return tx2games.send(room_id, (uid, msg));
            } else {
                return Err(AppError::UserNotInRoom);
            }
        };

        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(ws::Message::Text(json_text)) => {
                    tracing::debug!("recv {:?} from {}", json_text, uid);

                    match handle_message(&json_text).await {
                        Err(AppError::TxNotExist) => {
                            tx.send(ServerMessage::GameNotStart).unwrap();
                        }
                        Err(AppError::UserNotInRoom) => {
                            tx.send(ServerMessage::UserNotInRoom).unwrap();
                        }
                        Err(e) => {
                            tracing::error!("{:?}", e);
                        }
                        _ => (),
                    }
                }
                Ok(ws::Message::Close(_)) => break,
                Err(e) => {
                    tracing::error!("{:?}", e);
                    break;
                }
                _ => (),
            }
        }
    });

    tokio::select! {
        _ = recv_handle => {},
        _ = send_handle => {},
    }

    let mut tx2clients = state.tx2clients.write().await;
    if !tx2clients.delete(&uid) {
        tracing::error!("this should not happen");
    }
}

pub async fn handle_ws(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> Response<Body> {
    return ws.on_upgrade(move |socket| handle_socket(socket, state, uid));
}
