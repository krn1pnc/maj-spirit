use axum::{
    extract::{Path, State},
    http,
    response::{IntoResponse, Response},
};
use deadpool_sqlite::Pool;

use crate::{db::query_rankings, error::AppError, state::AppState};

async fn get_rankings(db_pool: &Pool, game_id: usize) -> Result<String, AppError> {
    let rankings = query_rankings(db_pool, game_id).await?;
    let res = serde_json::to_string(&rankings)?;
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
