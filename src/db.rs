use crate::error::AppError;
use deadpool_sqlite::{Pool, rusqlite};

pub async fn init_db(db_pool: &Pool) -> Result<(), AppError> {
    let db_conn = db_pool.get().await?;
    return db_conn
        .interact(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS users(
                    username TEXT PRIMARY KEY,
                    passhash TEXT
                )",
                (),
            )?;
            return Ok(());
        })
        .await?;
}

pub async fn add_user(db_pool: &Pool, username: &str, passhash: &str) -> Result<(), AppError> {
    let db_conn = db_pool.get().await?;
    let db_params = (username.to_string(), passhash.to_string());
    return db_conn
        .interact(|conn| {
            match conn.execute("INSERT INTO users VALUES (?1, ?2)", db_params) {
                Err(rusqlite::Error::SqliteFailure(info, s)) => {
                    // check primary key conflict
                    if info.extended_code == 1555 {
                        return Err(AppError::UserAlreadyExist);
                    } else {
                        return Err(AppError::Sqlite(rusqlite::Error::SqliteFailure(info, s)));
                    }
                }
                Err(e) => return Err(e.into()),
                Ok(_) => return Ok(()),
            }
        })
        .await?;
}

pub async fn get_passhash(db_pool: &Pool, username: &str) -> Result<String, AppError> {
    let db_conn = db_pool.get().await?;
    let db_params = (username.to_string(),);
    return db_conn
        .interact(|conn| {
            let res = conn.query_row(
                "SELECT passhash FROM users WHERE username = ?1",
                db_params,
                |row| row.get(0),
            );
            match res {
                Err(rusqlite::Error::QueryReturnedNoRows) => return Err(AppError::UserNotExist),
                Err(e) => return Err(e.into()),
                Ok(res) => return Ok(res),
            }
        })
        .await?;
}
