use std::collections::HashMap;

use crate::{
    Command, DomainError, DomainEvent, GamePhase, GameResult, PlayerState, RngLike, RoomState,
    RoundState, SuspicionState, TopicCatalog, Winner,
};

const MAX_ROOM_PLAYERS: usize = 10;
const MIN_PLAYERS_TO_START: usize = 3;
const MAX_NICKNAME_LEN: usize = 24;

/// Applies a command and returns a new state and emitted events.
///
/// `state` is `None` only for `Command::CreateRoom`.
pub fn apply(
    state: Option<&RoomState>,
    command: Command,
    catalog: &TopicCatalog,
    rng: &mut dyn RngLike,
) -> Result<(RoomState, Vec<DomainEvent>), DomainError> {
    match command {
        Command::CreateRoom {
            code,
            nickname,
            token_hash: _,
        } => {
            if state.is_some() {
                return Err(DomainError::RoomAlreadyExists);
            }
            validate_nickname(&nickname)?;
            let created = RoomState {
                code,
                phase: GamePhase::Lobby,
                category: None,
                players: vec![PlayerState {
                    id: "p1".to_string(),
                    nickname,
                    is_admin: true,
                    connected: true,
                }],
                round: None,
                result: None,
            };
            Ok((created, vec![DomainEvent::RoomCreated]))
        }
        Command::JoinRoom {
            nickname,
            token_hash: _,
        } => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase != GamePhase::Lobby {
                return Err(DomainError::NotInLobby);
            }
            validate_nickname(&nickname)?;
            if connected_player_indices(&room.players).len() >= MAX_ROOM_PLAYERS {
                return Err(DomainError::RoomFull);
            }
            if room
                .players
                .iter()
                .any(|p| p.nickname.eq_ignore_ascii_case(&nickname) && p.connected)
            {
                return Err(DomainError::NicknameTaken);
            }
            if let Some(existing) = room
                .players
                .iter_mut()
                .find(|p| p.nickname.eq_ignore_ascii_case(&nickname) && !p.connected)
            {
                existing.connected = true;
                let player_id = existing.id.clone();
                return Ok((room, vec![DomainEvent::PlayerReconnected { player_id }]));
            }
            let player_id = format!("p{}", room.players.len() + 1);
            room.players.push(PlayerState {
                id: player_id.clone(),
                nickname,
                is_admin: false,
                connected: true,
            });
            Ok((room, vec![DomainEvent::PlayerJoined { player_id }]))
        }
        Command::LeaveRoom { player_id } => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            let idx = room
                .players
                .iter()
                .position(|p| p.id == player_id)
                .ok_or(DomainError::PlayerNotFound)?;
            if !room.players[idx].connected {
                return Ok((room, Vec::new()));
            }
            let was_admin = room.players[idx].is_admin;
            room.players[idx].connected = false;
            room.players[idx].is_admin = false;

            if was_admin && let Some(next_admin) = room.players.iter_mut().find(|p| p.connected) {
                next_admin.is_admin = true;
            }

            let imposter_player_id = room.round.as_ref().map(|r| r.imposter_player_id.clone());
            if room.phase == GamePhase::InProgress
                && imposter_player_id.as_deref() == Some(room.players[idx].id.as_str())
            {
                let imposter_player_id = imposter_player_id.expect("imposter exists in progress");
                room.phase = GamePhase::Completed;
                room.result = Some(GameResult {
                    winner: Winner::Crew,
                    guessed_player_id: Some(imposter_player_id.clone()),
                    imposter_player_id,
                });
            }

            let next_turn = if room.phase == GamePhase::InProgress {
                room.round.as_ref().and_then(|round| {
                    if room.players[round.current_turn_index].connected {
                        None
                    } else {
                        next_connected_index(&room.players, round.current_turn_index)
                    }
                })
            } else {
                None
            };
            if let Some(next_turn) = next_turn
                && let Some(round) = room.round.as_mut()
            {
                round.current_turn_index = next_turn;
            }

            Ok((room, vec![DomainEvent::PlayerLeft { player_id }]))
        }
        Command::SetCategory { category } => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase != GamePhase::Lobby {
                return Err(DomainError::NotInLobby);
            }
            if !catalog.contains_category(&category) {
                return Err(DomainError::InvalidCategory);
            }
            room.category = Some(category.clone());
            Ok((room, vec![DomainEvent::CategorySet { category }]))
        }
        Command::StartGame => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase != GamePhase::Lobby {
                return Err(DomainError::NotInLobby);
            }
            let round_number = prepare_next_round(&mut room, catalog, rng)?;
            Ok((room, vec![DomainEvent::RoundStarted { round_number }]))
        }
        Command::NextTurn => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase != GamePhase::InProgress {
                return Err(DomainError::NotInProgress);
            }
            let current_turn_index = {
                let round = room.round.as_mut().ok_or(DomainError::NotInProgress)?;
                // Wrap-around guarantees deterministic cyclical turns.
                round.current_turn_index =
                    next_connected_index(&room.players, round.current_turn_index)
                        .ok_or(DomainError::NotInProgress)?;
                round.current_turn_index
            };
            Ok((room, vec![DomainEvent::TurnAdvanced { current_turn_index }]))
        }
        Command::GuessImposter {
            player_id,
            guessed_player_id,
        } => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase != GamePhase::InProgress {
                return Err(DomainError::NotInProgress);
            }
            if !room
                .players
                .iter()
                .any(|p| p.id == player_id && p.connected)
            {
                return Err(DomainError::PlayerNotFound);
            }
            if !room
                .players
                .iter()
                .any(|p| p.id == guessed_player_id && p.connected)
            {
                return Err(DomainError::PlayerNotFound);
            }
            let round = room.round.as_mut().ok_or(DomainError::NotInProgress)?;
            if let Some(existing) = round
                .suspicions
                .iter_mut()
                .find(|s| s.player_id == player_id)
            {
                existing.guessed_player_id = guessed_player_id.clone();
            } else {
                round.suspicions.push(SuspicionState {
                    player_id: player_id.clone(),
                    guessed_player_id: guessed_player_id.clone(),
                });
            }
            Ok((
                room,
                vec![DomainEvent::SuspicionSubmitted {
                    player_id,
                    guessed_player_id,
                }],
            ))
        }
        Command::RevealResult => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase != GamePhase::InProgress {
                return Err(DomainError::NotInProgress);
            }
            let round = room.round.as_ref().ok_or(DomainError::NotInProgress)?;
            let guessed_player_id = most_suspected_player_id(&round.suspicions);
            let winner = if guessed_player_id.as_deref() == Some(round.imposter_player_id.as_str())
            {
                Winner::Crew
            } else {
                Winner::Imposter
            };
            room.phase = GamePhase::Completed;
            room.result = Some(GameResult {
                winner,
                guessed_player_id,
                imposter_player_id: round.imposter_player_id.clone(),
            });
            Ok((room, vec![DomainEvent::GameRevealed { winner }]))
        }
        Command::RestartGame => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase != GamePhase::Completed {
                return Err(DomainError::NotCompleted);
            }
            let round_number = prepare_next_round(&mut room, catalog, rng)?;
            Ok((room, vec![DomainEvent::GameRestarted { round_number }]))
        }
        Command::EndGame => {
            let mut room = state.cloned().ok_or(DomainError::PlayerNotFound)?;
            if room.phase == GamePhase::Lobby {
                return Err(DomainError::NotInProgress);
            }
            room.phase = GamePhase::Lobby;
            room.round = None;
            room.result = None;
            Ok((room, vec![DomainEvent::GameEnded]))
        }
    }
}

