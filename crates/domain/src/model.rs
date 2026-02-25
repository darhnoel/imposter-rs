use serde::{Deserialize, Serialize};

/// Current room phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GamePhase {
    Lobby,
    InProgress,
    Completed,
}

/// Per-player hidden role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GameRole {
    Crew,
    Imposter,
}

/// Winner role for completed round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Winner {
    Crew,
    Imposter,
}

/// Stable room state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomState {
    pub code: String,
    pub phase: GamePhase,
    pub category: Option<String>,
    pub players: Vec<PlayerState>,
    pub round: Option<RoundState>,
    pub result: Option<GameResult>,
}

/// Public player state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub id: String,
    pub nickname: String,
    pub is_admin: bool,
    pub connected: bool,
}

/// Active round metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundState {
    pub round_number: u32,
    pub current_turn_index: usize,
    pub imposter_player_id: String,
    pub topic_id: String,
    pub suspicions: Vec<SuspicionState>,
}

/// Per-player suspicion state submitted during a live round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspicionState {
    pub player_id: String,
    pub guessed_player_id: String,
}

/// Completed game result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameResult {
    pub winner: Winner,
    pub guessed_player_id: Option<String>,
    pub imposter_player_id: String,
}

/// Private role data for one caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateRoleView {
    pub game_role: GameRole,
    pub category: String,
    pub topic_id: Option<String>,
}

/// Transport-facing room view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomView {
    pub code: String,
    pub category: Option<String>,
    pub phase: GamePhase,
    pub players: Vec<PlayerState>,
}

/// Transport-facing turn data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnState {
    pub round: u32,
    pub current_turn_index: usize,
    pub current_player_id: String,
}

/// Transport-facing snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSnapshot {
    pub room: RoomView,
    pub turn: Option<TurnState>,
    pub suspicions: Vec<SuspicionState>,
    pub result: Option<GameResult>,
}

impl From<&RoomState> for RoomView {
    fn from(value: &RoomState) -> Self {
        Self {
            code: value.code.clone(),
            category: value.category.clone(),
            phase: value.phase,
            players: value.players.clone(),
        }
    }
}

impl RoomState {
    /// Returns transport snapshot used by both WS protocol and future GraphQL responses.
    pub fn snapshot(&self) -> GameSnapshot {
        let turn = self.round.as_ref().map(|round| TurnState {
            round: round.round_number,
            current_turn_index: round.current_turn_index,
            current_player_id: self.players[round.current_turn_index].id.clone(),
        });
        let suspicions = self
            .round
            .as_ref()
            .map(|round| round.suspicions.clone())
            .unwrap_or_default();
        GameSnapshot {
            room: RoomView::from(self),
            turn,
            suspicions,
            result: self.result.clone(),
        }
    }

    /// Returns true when the given player id is the room admin.
    pub fn is_admin(&self, player_id: &str) -> bool {
        self.players
            .iter()
            .any(|p| p.id == player_id && p.is_admin && p.connected)
    }
}
