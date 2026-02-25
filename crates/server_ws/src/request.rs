use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::protocol::{
    ClientEnvelope, CodePayload, CreateRoomPayload, GuessPayload, JoinRoomPayload, ProtocolError,
    RoomCodePayload, SendChatPayload, SetCategoryPayload, decode_payload, session_from,
};
use crate::state::SharedService;
use crate::subscriptions::{subscribe_to_chat, subscribe_to_room};

pub(crate) async fn handle_request(
    req: ClientEnvelope,
    service: &SharedService,
    event_tx: &mpsc::UnboundedSender<Value>,
    game_event_task: &mut Option<tokio::task::JoinHandle<()>>,
    chat_event_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> Value {
    let id = req.id.clone();
    let op = req.op.clone();
    let result = match op.as_str() {
        "categories" => Ok(json!(service.categories())),
        "gameSnapshot" => decode_payload::<RoomCodePayload>(req.payload)
            .and_then(|p| service.game_snapshot(p.room_code).map_err(Into::into))
            .map(|snapshot| json!(snapshot)),
        "myRole" => decode_payload::<RoomCodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.my_role(p.room_code, session).map_err(Into::into)
            })
            .map(|role| json!(role)),
        "listRooms" => Ok(json!(service.list_public_rooms())),
        "createRoom" => decode_payload::<CreateRoomPayload>(req.payload)
            .and_then(|p| {
                service
                    .create_room_with_visibility(p.code.clone(), p.nickname, p.r#public)
                    .map_err(Into::into)
            })
            .and_then(|(room, token)| {
                subscribe_to_room(service, room.code.clone(), event_tx, game_event_task)?;
                subscribe_to_chat(service, room.code.clone(), event_tx, chat_event_task)?;
                Ok(json!({ "room": room, "token": token }))
            }),
        "joinRoom" => decode_payload::<JoinRoomPayload>(req.payload)
            .and_then(|p| {
                service
                    .join_room(p.code.clone(), p.nickname)
                    .map_err(Into::into)
            })
            .and_then(|(room, token)| {
                subscribe_to_room(service, room.code.clone(), event_tx, game_event_task)?;
                subscribe_to_chat(service, room.code.clone(), event_tx, chat_event_task)?;
                Ok(json!({ "room": room, "token": token }))
            }),
        "setCategory" => decode_payload::<SetCategoryPayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service
                    .set_category(p.code, p.category, session)
                    .map_err(Into::into)
            })
            .map(|room| json!(room)),
        "startGame" => decode_payload::<CodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.start_game(p.code, session).map_err(Into::into)
            })
            .map(|snapshot| json!(snapshot)),
        "nextTurn" => decode_payload::<CodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.next_turn(p.code, session).map_err(Into::into)
            })
            .map(|turn| json!(turn)),
        "guessImposter" => decode_payload::<GuessPayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service
                    .guess_imposter(p.code, p.guessed_player_id, session)
                    .map_err(Into::into)
            })
            .map(|suspicion| json!(suspicion)),
        "revealResult" => decode_payload::<CodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.reveal_result(p.code, session).map_err(Into::into)
            })
            .map(|result| json!(result)),
        "leaveRoom" => decode_payload::<CodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.leave_room(p.code, session).map_err(Into::into)
            })
            .map(|room| json!(room)),
        "restartGame" => decode_payload::<CodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.restart_game(p.code, session).map_err(Into::into)
            })
            .map(|snapshot| json!(snapshot)),
        "endGame" => decode_payload::<CodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.end_game(p.code, session).map_err(Into::into)
            })
            .map(|snapshot| json!(snapshot)),
        "chatHistory" => decode_payload::<CodePayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service.chat_history(p.code, session).map_err(Into::into)
            })
            .map(|messages| json!(messages)),
        "sendChat" => decode_payload::<SendChatPayload>(req.payload)
            .and_then(|p| {
                let session = session_from(req.token)?;
                service
                    .send_chat(p.code, p.text, session)
                    .map_err(Into::into)
            })
            .map(|message| json!(message)),
        _ => Err(ProtocolError::bad_request(format!(
            "unknown operation: {}",
            req.op
        ))),
    };

    match result {
        Ok(data) => json!({
            "id": id,
            "type": "response",
            "ok": true,
            "data": data
        }),
        Err(err) => json!({
            "id": id,
            "type": "response",
            "ok": false,
            "error": {
                "code": err.code(),
                "message": err.to_string(),
            }
        }),
    }
}
