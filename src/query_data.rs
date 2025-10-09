use axum::{
    extract::{Path, State},
    http,
    response::{IntoResponse, Response},
};
use deadpool_sqlite::Pool;
use serde::Serialize;

use crate::{
    db::{query_game_detail, query_rankings, query_round_detail},
    error::AppError,
    state::AppState,
};

#[derive(Serialize)]
pub struct GameDetail {
    pub players: Vec<u64>,
    pub players_score: Vec<i64>,
}
impl GameDetail {
    pub fn new() -> GameDetail {
        return GameDetail {
            players: Vec::with_capacity(4),
            players_score: Vec::with_capacity(4),
        };
    }
}

#[derive(Serialize)]
pub struct RoundDetail {
    pub stack: Vec<u8>,
    pub discard: Vec<u8>,
    pub winner_seat: Option<usize>,
    pub loser_seat: Option<usize>,
}
impl RoundDetail {
    pub fn new(
        stack: Vec<u8>,
        discard: Vec<u8>,
        winner_seat: Option<usize>,
        loser_seat: Option<usize>,
    ) -> RoundDetail {
        return RoundDetail {
            stack,
            discard,
            winner_seat,
            loser_seat,
        };
    }
}

async fn get_rankings(db_pool: &Pool, game_id: usize) -> Result<String, AppError> {
    let rankings = query_rankings(db_pool, game_id).await?;
    let res = serde_json::to_string(&rankings)?;
    return Ok(res);
}

async fn get_game_detail(db_pool: &Pool, game_id: usize) -> Result<String, AppError> {
    let game_detail = query_game_detail(db_pool, game_id).await?;
    let res = serde_json::to_string(&game_detail)?;
    return Ok(res);
}

async fn get_round_detail(
    db_pool: &Pool,
    game_id: usize,
    round_id: usize,
) -> Result<String, AppError> {
    let round_detail = query_round_detail(db_pool, game_id, round_id).await?;
    let res = serde_json::to_string(&round_detail)?;
    return Ok(res);
}

pub async fn handle_get_rankings(
    Path(game_id): Path<usize>,
    State(state): State<AppState>,
) -> Response {
    match get_rankings(&state.db_pool, game_id).await {
        Ok(res) => return res.into_response(),
        Err(AppError::GameNotExist) => return http::StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_get_game_detail(
    Path(game_id): Path<usize>,
    State(state): State<AppState>,
) -> Response {
    match get_game_detail(&state.db_pool, game_id).await {
        Ok(res) => return res.into_response(),
        Err(AppError::GameNotExist) => return http::StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_get_round_detail(
    Path((game_id, round_id)): Path<(usize, usize)>,
    State(state): State<AppState>,
) -> Response {
    if round_id >= 4 {
        return http::StatusCode::NOT_FOUND.into_response();
    }
    match get_round_detail(&state.db_pool, game_id, round_id).await {
        Ok(res) => return res.into_response(),
        Err(AppError::GameNotExist) => return http::StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}
