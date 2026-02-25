//! Pure domain model and state transitions for the Imposter MVP.
//!
//! This crate owns game invariants and deterministic transitions.
//! It has no networking and no async runtime dependencies.

mod commands;
mod errors;
mod model;
mod rng;
mod role;
mod topics;
mod transitions;

pub use commands::{Command, DomainEvent};
pub use errors::DomainError;
pub use model::{
    GamePhase, GameResult, GameRole, GameSnapshot, PlayerState, PrivateRoleView, RoomState,
    RoomView, RoundState, SuspicionState, TurnState, Winner,
};
pub use rng::{FixedRng, ProductionRng, RngLike};
pub use role::private_role_view;
pub use topics::{Topic, TopicCatalog, default_catalog};
pub use transitions::apply;

#[cfg(test)]
mod tests;
