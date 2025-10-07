use std::sync::Arc;

use deadpool_sqlite::{Pool, rusqlite};
use serde::Serialize;

use crate::error::AppError;
use crate::game::Game;

pub async fn init_db(db_pool: &Pool) -> Result<(), AppError> {
    let db_conn = db_pool.get().await?;
    return db_conn
        .interact(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS users(
                    uid INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT UNIQUE NOT NULL,
                    passhash TEXT NOT NULL,
                )",
                (),
            )?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS games(
                    game_id INTEGER PRIMARY KEY AUTOINCREMENT,
                )",
                (),
            )?;
            conn.execute(
                "CREATE TABLE IF NOT EXIST game_players(
                    game_id INTEGER NOT NULL,
                    uid INTEGER NOT NULL,
                    seat INTEGER NOT NULL,
                    score INTEGER NOT NULL,
                    rank INTEGER NOT NULL,
                )",
                (),
            )?;
            conn.execute(
                "CREATE TABLE IF NOT EXIST game_rounds(
                    game_id INTEGER NOT NULL,
                    round_id INTEGER NOT NULL,
                    stack TEXT NOT NULL,
                    winner_seat INTEGER,
                    loser_seat INTEGER,
                    discard TEXT NOT NULL,
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

pub async fn add_game(db_pool: &Pool, game: Arc<Game>) -> Result<usize, AppError> {
    let db_conn = db_pool.get().await?;
    return db_conn
        .interact(move |conn| {
            let tx = conn.transaction()?;

            let game_id =
                tx.query_one("INSERT INTO games RETURNING game_id", (), |row| row.get(0))?;

            let mut players = Vec::with_capacity(4);
            for i in 0..4 {
                players.push((game.players_score[i], game.players[i], i));
            }
            players.sort();
            for i in 0..4 {
                tx.execute(
                    "INSERT INTO game_players(game_id, uid, seat, score, rank)
                    VALUES (?1, ?2, ?3, ?4, ?5)",
                    (game_id, players[i].1, players[i].2, players[i].0, i),
                )?;
            }

            for i in 0..4 {
                #[derive(Serialize)]
                #[serde(transparent)]
                struct Helper<'a>(#[serde(with = "serde_bytes")] &'a [u8]);

                let stack = serde_json::to_string(&Helper(&game.round_records[i].stack))?;
                let discard = serde_json::to_string(&Helper(&game.round_records[i].discard))?;
                tx.execute(
                    "INSERT INTO game_rounds(game_id, round_id, stack, winner_seat, loser_seat, discard)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    (game_id, i, stack, game.round_records[i].winner_seat, game.round_records[i].loser_seat, discard)
                )?;
            }

            tx.commit()?;

            return Ok(game_id);
        })
        .await?;
}
