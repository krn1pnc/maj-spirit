use axum::body::Body;
use axum::extract::Extension;
use axum::extract::ws::{Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::http::Response;

async fn handle_socket(mut socket: WebSocket, current_user: String) {
    socket
        .send(axum::extract::ws::Message::Text(Utf8Bytes::from(format!(
            "hello, {}",
            current_user
        ))))
        .await
        .unwrap();
}

pub async fn handle_ws(
    ws: WebSocketUpgrade,
    Extension(current_user): Extension<String>,
) -> Response<Body> {
    return ws.on_upgrade(|socket| handle_socket(socket, current_user));
}
