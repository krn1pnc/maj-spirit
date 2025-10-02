use std::collections::{HashMap, HashSet};

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Default)]
pub struct Room {
    players: HashSet<String>,
}

#[derive(Default)]
pub struct Hall {
    pub available_room: HashMap<usize, Room>,
    pub in_room: HashMap<String, usize>,
}

async fn room_join(hall: &mut Hall, room_id: usize, current_user: String) -> Result<(), AppError> {
    if hall.in_room.contains_key(&current_user) {
        return Err(AppError::UserAlreadyInRoom(hall.in_room[&current_user]));
    } else {
        if let Some(room) = hall.available_room.get_mut(&room_id) {
            if room.players.len() < 4 {
                hall.in_room.insert(current_user.clone(), room_id);
                room.players.insert(current_user);
                return Ok(());
            } else {
                return Err(AppError::RoomIsFull);
            }
        } else {
            hall.in_room.insert(current_user.clone(), room_id);
            let mut room = Room::default();
            room.players.insert(current_user);
            hall.available_room.insert(room_id, room);
            return Ok(());
        }
    }
}

async fn room_leave(hall: &mut Hall, room_id: usize, current_user: String) -> Result<(), AppError> {
    if !hall.available_room.contains_key(&room_id) {
        return Err(AppError::RoomNotExist);
    } else if !hall.in_room.contains_key(&current_user) || room_id != hall.in_room[&current_user] {
        return Err(AppError::UserNotInRoom);
    } else {
        hall.in_room.remove(&current_user);
        let room = hall.available_room.get_mut(&room_id).unwrap();
        room.players.remove(&current_user);
        if room.players.len() == 0 {
            hall.available_room.remove(&room_id);
        }
        return Ok(());
    }
}

pub async fn handle_room_join(
    Path(room_id): Path<usize>,
    State(state): State<AppState>,
    Extension(current_user): Extension<String>,
) -> http::Response<Body> {
    let mut hall = state.hall.write().await;
    match room_join(&mut hall, room_id, current_user).await {
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
    Extension(current_user): Extension<String>,
) -> http::Response<Body> {
    let mut hall = state.hall.write().await;
    match room_leave(&mut hall, room_id, current_user).await {
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
