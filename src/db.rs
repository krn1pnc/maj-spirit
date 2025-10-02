use crate::error::AppError;
use deadpool_sqlite::{Pool, rusqlite};

pub async fn init_db(db_pool: &Pool) -> Result<(), AppError> {
    let db_conn = db_pool.get().await?;
    return db_conn
        .interact(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS users(
                    uid INTEGER PRIMARY KEY,
                    username TEXT UNIQUE,
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
            match conn.execute(
                "INSERT INTO users(username, passhash) VALUES (?1, ?2)",
                db_params,
            ) {
                Err(rusqlite::Error::SqliteFailure(info, s)) => {
                    // check username conflict
                    if info.extended_code == 2067 {
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

pub async fn get_username(db_pool: &Pool, uid: u64) -> Result<String, AppError> {
    let db_conn = db_pool.get().await?;
    let db_param = (uid,);
    return db_conn
        .interact(move |conn| {
            let res = conn.query_one(
                "SELECT username FROM users WHERE uid = ?1",
                db_param,
                |row| row.get(0),
            )?;
            return Ok(res);
        })
        .await?;
}

pub async fn verify_passhash(
    db_pool: &Pool,
    username: &str,
    passhash: &str,
) -> Result<u64, AppError> {
    let db_conn = db_pool.get().await?;
    let db_params = (username.to_string(),);
    let passhash = passhash.to_string();
    return db_conn
        .interact(move |conn| {
            let res = conn.query_one(
                "SELECT uid, passhash FROM users WHERE username = ?1",
                db_params,
                |row| Ok((row.get::<_, u64>(0)?, row.get::<_, String>(1)?)),
            );
            match res {
                Ok((uid, correct_passhash)) => {
                    if passhash == correct_passhash {
                        return Ok(uid);
                    } else {
                        return Err(AppError::PasswordIncorrect);
                    }
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => return Err(AppError::UserNotExist),
                Err(e) => return Err(e.into()),
            }
        })
        .await?;
}
