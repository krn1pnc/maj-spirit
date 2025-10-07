use axum::{
    extract::{Path, State},
    http,
    response::{IntoResponse, Response},
};
use deadpool_sqlite::Pool;
use serde::Serialize;

use crate::{
    db::{query_game_details, query_rankings},
    error::AppError,
    state::AppState,
};

#[derive(Serialize)]
pub struct GameDetail {
    pub players: Vec<u64>,
    pub players_score: Vec<u64>,
}
impl GameDetail {
    pub fn new() -> GameDetail {
        return GameDetail {
            players: Vec::with_capacity(4),
            players_score: Vec::with_capacity(4),
        };
    }
}

async fn get_rankings(db_pool: &Pool, game_id: usize) -> Result<String, AppError> {
    let rankings = query_rankings(db_pool, game_id).await?;
    let res = serde_json::to_string(&rankings)?;
    return Ok(res);
}

async fn get_game_detail(db_pool: &Pool, game_id: usize) -> Result<String, AppError> {
    let game_detail = query_game_details(db_pool, game_id).await?;
    let res = serde_json::to_string(&game_detail)?;
    return Ok(res);
}

pub async fn handle_get_rankings(
    Path(game_id): Path<usize>,
    State(state): State<AppState>,
) -> Response {
    match get_rankings(&state.db_pool, game_id).await {
        Ok(res) => return res.into_response(),
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
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}