fn validate_nickname(nickname: &str) -> Result<(), DomainError> {
    let trimmed = nickname.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_NICKNAME_LEN {
        return Err(DomainError::InvalidNickname);
    }
    Ok(())
}

fn prepare_next_round(
    room: &mut RoomState,
    catalog: &TopicCatalog,
    rng: &mut dyn RngLike,
) -> Result<u32, DomainError> {
    let connected = connected_player_indices(&room.players);
    if connected.len() < MIN_PLAYERS_TO_START {
        return Err(DomainError::InsufficientPlayers);
    }
    let category = room.category.clone().ok_or(DomainError::CategoryNotSet)?;
    let topics = catalog
        .topics_in_category(&category)
        .ok_or(DomainError::InvalidCategory)?;
    let imposter_idx = connected[rng.choose_imposter(connected.len())];
    let topic_idx = rng.choose_topic(topics.len());
    let round_number = room.round.as_ref().map_or(1, |r| r.round_number + 1);
    room.phase = GamePhase::InProgress;
    room.result = None;
    room.round = Some(RoundState {
        round_number,
        current_turn_index: connected[0],
        imposter_player_id: room.players[imposter_idx].id.clone(),
        topic_id: topics[topic_idx].id.to_string(),
        suspicions: Vec::new(),
    });
    Ok(round_number)
}

fn connected_player_indices(players: &[PlayerState]) -> Vec<usize> {
    players
        .iter()
        .enumerate()
        .filter_map(|(idx, p)| p.connected.then_some(idx))
        .collect()
}

fn next_connected_index(players: &[PlayerState], current: usize) -> Option<usize> {
    if players.is_empty() {
        return None;
    }
    for step in 1..=players.len() {
        let idx = (current + step) % players.len();
        if players[idx].connected {
            return Some(idx);
        }
    }
    None
}

fn most_suspected_player_id(suspicions: &[SuspicionState]) -> Option<String> {
    if suspicions.is_empty() {
        return None;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for suspicion in suspicions {
        *counts
            .entry(suspicion.guessed_player_id.as_str())
            .or_insert(0) += 1;
    }
    let mut sorted: Vec<(&str, usize)> = counts.into_iter().collect();
    sorted.sort_by(|(id_a, count_a), (id_b, count_b)| {
        count_b.cmp(count_a).then_with(|| id_a.cmp(id_b))
    });
    sorted.first().map(|(id, _)| (*id).to_string())
}
