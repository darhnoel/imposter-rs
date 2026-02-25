use axum::serve;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::default_app;

async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let app = default_app();
    let handle = tokio::spawn(async move {
        if let Err(err) = serve(listener, app).await {
            tracing::error!("test server failed: {err}");
        }
    });
    (format!("ws://{}/ws", addr), handle)
}

async fn send_op(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    id: &str,
    op: &str,
    payload: Value,
    token: Option<String>,
) -> Value {
    let req = json!({
        "id": id,
        "op": op,
        "payload": payload,
        "token": token,
    });
    ws.send(Message::Text(req.to_string().into()))
        .await
        .expect("send");
    loop {
        let next = timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("timely response")
            .expect("ws msg")
            .expect("ws ok");
        let Message::Text(text) = next else {
            continue;
        };
        let value: Value = serde_json::from_str(&text).expect("json response");
        if value.get("type").and_then(Value::as_str) == Some("response")
            && value.get("id").and_then(Value::as_str) == Some(id)
        {
            return value;
        }
    }
}

async fn recv_event(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Value {
    loop {
        let next = timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("event timeout")
            .expect("ws msg")
            .expect("ws ok");
        let Message::Text(text) = next else {
            continue;
        };
        let value: Value = serde_json::from_str(&text).expect("json");
        if value.get("type").and_then(Value::as_str) == Some("event") {
            return value;
        }
    }
}

async fn recv_event_named(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    event_name: &str,
) -> Value {
    for _ in 0..12 {
        let event = recv_event(ws).await;
        if event["event"].as_str() == Some(event_name) {
            return event;
        }
    }
    panic!("did not receive event {event_name}");
}

async fn recv_event_with_player_count(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    expected_players: usize,
) -> Value {
    for _ in 0..8 {
        let event = recv_event(ws).await;
        let players = event["snapshot"]["room"]["players"]
            .as_array()
            .map(Vec::len);
        if players == Some(expected_players) {
            return event;
        }
    }
    panic!("did not receive gameUpdated with expected player count {expected_players}");
}

#[tokio::test]
async fn auth_gating_non_admin_rejected_for_set_category() {
    let (url, _server) = start_test_server().await;
    let (mut ws, _) = connect_async(url).await.expect("connect");
    let create = send_op(
        &mut ws,
        "1",
        "createRoom",
        json!({"code":"ABCD","nickname":"Host"}),
        None,
    )
    .await;
    let admin_token = create["data"]["token"]
        .as_str()
        .expect("admin token")
        .to_string();
    let _ = admin_token;
    let join = send_op(
        &mut ws,
        "2",
        "joinRoom",
        json!({"code":"ABCD","nickname":"Alice"}),
        None,
    )
    .await;
    let user_token = join["data"]["token"]
        .as_str()
        .expect("user token")
        .to_string();
    let denied = send_op(
        &mut ws,
        "3",
        "setCategory",
        json!({"code":"ABCD","category":"Animals"}),
        Some(user_token),
    )
    .await;
    assert_eq!(denied["ok"], Value::Bool(false));
    assert_eq!(
        denied["error"]["code"],
        Value::String("Forbidden".to_string())
    );
}

#[tokio::test]
async fn guess_imposter_rejected_when_not_in_progress() {
    let (url, _server) = start_test_server().await;
    let (mut ws, _) = connect_async(url).await.expect("connect");
    let _ = send_op(
        &mut ws,
        "1",
        "createRoom",
        json!({"code":"ABCD","nickname":"Host"}),
        None,
    )
    .await;
    let join = send_op(
        &mut ws,
        "2",
        "joinRoom",
        json!({"code":"ABCD","nickname":"Alice"}),
        None,
    )
    .await;
    let token = join["data"]["token"].as_str().expect("token").to_string();
    let guessed = send_op(
        &mut ws,
        "3",
        "guessImposter",
        json!({"code":"ABCD","guessedPlayerId":"p1"}),
        Some(token),
    )
    .await;
    assert_eq!(guessed["ok"], Value::Bool(false));
    assert_eq!(
        guessed["error"]["code"],
        Value::String("NotInProgress".to_string())
    );
}

#[tokio::test]
async fn my_role_requires_matching_token_identity() {
    let (url, _server) = start_test_server().await;
    let (mut ws, _) = connect_async(url).await.expect("connect");
    let _ = send_op(
        &mut ws,
        "1",
        "createRoom",
        json!({"code":"ABCD","nickname":"Host"}),
        None,
    )
    .await;
    let response = send_op(
        &mut ws,
        "2",
        "myRole",
        json!({"roomCode":"ABCD"}),
        Some("bad-token".to_string()),
    )
    .await;
    assert_eq!(response["ok"], Value::Bool(false));
    assert_eq!(
        response["error"]["code"],
        Value::String("InvalidSession".to_string())
    );
}

#[tokio::test]
async fn broadcast_event_emitted_after_join() {
    let (url, _server) = start_test_server().await;
    let (mut ws_admin, _) = connect_async(url.clone()).await.expect("admin connect");
    let (mut ws_observer, _) = connect_async(url).await.expect("observer connect");

    let _ = send_op(
        &mut ws_admin,
        "1",
        "createRoom",
        json!({"code":"ABCD","nickname":"Host"}),
        None,
    )
    .await;

    // Observer joins first; this establishes a room subscription on observer socket.
    let _ = send_op(
        &mut ws_observer,
        "2",
        "joinRoom",
        json!({"code":"ABCD","nickname":"Alice"}),
        None,
    )
    .await;

    // Admin triggers another join, observer should receive gameUpdated with 3 players.
    let _ = send_op(
        &mut ws_admin,
        "3",
        "joinRoom",
        json!({"code":"ABCD","nickname":"Bob"}),
        None,
    )
    .await;

    let event = recv_event_with_player_count(&mut ws_observer, 3).await;
    assert_eq!(event["event"], Value::String("gameUpdated".to_string()));
    assert_eq!(
        event["snapshot"]["room"]["players"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
}

#[tokio::test]
async fn e2e_flow_create_join_start_guess_reveal_restart_end() {
    let (url, _server) = start_test_server().await;
    let (mut ws, _) = connect_async(url).await.expect("connect");
    let create = send_op(
        &mut ws,
        "1",
        "createRoom",
        json!({"code":"ABCD","nickname":"Host"}),
        None,
    )
    .await;
    let admin_token = create["data"]["token"]
        .as_str()
        .expect("admin token")
        .to_string();

    let _ = send_op(
        &mut ws,
        "2",
        "joinRoom",
        json!({"code":"ABCD","nickname":"Alice"}),
        None,
    )
    .await;
    let _ = send_op(
        &mut ws,
        "3",
        "joinRoom",
        json!({"code":"ABCD","nickname":"Bob"}),
        None,
    )
    .await;
    let _ = send_op(
        &mut ws,
        "4",
        "setCategory",
        json!({"code":"ABCD","category":"Countries"}),
        Some(admin_token.clone()),
    )
    .await;
    let started = send_op(
        &mut ws,
        "5",
        "startGame",
        json!({"code":"ABCD"}),
        Some(admin_token.clone()),
    )
    .await;
    assert_eq!(
        started["data"]["room"]["phase"],
        Value::String("IN_PROGRESS".to_string())
    );
    let next = send_op(
        &mut ws,
        "6",
        "nextTurn",
        json!({"code":"ABCD"}),
        Some(admin_token.clone()),
    )
    .await;
    assert_eq!(next["ok"], Value::Bool(true));
    let guessed = send_op(
        &mut ws,
        "7",
        "guessImposter",
        json!({"code":"ABCD","guessedPlayerId":"p2"}),
        Some(admin_token.clone()),
    )
    .await;
    assert_eq!(guessed["ok"], Value::Bool(true));
    let revealed = send_op(
        &mut ws,
        "8",
        "revealResult",
        json!({"code":"ABCD"}),
        Some(admin_token.clone()),
    )
    .await;
    assert_eq!(revealed["ok"], Value::Bool(true));
    let restarted = send_op(
        &mut ws,
        "9",
        "restartGame",
        json!({"code":"ABCD"}),
        Some(admin_token.clone()),
    )
    .await;
    assert_eq!(
        restarted["data"]["room"]["phase"],
        Value::String("IN_PROGRESS".to_string())
    );
    let ended = send_op(
        &mut ws,
        "10",
        "endGame",
        json!({"code":"ABCD"}),
        Some(admin_token),
    )
    .await;
    assert_eq!(
        ended["data"]["room"]["phase"],
        Value::String("LOBBY".to_string())
    );
}

#[tokio::test]
async fn send_chat_rejected_when_not_in_progress() {
    let (url, _server) = start_test_server().await;
    let (mut ws, _) = connect_async(url).await.expect("connect");
    let create = send_op(
        &mut ws,
        "1",
        "createRoom",
        json!({"code":"CHAT","nickname":"Host"}),
        None,
    )
    .await;
    let token = create["data"]["token"].as_str().expect("token").to_string();
    let send = send_op(
        &mut ws,
        "2",
        "sendChat",
        json!({"code":"CHAT","text":"hello"}),
        Some(token),
    )
    .await;
    assert_eq!(send["ok"], Value::Bool(false));
    assert_eq!(
        send["error"]["code"],
        Value::String("NotInProgress".to_string())
    );
}

#[tokio::test]
async fn send_chat_rejected_when_not_current_turn() {
    let (url, _server) = start_test_server().await;
    let (mut ws_admin, _) = connect_async(url.clone()).await.expect("admin connect");
    let (mut ws_player, _) = connect_async(url).await.expect("player connect");

    let create = send_op(
        &mut ws_admin,
        "1",
        "createRoom",
        json!({"code":"TURNCHAT","nickname":"Host"}),
        None,
    )
    .await;
    let admin_token = create["data"]["token"]
        .as_str()
        .expect("admin token")
        .to_string();
    let join = send_op(
        &mut ws_player,
        "2",
        "joinRoom",
        json!({"code":"TURNCHAT","nickname":"Alice"}),
        None,
    )
    .await;
    let player_token = join["data"]["token"]
        .as_str()
        .expect("player token")
        .to_string();
    let _ = send_op(
        &mut ws_admin,
        "3",
        "joinRoom",
        json!({"code":"TURNCHAT","nickname":"Bob"}),
        None,
    )
    .await;
    let _ = send_op(
        &mut ws_admin,
        "4",
        "setCategory",
        json!({"code":"TURNCHAT","category":"Countries"}),
        Some(admin_token.clone()),
    )
    .await;
    let _ = send_op(
        &mut ws_admin,
        "5",
        "startGame",
        json!({"code":"TURNCHAT"}),
        Some(admin_token),
    )
    .await;
    let send = send_op(
        &mut ws_player,
        "6",
        "sendChat",
        json!({"code":"TURNCHAT","text":"off-turn"}),
        Some(player_token),
    )
    .await;
    assert_eq!(send["ok"], Value::Bool(false));
    assert_eq!(
        send["error"]["code"],
        Value::String("InvalidInput".to_string())
    );
}

#[tokio::test]
async fn chat_event_emitted_with_sender_and_text() {
    let (url, _server) = start_test_server().await;
    let (mut ws_admin, _) = connect_async(url.clone()).await.expect("admin connect");
    let (mut ws_player, _) = connect_async(url).await.expect("player connect");

    let create = send_op(
        &mut ws_admin,
        "1",
        "createRoom",
        json!({"code":"CHAT","nickname":"Host"}),
        None,
    )
    .await;
    let admin_token = create["data"]["token"]
        .as_str()
        .expect("admin token")
        .to_string();

    let join = send_op(
        &mut ws_player,
        "2",
        "joinRoom",
        json!({"code":"CHAT","nickname":"Alice"}),
        None,
    )
    .await;
    let player_token = join["data"]["token"]
        .as_str()
        .expect("player token")
        .to_string();
    let _ = send_op(
        &mut ws_admin,
        "3",
        "joinRoom",
        json!({"code":"CHAT","nickname":"Bob"}),
        None,
    )
    .await;

    let _ = send_op(
        &mut ws_admin,
        "4",
        "setCategory",
        json!({"code":"CHAT","category":"Countries"}),
        Some(admin_token.clone()),
    )
    .await;
    let _ = send_op(
        &mut ws_admin,
        "5",
        "startGame",
        json!({"code":"CHAT"}),
        Some(admin_token),
    )
    .await;
    let _ = send_op(
        &mut ws_admin,
        "6",
        "nextTurn",
        json!({"code":"CHAT"}),
        create["data"]["token"].as_str().map(ToString::to_string),
    )
    .await;
    let _ = send_op(
        &mut ws_player,
        "7",
        "sendChat",
        json!({"code":"CHAT","text":"Alice clue"}),
        Some(player_token),
    )
    .await;

    let event = recv_event_named(&mut ws_admin, "chatMessage").await;
    assert_eq!(
        event["message"]["senderNickname"],
        Value::String("Alice".to_string())
    );
    assert_eq!(
        event["message"]["text"],
        Value::String("Alice clue".to_string())
    );
}

#[tokio::test]
async fn disconnect_marks_player_as_disconnected() {
    let (url, _server) = start_test_server().await;
    let (mut ws_admin, _) = connect_async(url.clone()).await.expect("admin connect");
    let (mut ws_player, _) = connect_async(url).await.expect("player connect");

    let create = send_op(
        &mut ws_admin,
        "1",
        "createRoom",
        json!({"code":"ABCD","nickname":"Host"}),
        None,
    )
    .await;
    let admin_token = create["data"]["token"]
        .as_str()
        .expect("admin token")
        .to_string();

    let _ = send_op(
        &mut ws_player,
        "2",
        "joinRoom",
        json!({"code":"ABCD","nickname":"Alice"}),
        None,
    )
    .await;
    let _ = ws_player.close(None).await;

    let mut found_disconnected = false;
    for _ in 0..10 {
        let snapshot = send_op(
            &mut ws_admin,
            "3",
            "gameSnapshot",
            json!({"roomCode":"ABCD"}),
            Some(admin_token.clone()),
        )
        .await;
        let players = snapshot["data"]["room"]["players"]
            .as_array()
            .expect("players array");
        if players.iter().any(|p| {
            p["nickname"].as_str() == Some("Alice") && p["connected"].as_bool() == Some(false)
        }) {
            found_disconnected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        found_disconnected,
        "player should be marked disconnected after socket close"
    );
}
