use thiserror::Error;

/// Stable domain errors used across transports.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("room already exists")]
    RoomAlreadyExists,
    #[error("room full")]
    RoomFull,
    #[error("invalid nickname")]
    InvalidNickname,
    #[error("nickname already exists")]
    NicknameTaken,
    #[error("operation allowed only in lobby")]
    NotInLobby,
    #[error("not enough players to start")]
    InsufficientPlayers,
    #[error("operation allowed only in progress")]
    NotInProgress,
    #[error("operation allowed only in completed state")]
    NotCompleted,
    #[error("category must be set before starting")]
    CategoryNotSet,
    #[error("invalid category")]
    InvalidCategory,
    #[error("player not found")]
    PlayerNotFound,
    #[error("caller is not a room member")]
    NotMember,
    #[error("caller is not room admin")]
    NotAdmin,
}

impl DomainError {
    /// Stable code for transport-level error mapping.
    pub fn code(&self) -> &'static str {
        match self {
            Self::RoomAlreadyExists => "RoomAlreadyExists",
            Self::RoomFull => "RoomFull",
            Self::InvalidNickname => "InvalidNickname",
            Self::NicknameTaken => "NicknameTaken",
            Self::NotInLobby => "NotInLobby",
            Self::InsufficientPlayers => "InsufficientPlayers",
            Self::NotInProgress => "NotInProgress",
            Self::NotCompleted => "NotCompleted",
            Self::CategoryNotSet => "CategoryNotSet",
            Self::InvalidCategory => "InvalidCategory",
            Self::PlayerNotFound => "PlayerNotFound",
            Self::NotMember => "NotMember",
            Self::NotAdmin => "NotAdmin",
        }
    }
}
