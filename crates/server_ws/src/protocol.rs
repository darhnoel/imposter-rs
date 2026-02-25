use serde::Deserialize;
use serde_json::Value;
use service::{ServiceError, Session};
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientEnvelope {
    pub(crate) id: String,
    pub(crate) op: String,
    #[serde(default)]
    pub(crate) payload: Value,
    pub(crate) token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomCodePayload {
    pub(crate) room_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodePayload {
    pub(crate) code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateRoomPayload {
    pub(crate) code: String,
    pub(crate) nickname: String,
    #[serde(default)]
    pub(crate) r#public: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JoinRoomPayload {
    pub(crate) code: String,
    pub(crate) nickname: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetCategoryPayload {
    pub(crate) code: String,
    pub(crate) category: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuessPayload {
    pub(crate) code: String,
    pub(crate) guessed_player_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendChatPayload {
    pub(crate) code: String,
    pub(crate) text: String,
}

pub(crate) fn decode_payload<T: for<'de> Deserialize<'de>>(
    payload: Value,
) -> Result<T, ProtocolError> {
    serde_json::from_value(payload).map_err(|err| ProtocolError::bad_request(err.to_string()))
}

pub(crate) fn session_from(token: Option<String>) -> Result<Session, ProtocolError> {
    token
        .filter(|t| !t.trim().is_empty())
        .map(Session::new)
        .ok_or_else(|| ProtocolError::from(ServiceError::InvalidSession))
}

#[derive(Debug, Error)]
pub(crate) enum ProtocolError {
    #[error("{0}")]
    BadRequest(String),
    #[error(transparent)]
    Service(#[from] ServiceError),
}

impl ProtocolError {
    pub(crate) fn bad_request(message: String) -> Self {
        Self::BadRequest(message)
    }

    pub(crate) fn code(&self) -> String {
        match self {
            ProtocolError::BadRequest(_) => "BadRequest".to_string(),
            ProtocolError::Service(err) => err.code(),
        }
    }
}
