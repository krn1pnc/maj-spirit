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
    SystemTime(#[from] std::time::SystemTimeError),

    #[error("")]
    UserAlreadyExist,

    #[error("")]
    UserNotExist,

    #[error("")]
    PasswordIncorrect,

    #[error("")]
    UserAlreadyInRoom(usize),

    #[error("")]
    UserNotInRoom,

    #[error("")]
    RoomNotExist,

    #[error("")]
    RoomAlreadyFull,

    #[error("")]
    RoomNotFull,

    #[error("")]
    UserNotConnected,

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("")]
    GameNotStart,

    #[error("mpsc send error: {0}")]
    ClientMessageSendError(#[from] tokio::sync::mpsc::error::SendError<crate::ws::ClientMessage>),
}
