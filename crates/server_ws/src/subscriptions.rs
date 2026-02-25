use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::warn;

use crate::protocol::ProtocolError;
use crate::state::SharedService;

pub(crate) fn subscribe_to_room(
    service: &SharedService,
    room_code: String,
    event_tx: &mpsc::UnboundedSender<Value>,
    event_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<(), ProtocolError> {
    if let Some(task) = event_task.take() {
        task.abort();
    }

    let mut rx = service
        .subscribe_game_updated(room_code.clone())
        .map_err(ProtocolError::from)?;
    let tx = event_tx.clone();
    if let Ok(snapshot) = service.game_snapshot(room_code.clone()) {
        let initial = json!({
            "type": "event",
            "event": "gameUpdated",
            "code": room_code,
            "snapshot": snapshot,
        });
        let _ = tx.send(initial);
    }

    let room_code_for_task = room_code.clone();
    *event_task = Some(tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(snapshot) => {
                    let msg = json!({
                        "type": "event",
                        "event": "gameUpdated",
                        "code": room_code_for_task,
                        "snapshot": snapshot,
                    });
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!("subscription lagged; skipped {skipped} snapshots");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    warn!("subscription closed");
                    break;
                }
            }
        }
    }));
    Ok(())
}

pub(crate) fn subscribe_to_chat(
    service: &SharedService,
    room_code: String,
    event_tx: &mpsc::UnboundedSender<Value>,
    event_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<(), ProtocolError> {
    if let Some(task) = event_task.take() {
        task.abort();
    }

    let mut rx = service
        .subscribe_chat_messages(room_code.clone())
        .map_err(ProtocolError::from)?;
    let tx = event_tx.clone();
    let room_code_for_task = room_code.clone();
    *event_task = Some(tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(message) => {
                    let msg = json!({
                        "type": "event",
                        "event": "chatMessage",
                        "code": room_code_for_task,
                        "message": message,
                    });
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!("chat subscription lagged; skipped {skipped} messages");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    warn!("chat subscription closed");
                    break;
                }
            }
        }
    }));
    Ok(())
}
