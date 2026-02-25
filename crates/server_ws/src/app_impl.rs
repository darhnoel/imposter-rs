use anyhow::Context;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use service::{Session, build_default_service};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::protocol::ClientEnvelope;
use crate::request::handle_request;
use crate::state::{AppState, SharedService};

pub fn app(service: SharedService) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health))
        .with_state(AppState { service })
}

pub fn default_app() -> Router {
    app(build_default_service())
}

pub async fn run_server(bind_addr: &str) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;
    info!("server_ws listening on {bind_addr}");
    axum::serve(listener, default_app())
        .await
        .context("server failed")
}

async fn health() -> &'static str {
    "ok"
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.service))
}

async fn handle_socket(socket: WebSocket, service: SharedService) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Value>();
    let mut game_event_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut chat_event_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut socket_session: Option<(String, String)> = None;

    loop {
        tokio::select! {
            maybe_msg = ws_rx.next() => {
                match maybe_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientEnvelope>(&text) {
                            Ok(req) => {
                                let op_name = req.op.clone();
                                let response = handle_request(
                                    req,
                                    &service,
                                    &event_tx,
                                    &mut game_event_task,
                                    &mut chat_event_task,
                                )
                                .await;
                                if response.get("ok").and_then(Value::as_bool) == Some(true) {
                                    match op_name.as_str() {
                                        "createRoom" | "joinRoom" => {
                                            if let Some(code) = response
                                                .get("data")
                                                .and_then(|d| d.get("room"))
                                                .and_then(|r| r.get("code"))
                                                .and_then(Value::as_str)
                                                && let Some(token) = response
                                                    .get("data")
                                                    .and_then(|d| d.get("token"))
                                                    .and_then(Value::as_str)
                                            {
                                                socket_session =
                                                    Some((code.to_string(), token.to_string()));
                                            }
                                        }
                                        "leaveRoom" => {
                                            socket_session = None;
                                        }
                                        _ => {}
                                    }
                                }
                                if ws_tx
                                    .send(Message::Text(response.to_string().into()))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(err) => {
                                let fallback = json!({
                                    "id": "unknown",
                                    "type": "response",
                                    "ok": false,
                                    "error": { "code": "BadRequest", "message": format!("invalid json request: {err}") }
                                });
                                if ws_tx
                                    .send(Message::Text(fallback.to_string().into()))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        warn!("ws receive error: {err}");
                        break;
                    }
                    None => break,
                }
            }
            Some(event_msg) = event_rx.recv() => {
                if ws_tx.send(Message::Text(event_msg.to_string().into())).await.is_err() {
                    break;
                }
            }
        }
    }

    if let Some(task) = game_event_task {
        task.abort();
    }
    if let Some(task) = chat_event_task {
        task.abort();
    }
    if let Some((room_code, token)) = socket_session {
        let _ = service.leave_room(room_code, Session::new(token));
    }
}
