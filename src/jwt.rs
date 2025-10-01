use std::time::SystemTime;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::config::{JWT_EXPIRE_DURATION, JWT_SECRET};
use crate::error::AppError;

#[derive(Clone, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    name: String,
}

pub fn get_token(username: &str) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let token = encode(
        &Header::default(),
        &Claims {
            exp: now + JWT_EXPIRE_DURATION,
            name: username.to_string(),
        },
        &EncodingKey::from_secret(JWT_SECRET),
    )?;
    return Ok(token);
}

pub fn verify_token(token: &str) -> Result<String, AppError> {
    let token = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )?;
    return Ok(token.claims.name);
}
