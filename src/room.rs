use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http;
use axum::response::IntoResponse;
use tokio::sync::mpsc;

use crate::db::add_game;
use crate::error::AppError;
use crate::game::Game;
use crate::state::AppState;
use crate::ws::{ClientMessage, ServerMessage};

#[derive(Default, Debug)]
pub struct Hall {
    pub rooms: HashMap<usize, HashSet<u64>>,
    pub belongs: HashMap<u64, usize>,
}

async fn room_join(state: &AppState, room_id: usize, uid: u64) -> Result<(), AppError> {
    let mut hall = state.hall.write().await;
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

async fn room_leave(state: &AppState, room_id: usize, uid: u64) -> Result<(), AppError> {
    let mut hall = state.hall.write().await;
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

async fn room_start(state: &AppState, room_id: usize, uid: u64) -> Result<(), AppError> {
    let hall = state.hall.read().await;
    if !hall.rooms.contains_key(&room_id) {
        return Err(AppError::RoomNotExist);
    } else if !hall.belongs.contains_key(&uid) || room_id != hall.belongs[&uid] {
        return Err(AppError::UserNotInRoom);
    } else if hall.rooms[&room_id].len() != 4 {
        return Err(AppError::RoomNotFull);
    } else {
        let mut players = Vec::with_capacity(4);
        for &i in hall.rooms[&room_id].iter() {
            players.push(i);
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<(u64, ClientMessage)>();

        let players: [u64; 4] = players.try_into().unwrap();
        let _state = state.clone();
        tokio::spawn(async move {
            let state = _state;

            let mut game = Game::new(players, state.tx2clients);
            game.round_start().await;
            while let Some((msg_uid, msg)) = rx.recv().await {
                if game.handle_message(msg, msg_uid).await {
                    break;
                }
            }

            let game = Arc::new(game);
            match add_game(&state.db_pool, game.clone()).await {
                Ok(game_id) => {
                    game.broadcast(ServerMessage::GameEnd(game_id)).await;
                }
                Err(e) => {
                    tracing::error!("{:?}", e);
                }
            }

            let mut tx2games = state.tx2games.write().await;
            tx2games.delete(&room_id);
        });

        let mut tx2rooms = state.tx2games.write().await;
        tx2rooms.insert(room_id, tx);

        return Ok(());
    }
}

pub async fn handle_room_join(
    Path(room_id): Path<usize>,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> http::Response<Body> {
    match room_join(&state, room_id, uid).await {
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
            tracing::error!("{:?}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_room_leave(
    Path(room_id): Path<usize>,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> http::Response<Body> {
    match room_leave(&state, room_id, uid).await {
        Ok(_) => return http::StatusCode::OK.into_response(),
        Err(AppError::RoomNotExist) => {
            return (http::StatusCode::NOT_FOUND, "room not exist").into_response();
        }
        Err(AppError::UserNotInRoom) => {
            return (http::StatusCode::CONFLICT, "user not in room").into_response();
        }
        Err(e) => {
            tracing::error!("{:?}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_room_start(
    Path(room_id): Path<usize>,
    State(state): State<AppState>,
    Extension(uid): Extension<u64>,
) -> http::Response<Body> {
    match room_start(&state, room_id, uid).await {
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
        Err(e) => {
            tracing::error!("{:?}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}
