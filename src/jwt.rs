use std::time::SystemTime;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::config::{JWT_EXPIRE_DURATION, JWT_SECRET};
use crate::error::AppError;

#[derive(Clone, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    uid: u64,
}

pub fn get_token(uid: u64) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let token = encode(
        &Header::default(),
        &Claims {
            exp: now + JWT_EXPIRE_DURATION,
            uid: uid,
        },
        &EncodingKey::from_secret(JWT_SECRET),
    )?;
    return Ok(token);
}

pub fn verify_token(token: &str) -> Result<u64, AppError> {
    let token = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )?;
    return Ok(token.claims.uid);
}
