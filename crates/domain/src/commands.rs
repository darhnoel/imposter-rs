use crate::Winner;

/// Domain command enum used with [`crate::apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    CreateRoom {
        code: String,
        nickname: String,
        token_hash: String,
    },
    JoinRoom {
        nickname: String,
        token_hash: String,
    },
    LeaveRoom {
        player_id: String,
    },
    SetCategory {
        category: String,
    },
    StartGame,
    NextTurn,
    GuessImposter {
        player_id: String,
        guessed_player_id: String,
    },
    RevealResult,
    RestartGame,
    EndGame,
}

/// Domain events for higher layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    RoomCreated,
    PlayerJoined {
        player_id: String,
    },
    PlayerLeft {
        player_id: String,
    },
    PlayerReconnected {
        player_id: String,
    },
    CategorySet {
        category: String,
    },
    RoundStarted {
        round_number: u32,
    },
    TurnAdvanced {
        current_turn_index: usize,
    },
    SuspicionSubmitted {
        player_id: String,
        guessed_player_id: String,
    },
    GameRevealed {
        winner: Winner,
    },
    GameRestarted {
        round_number: u32,
    },
    GameEnded,
}
