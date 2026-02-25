use domain::DomainError;
use thiserror::Error;

/// Stable service-level errors consumed by transports.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ServiceError {
    #[error("room not found")]
    RoomNotFound,
    #[error("room already exists")]
    RoomAlreadyExists,
    #[error("invalid session token")]
    InvalidSession,
    #[error("{0}")]
    InvalidInput(String),
    #[error("caller is not authorized")]
    Forbidden,
    #[error("{0}")]
    Domain(#[from] DomainError),
}

impl ServiceError {
    pub fn code(&self) -> String {
        match self {
            ServiceError::RoomNotFound => "RoomNotFound".to_string(),
            ServiceError::RoomAlreadyExists => "RoomAlreadyExists".to_string(),
            ServiceError::InvalidSession => "InvalidSession".to_string(),
            ServiceError::InvalidInput(_) => "InvalidInput".to_string(),
            ServiceError::Forbidden => "Forbidden".to_string(),
            ServiceError::Domain(err) => err.code().to_string(),
        }
    }
}
