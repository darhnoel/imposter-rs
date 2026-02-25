use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Client session token wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub token: String,
}

impl Session {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

/// Public board listing entry used by lobby discovery UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRoomSummary {
    pub code: String,
    pub host_nickname: String,
    pub phase: domain::GamePhase,
    pub category: Option<String>,
    pub connected_players: usize,
    pub total_players: usize,
    pub joinable: bool,
}

/// Chat message payload shared across WS/GraphQL transports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub room_code: String,
    pub sender_player_id: String,
    pub sender_nickname: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}
