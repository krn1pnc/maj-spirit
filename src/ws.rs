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
use crate::room::Hall;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "tag", content = "content")]
pub enum ServerMessage {
    GameNotStart,
    UserNotInRoom,
    NotCurrentPlayer,
    GetCard(u8),
    NotHaveCard,
    Discard((String, u8)),
    RoundStart((String, Cards)),
    WinAll(String),
    WinOne((String, String)),
    Tie,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "tag", content = "content")]
pub enum ClientMessage {
    Discard(u8),
}

async fn handle_socket(socket: ws::WebSocket, state: AppState, uid: u64) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let mut hall = state.hall.write().await;
    if hall.tx2clients.contains_key(&uid) {
        ws_tx.send(ws::Message::Close(None)).await.unwrap();
        return;
    }

    let (server_tx, mut server_rx) = mpsc::unbounded_channel::<ServerMessage>();
    hall.tx2clients.insert(uid, server_tx.clone());
    let send_handle = tokio::spawn(async move {
        while let Some(msg) = server_rx.recv().await {
            let msg = serde_json::to_string(&msg).unwrap();
            tracing::info!("send {} to {}", msg, uid);
            if ws_tx.send(ws::Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });
    drop(hall);

    let hall = state.hall.clone();
    let recv_handle = tokio::spawn(async move {
        let handle_message = async |hall: &Hall, json_text: &str| -> Result<(), AppError> {
            let msg = serde_json::from_str(json_text)?;
            if let Some(room_id) = hall.belongs.get(&uid) {
                if let Some(tx2room) = hall.tx2rooms.read().await.get(&room_id) {
                    tx2room.send((uid, msg))?
                } else {
                    return Err(AppError::GameNotStart);
                }
            } else {
                return Err(AppError::UserNotInRoom);
            }
            return Ok(());
        };

        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(ws::Message::Text(json_text)) => {
                    tracing::info!("recv {} from {}", json_text, uid);
                    match handle_message(&*hall.read().await, &json_text).await {
                        Ok(_) => (),
                        Err(AppError::GameNotStart) => {
                            server_tx.send(ServerMessage::GameNotStart).unwrap();
                        }
                        Err(AppError::UserNotInRoom) => {
                            server_tx.send(ServerMessage::UserNotInRoom).unwrap();
                        }
                        Err(e) => {
                            tracing::error!("{}", e);
                        }
                    }
                }
                Ok(ws::Message::Close(_)) => break,
                Err(e) => {
                    tracing::error!("{}", e);
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

    let mut hall = state.hall.write().await;
    hall.tx2clients.remove(&uid);
}

pub async fn handle_ws(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> Response<Body> {
    return ws.on_upgrade(move |socket| handle_socket(socket, state, uid));
}
