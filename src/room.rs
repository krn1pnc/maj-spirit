use std::collections::{HashMap, HashSet};

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::state::AppState;

pub struct Room {
    players: HashSet<u64>,
}
impl Room {
    pub fn new() -> Room {
        let players = HashSet::with_capacity(4);
        return Room { players };
    }
}

#[derive(Default)]
pub struct Hall {
    pub available_room: HashMap<usize, Room>,
    pub in_room: HashMap<u64, usize>,
}

async fn room_join(hall: &mut Hall, room_id: usize, uid: u64) -> Result<(), AppError> {
    if hall.in_room.contains_key(&uid) {
        return Err(AppError::UserAlreadyInRoom(hall.in_room[&uid]));
    } else {
        if let Some(room) = hall.available_room.get_mut(&room_id) {
            if room.players.len() < 4 {
                hall.in_room.insert(uid, room_id);
                room.players.insert(uid);
                return Ok(());
            } else {
                return Err(AppError::RoomIsFull);
            }
        } else {
            hall.in_room.insert(uid, room_id);
            let mut room = Room::new();
            room.players.insert(uid);
            hall.available_room.insert(room_id, room);
            return Ok(());
        }
    }
}

async fn room_leave(hall: &mut Hall, room_id: usize, uid: u64) -> Result<(), AppError> {
    if !hall.available_room.contains_key(&room_id) {
        return Err(AppError::RoomNotExist);
    } else if !hall.in_room.contains_key(&uid) || room_id != hall.in_room[&uid] {
        return Err(AppError::UserNotInRoom);
    } else {
        hall.in_room.remove(&uid);
        let room = hall.available_room.get_mut(&room_id).unwrap();
        room.players.remove(&uid);
        if room.players.len() == 0 {
            hall.available_room.remove(&room_id);
        }
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
        Err(AppError::RoomIsFull) => {
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
