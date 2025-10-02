use axum::body::Body;
use axum::extract::Extension;
use axum::extract::ws::{Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::http::Response;

async fn handle_socket(mut socket: WebSocket, uid: u64) {
    socket
        .send(axum::extract::ws::Message::Text(Utf8Bytes::from(format!(
            "hello, user uid = {}",
            uid
        ))))
        .await
        .unwrap();
}

pub async fn handle_ws(ws: WebSocketUpgrade, Extension(uid): Extension<u64>) -> Response<Body> {
    return ws.on_upgrade(move |socket| handle_socket(socket, uid));
}
