use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http;
use axum::response::IntoResponse;
use tokio::sync::{RwLock, mpsc};

use crate::error::AppError;
use crate::game;
use crate::state::AppState;
use crate::ws::{ClientMessage, ServerMessage};

#[derive(Default)]
pub struct Hall {
    pub rooms: HashMap<usize, HashSet<u64>>,
    pub belongs: HashMap<u64, usize>,
    pub tx2rooms: Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<(u64, ClientMessage)>>>>,
    pub tx2clients: HashMap<u64, mpsc::UnboundedSender<ServerMessage>>,
}

async fn room_join(hall: &mut Hall, room_id: usize, uid: u64) -> Result<(), AppError> {
    if hall.belongs.contains_key(&uid) {
        return Err(AppError::UserAlreadyInRoom(hall.belongs[&uid]));
    } else {
        if let Some(players) = hall.rooms.get_mut(&room_id) {
            if players.len() < 4 {
                players.insert(uid);
                hall.belongs.insert(uid, room_id);
                return Ok(());
            } else {
                return Err(AppError::RoomAlreadyFull);
            }
        } else {
            let mut players = HashSet::with_capacity(4);
            players.insert(uid);
            hall.belongs.insert(uid, room_id);
            hall.rooms.insert(room_id, players);
            return Ok(());
        }
    }
}

async fn room_leave(hall: &mut Hall, room_id: usize, uid: u64) -> Result<(), AppError> {
    if !hall.rooms.contains_key(&room_id) {
        return Err(AppError::RoomNotExist);
    } else if !hall.belongs.contains_key(&uid) || room_id != hall.belongs[&uid] {
        return Err(AppError::UserNotInRoom);
    } else {
        hall.belongs.remove(&uid);
        let room = hall.rooms.get_mut(&room_id).unwrap();
        room.remove(&uid);
        if room.len() == 0 {
            hall.rooms.remove(&room_id);
        }
        return Ok(());
    }
}

async fn room_start(hall: &mut Hall, room_id: usize, uid: u64) -> Result<(), AppError> {
    if !hall.rooms.contains_key(&room_id) {
        return Err(AppError::RoomNotExist);
    } else if !hall.belongs.contains_key(&uid) || room_id != hall.belongs[&uid] {
        return Err(AppError::UserNotInRoom);
    } else if hall.rooms[&room_id].len() != 4 {
        return Err(AppError::RoomNotFull);
    } else {
        let (tx, mut rx) = mpsc::unbounded_channel::<(u64, ClientMessage)>();

        let mut players = Vec::with_capacity(4);
        let mut players_tx = Vec::with_capacity(4);
        for i in hall.rooms[&room_id].iter() {
            players.push(*i);
            if let Some(player_tx) = hall.tx2clients.get(i) {
                players_tx.push(player_tx.clone());
            } else {
                return Err(AppError::UserNotConnected);
            }
        }

        let players: [u64; 4] = players.try_into().unwrap();
        let players_tx: [_; 4] = players_tx.try_into().unwrap();
        let tx2rooms_lock = hall.tx2rooms.clone();
        tokio::spawn(async move {
            let mut game = game::Game::new(players, players_tx);
            while let Some((msg_uid, msg)) = rx.recv().await {
                if game.handle_message(msg, msg_uid) {
                    break;
                }
            }
            let mut tx2rooms = tx2rooms_lock.write().await;
            tx2rooms.remove(&room_id);
        });

        let mut tx2rooms = hall.tx2rooms.write().await;
        tx2rooms.insert(room_id, tx);

        return Ok(());
    }
}

pub async fn handle_room_join(
    Path(room_id): Path<usize>,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> http::Response<Body> {
    let mut hall = state.hall.write().await;
    match room_join(&mut hall, room_id, uid).await {
        Ok(_) => return http::StatusCode::OK.into_response(),
        Err(AppError::UserAlreadyInRoom(room_id)) => {
            return (
                http::StatusCode::CONFLICT,
                format!("user already in room {}", room_id),
            )
                .into_response();
        }
        Err(AppError::RoomNotExist) => {
            return (http::StatusCode::NOT_FOUND, "room not exist").into_response();
        }
        Err(AppError::RoomAlreadyFull) => {
            return (http::StatusCode::CONFLICT, "room is full").into_response();
        }
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_room_leave(
    Path(room_id): Path<usize>,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> http::Response<Body> {
    let mut hall = state.hall.write().await;
    match room_leave(&mut hall, room_id, uid).await {
        Ok(_) => return http::StatusCode::OK.into_response(),
        Err(AppError::RoomNotExist) => {
            return (http::StatusCode::NOT_FOUND, "room not exist").into_response();
        }
        Err(AppError::UserNotInRoom) => {
            return (http::StatusCode::CONFLICT, "user not in room").into_response();
        }
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_room_start(
    Path(room_id): Path<usize>,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> http::Response<Body> {
    let mut hall = state.hall.write().await;
    match room_start(&mut hall, room_id, uid).await {
        Ok(_) => return http::StatusCode::OK.into_response(),
        Err(AppError::RoomNotExist) => {
            return (http::StatusCode::CONFLICT, "room not exist").into_response();
        }
        Err(AppError::UserNotInRoom) => {
            return (http::StatusCode::CONFLICT, "user not in room").into_response();
        }
        Err(AppError::RoomNotFull) => {
            return (http::StatusCode::CONFLICT, "room not full").into_response();
        }
        Err(AppError::UserNotConnected) => {
            return (http::StatusCode::CONFLICT, "user not connected").into_response();
        }
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}
