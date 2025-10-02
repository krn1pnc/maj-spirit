use std::sync::Arc;

use axum::routing::{any, get, post};
use axum::{Router, middleware};
use deadpool_sqlite::{Config, Runtime};

use maj_spirit::config::{DATABASE_FILE, LISTEN_ADDR};
use maj_spirit::{handle_hello, handle_login, handle_register, handle_ws, init_db, jwt_auth};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_cfg = Config::new(DATABASE_FILE);
    let db_pool = Arc::new(db_cfg.create_pool(Runtime::Tokio1).unwrap());
    init_db(&db_pool).await.unwrap();

    let app = Router::new()
        .route("/hello", get(handle_hello))
        .route("/ws", any(handle_ws))
        .route_layer(middleware::from_fn(jwt_auth))
        .route("/register", post(handle_register))
        .route("/login", post(handle_login))
        .with_state(db_pool);

    let listener = tokio::net::TcpListener::bind(LISTEN_ADDR).await.unwrap();
    tracing::info!("listening at {}", LISTEN_ADDR);

    axum::serve(listener, app).await.unwrap();
}
