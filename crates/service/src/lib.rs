//! Service layer that mirrors future GraphQL operation names.
//!
//! Transport adapters (WS now, GraphQL later) should call only this API.

mod errors;
mod game_service;
mod store;
mod types;

pub use errors::ServiceError;
pub use game_service::{GameService, build_default_service};
pub use store::{InMemoryRoomStore, RoomEntry, RoomStore};
pub use types::{ChatMessage, PublicRoomSummary, Session};

#[cfg(test)]
mod tests;
