use std::time::SystemTimeError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("database connection pool error: {0}")]
    Pool(#[from] deadpool_sqlite::PoolError),

    #[error("database interaction error: {0}")]
    Interaction(#[from] deadpool_sqlite::InteractError),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] deadpool_sqlite::rusqlite::Error),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("system time error: {0}")]
    SystemTime(#[from] SystemTimeError),

    #[error("")]
    UserAlreadyExist,

    #[error("")]
    UserNotExist,

    #[error("")]
    PasswordIncorrect,
}
