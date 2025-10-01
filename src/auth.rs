use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Form, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Extension, http};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::PASSWORD_SALT;
use crate::db::{add_user, get_passhash};
use crate::error::AppError;
use crate::jwt;

#[derive(Deserialize)]
pub struct User {
    username: String,
    password: String,
}

async fn register(db_pool: &Pool, user: User) -> Result<(), AppError> {
    let password_salted = user.password + PASSWORD_SALT;
    let passhash = hex::encode(Sha256::digest(&password_salted));
    return add_user(db_pool, &user.username, &passhash).await;
}

async fn login(db_pool: &Pool, user: User) -> Result<String, AppError> {
    let password_salted = user.password + PASSWORD_SALT;
    let passhash = hex::encode(Sha256::digest(&password_salted));
    if passhash == get_passhash(db_pool, &user.username).await? {
        return jwt::get_token(&user.username);
    } else {
        return Err(AppError::PasswordIncorrect);
    }
}

pub async fn jwt_auth(mut req: Request, next: Next) -> Result<Response, http::StatusCode> {
    let auth_header = req.headers().get(http::header::AUTHORIZATION);
    let auth_str = auth_header.and_then(|header| header.to_str().ok());
    let token = auth_str.and_then(|s| s.strip_prefix("Bearer "));
    let username = token.and_then(|t| jwt::verify_token(t).ok());
    match username {
        Some(username) => {
            req.extensions_mut().insert(username);
            return Ok(next.run(req).await);
        }
        None => return Err(http::StatusCode::UNAUTHORIZED),
    }
}

pub async fn handle_register(
    State(db_pool): State<Arc<Pool>>,
    Form(user): Form<User>,
) -> http::Response<Body> {
    match register(&db_pool, user).await {
        Ok(_) => return http::StatusCode::OK.into_response(),
        Err(AppError::UserAlreadyExist) => {
            return (http::StatusCode::CONFLICT, "user exists").into_response();
        }
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_login(
    State(db_pool): State<Arc<Pool>>,
    Form(user): Form<User>,
) -> http::Response<Body> {
    match login(&db_pool, user).await {
        Ok(token) => return token.into_response(),
        Err(AppError::UserNotExist) => {
            return (http::StatusCode::UNAUTHORIZED, "user not exist").into_response();
        }
        Err(AppError::PasswordIncorrect) => {
            return (http::StatusCode::UNAUTHORIZED, "passord incorrect").into_response();
        }
        Err(e) => {
            tracing::error!("{}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}

pub async fn handle_hello(Extension(current_user): Extension<String>) -> String {
    return format!("hello, {}", current_user);
}
