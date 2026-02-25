use std::sync::{Arc, Mutex};

use chrono::Utc;
use domain::{
    Command, DomainError, GameResult, GameSnapshot, PrivateRoleView, ProductionRng, RngLike,
    RoomView, SuspicionState, TopicCatalog, TurnState, apply, default_catalog, private_role_view,
};
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{ChatMessage, InMemoryRoomStore, PublicRoomSummary, RoomStore, ServiceError, Session};

/// Service interface mirroring future GraphQL operation names.
pub struct GameService<S: RoomStore> {
    store: S,
    catalog: TopicCatalog,
    rng: Mutex<Box<dyn RngLike>>,
}

impl<S: RoomStore> GameService<S> {
    /// Creates a service with production RNG and default catalog.
    pub fn new(store: S) -> Self {
        Self::with_rng_and_catalog(store, Box::new(ProductionRng), default_catalog())
    }

    /// Creates a service with custom rng/catalog, primarily for tests.
    pub fn with_rng_and_catalog(store: S, rng: Box<dyn RngLike>, catalog: TopicCatalog) -> Self {
        Self {
            store,
            catalog,
            rng: Mutex::new(rng),
        }
    }

    /// Query: categories
    pub fn categories(&self) -> Vec<String> {
        self.catalog.categories()
    }

    /// Query: gameSnapshot
    pub fn game_snapshot(&self, room_code: String) -> Result<GameSnapshot, ServiceError> {
        let entry = self
            .store
            .get_room(&room_code)
            .ok_or(ServiceError::RoomNotFound)?;
        let state = entry.state.lock().expect("room state lock poisoned");
        Ok(state.snapshot())
    }

    /// Query: myRole
    pub fn my_role(
        &self,
        room_code: String,
        session: Session,
    ) -> Result<PrivateRoleView, ServiceError> {
        let player_id = self.resolve_player_id(&room_code, &session)?;
        let entry = self
            .store
            .get_room(&room_code)
            .ok_or(ServiceError::RoomNotFound)?;
        let state = entry.state.lock().expect("room state lock poisoned");
        Ok(private_role_view(&state, &player_id)?)
    }

    /// Mutation: createRoom
    pub fn create_room(
        &self,
        code: String,
        nickname: String,
    ) -> Result<(RoomView, String), ServiceError> {
        self.create_room_with_visibility(code, nickname, false)
    }

    /// Mutation: createRoom with explicit public visibility.
    pub fn create_room_with_visibility(
        &self,
        code: String,
        nickname: String,
        is_public: bool,
    ) -> Result<(RoomView, String), ServiceError> {
        let token = Uuid::new_v4().to_string();
        let token_hash = hash_token(&token);
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        let (state, _) = apply(
            None,
            Command::CreateRoom {
                code: code.clone(),
                nickname,
                token_hash: token_hash.clone(),
            },
            &self.catalog,
            rng.as_mut(),
        )?;
        let admin_id = state.players[0].id.clone();
        let entry = self.store.create_room(&code, state.clone(), is_public)?;
        {
            let mut idx = entry.token_index.lock().expect("token index lock poisoned");
            idx.insert(token_hash, admin_id);
        }
        let _ = entry.game_updated_tx.send(state.snapshot());
        Ok((RoomView::from(&state), token))
    }

    /// Query: listRooms
    pub fn list_public_rooms(&self) -> Vec<PublicRoomSummary> {
        let mut rooms = self.store.list_rooms();
        rooms.sort_by_key(|entry| std::cmp::Reverse(entry.created_at));
        rooms
            .into_iter()
            .filter(|entry| entry.is_public)
            .map(|entry| {
                let state = entry.state.lock().expect("room state lock poisoned");
                let connected_players = state.players.iter().filter(|p| p.connected).count();
                let host_nickname = state
                    .players
                    .iter()
                    .find(|p| p.is_admin)
                    .or_else(|| state.players.first())
                    .map(|p| p.nickname.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                PublicRoomSummary {
                    code: state.code.clone(),
                    host_nickname,
                    phase: state.phase,
                    category: state.category.clone(),
                    connected_players,
                    total_players: state.players.len(),
                    joinable: state.phase == domain::GamePhase::Lobby && connected_players < 10,
                }
            })
            .collect()
    }

    /// Mutation: joinRoom
    pub fn join_room(
        &self,
        code: String,
        nickname: String,
    ) -> Result<(RoomView, String), ServiceError> {
        let token = Uuid::new_v4().to_string();
        let token_hash = hash_token(&token);
        let join_name = nickname.clone();
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        self.store.with_room_lock(&code, |state, token_index, tx| {
            let (new_state, _) = apply(
                Some(state),
                Command::JoinRoom {
                    nickname,
                    token_hash: token_hash.clone(),
                },
                &self.catalog,
                rng.as_mut(),
            )?;
            let player_id = new_state
                .players
                .iter()
                .find(|p| p.nickname.eq_ignore_ascii_case(&join_name) && p.connected)
                .map(|p| p.id.clone())
                .ok_or(ServiceError::Domain(DomainError::PlayerNotFound))?;
            *state = new_state.clone();
            token_index.insert(token_hash, player_id);
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot);
            Ok((RoomView::from(&*state), token))
        })
    }

    /// Mutation: leaveRoom
    pub fn leave_room(&self, code: String, session: Session) -> Result<RoomView, ServiceError> {
        let player_id = self.require_member(&code, &session)?;
        let token_hash = hash_token(&session.token);
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        self.store.with_room_lock(&code, |state, token_index, tx| {
            let (new_state, _) = apply(
                Some(state),
                Command::LeaveRoom {
                    player_id: player_id.clone(),
                },
                &self.catalog,
                rng.as_mut(),
            )?;
            *state = new_state;
            token_index.remove(&token_hash);
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot);
            Ok(RoomView::from(&*state))
        })
    }

    /// Mutation: setCategory
    pub fn set_category(
        &self,
        code: String,
        category: String,
        session: Session,
    ) -> Result<RoomView, ServiceError> {
        self.require_admin(&code, &session)?;
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        self.store.with_room_lock(&code, |state, _, tx| {
            let (new_state, _) = apply(
                Some(state),
                Command::SetCategory { category },
                &self.catalog,
                rng.as_mut(),
            )?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot);
            Ok(RoomView::from(&*state))
        })
    }

    /// Mutation: startGame
    pub fn start_game(&self, code: String, session: Session) -> Result<GameSnapshot, ServiceError> {
        self.require_admin(&code, &session)?;
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        let snapshot = self.store.with_room_lock(&code, |state, _, tx| {
            let (new_state, _) =
                apply(Some(state), Command::StartGame, &self.catalog, rng.as_mut())?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot.clone());
            Ok(snapshot)
        })?;
        self.clear_chat_history(&code)?;
        Ok(snapshot)
    }

    /// Mutation: nextTurn
    pub fn next_turn(&self, code: String, session: Session) -> Result<TurnState, ServiceError> {
        self.require_admin(&code, &session)?;
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        self.store.with_room_lock(&code, |state, _, tx| {
            let (new_state, _) =
                apply(Some(state), Command::NextTurn, &self.catalog, rng.as_mut())?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot.clone());
            snapshot
                .turn
                .ok_or(ServiceError::Domain(DomainError::NotInProgress))
        })
    }

    /// Mutation: guessImposter
    pub fn guess_imposter(
        &self,
        code: String,
        guessed_player_id: String,
        session: Session,
    ) -> Result<SuspicionState, ServiceError> {
        let player_id = self.require_member(&code, &session)?;
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        self.store.with_room_lock(&code, |state, _, tx| {
            let (new_state, _) = apply(
                Some(state),
                Command::GuessImposter {
                    player_id: player_id.clone(),
                    guessed_player_id,
                },
                &self.catalog,
                rng.as_mut(),
            )?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot.clone());
            snapshot
                .suspicions
                .into_iter()
                .find(|s| s.player_id == player_id)
                .ok_or(ServiceError::Domain(DomainError::NotInProgress))
        })
    }

    /// Mutation: revealResult
    pub fn reveal_result(
        &self,
        code: String,
        session: Session,
    ) -> Result<GameResult, ServiceError> {
        self.require_admin(&code, &session)?;
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        self.store.with_room_lock(&code, |state, _, tx| {
            let (new_state, _) = apply(
                Some(state),
                Command::RevealResult,
                &self.catalog,
                rng.as_mut(),
            )?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot.clone());
            snapshot
                .result
                .ok_or(ServiceError::Domain(DomainError::NotInProgress))
        })
    }

    /// Mutation: restartGame
    pub fn restart_game(
        &self,
        code: String,
        session: Session,
    ) -> Result<GameSnapshot, ServiceError> {
        self.require_admin(&code, &session)?;
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        let snapshot = self.store.with_room_lock(&code, |state, _, tx| {
            let (new_state, _) = apply(
                Some(state),
                Command::RestartGame,
                &self.catalog,
                rng.as_mut(),
            )?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot.clone());
            Ok(snapshot)
        })?;
        self.clear_chat_history(&code)?;
        Ok(snapshot)
    }

    /// Mutation: endGame
    pub fn end_game(&self, code: String, session: Session) -> Result<GameSnapshot, ServiceError> {
        self.require_admin(&code, &session)?;
        let mut rng = self.rng.lock().expect("rng lock poisoned");
        let snapshot = self.store.with_room_lock(&code, |state, _, tx| {
            let (new_state, _) = apply(Some(state), Command::EndGame, &self.catalog, rng.as_mut())?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = tx.send(snapshot.clone());
            Ok(snapshot)
        })?;
        self.clear_chat_history(&code)?;
        Ok(snapshot)
    }

    /// Subscription primitive used by WS transport to push `gameUpdated`.
    pub fn subscribe_game_updated(
        &self,
        code: String,
    ) -> Result<broadcast::Receiver<GameSnapshot>, ServiceError> {
        let entry = self
            .store
            .get_room(&code)
            .ok_or(ServiceError::RoomNotFound)?;
        Ok(entry.game_updated_tx.subscribe())
    }

    /// Query: chatHistory
    pub fn chat_history(
        &self,
        code: String,
        session: Session,
    ) -> Result<Vec<ChatMessage>, ServiceError> {
        self.require_member(&code, &session)?;
        let entry = self
            .store
            .get_room(&code)
            .ok_or(ServiceError::RoomNotFound)?;
        let chat = entry
            .chat_messages
            .lock()
            .expect("chat messages lock poisoned");
        Ok(chat.clone())
    }

    /// Mutation: sendChat
    pub fn send_chat(
        &self,
        code: String,
        text: String,
        session: Session,
    ) -> Result<ChatMessage, ServiceError> {
        let sender_player_id = self.require_member(&code, &session)?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(ServiceError::InvalidInput(
                "chat message cannot be empty".to_string(),
            ));
        }
        if trimmed.len() > 240 {
            return Err(ServiceError::InvalidInput(
                "chat message is too long (max 240 chars)".to_string(),
            ));
        }

        let entry = self
            .store
            .get_room(&code)
            .ok_or(ServiceError::RoomNotFound)?;

        let mut rng = self.rng.lock().expect("rng lock poisoned");
        let sender_nickname = {
            let mut state = entry.state.lock().expect("room state lock poisoned");
            if state.phase != domain::GamePhase::InProgress {
                return Err(ServiceError::Domain(DomainError::NotInProgress));
            }
            let round = state.round.as_ref().ok_or(DomainError::NotInProgress)?;
            let current_turn_player_id = &state.players[round.current_turn_index].id;
            if current_turn_player_id != &sender_player_id {
                return Err(ServiceError::InvalidInput(
                    "chat is allowed only for the current turn player".to_string(),
                ));
            }
            let sender_nickname = state
                .players
                .iter()
                .find(|p| p.id == sender_player_id)
                .map(|p| p.nickname.clone())
                .ok_or(ServiceError::Domain(DomainError::PlayerNotFound))?;

            let (new_state, _) =
                apply(Some(&state), Command::NextTurn, &self.catalog, rng.as_mut())?;
            *state = new_state;
            let snapshot = state.snapshot();
            let _ = entry.game_updated_tx.send(snapshot);
            sender_nickname
        };

        let message = ChatMessage {
            id: Uuid::new_v4().to_string(),
            room_code: code,
            sender_player_id,
            sender_nickname,
            text: trimmed.to_string(),
            created_at: Utc::now(),
        };

        {
            let mut history = entry
                .chat_messages
                .lock()
                .expect("chat messages lock poisoned");
            history.push(message.clone());
            if history.len() > 120 {
                let drain_count = history.len().saturating_sub(120);
                history.drain(0..drain_count);
            }
        }
        let _ = entry.chat_tx.send(message.clone());
        Ok(message)
    }

    /// Subscription primitive for transport adapters to push `chatMessage`.
    pub fn subscribe_chat_messages(
        &self,
        code: String,
    ) -> Result<broadcast::Receiver<ChatMessage>, ServiceError> {
        let entry = self
            .store
            .get_room(&code)
            .ok_or(ServiceError::RoomNotFound)?;
        Ok(entry.chat_tx.subscribe())
    }

    /// Returns topic metadata by id.
    pub fn topic_by_id(&self, topic_id: &str) -> Option<domain::Topic> {
        self.catalog.topic_by_id(topic_id).cloned()
    }

    fn require_admin(&self, room_code: &str, session: &Session) -> Result<String, ServiceError> {
        let player_id = self.resolve_player_id(room_code, session)?;
        let entry = self
            .store
            .get_room(room_code)
            .ok_or(ServiceError::RoomNotFound)?;
        let state = entry.state.lock().expect("room state lock poisoned");
        if !state.is_admin(&player_id) {
            return Err(ServiceError::Forbidden);
        }
        Ok(player_id)
    }

    fn require_member(&self, room_code: &str, session: &Session) -> Result<String, ServiceError> {
        let player_id = self.resolve_player_id(room_code, session)?;
        let entry = self
            .store
            .get_room(room_code)
            .ok_or(ServiceError::RoomNotFound)?;
        let state = entry.state.lock().expect("room state lock poisoned");
        if !state
            .players
            .iter()
            .any(|p| p.id == player_id && p.connected)
        {
            return Err(ServiceError::Forbidden);
        }
        Ok(player_id)
    }

    fn resolve_player_id(
        &self,
        room_code: &str,
        session: &Session,
    ) -> Result<String, ServiceError> {
        let entry = self
            .store
            .get_room(room_code)
            .ok_or(ServiceError::RoomNotFound)?;
        let token_hash = hash_token(&session.token);
        let idx = entry.token_index.lock().expect("token index lock poisoned");
        idx.get(&token_hash)
            .cloned()
            .ok_or(ServiceError::InvalidSession)
    }

    fn clear_chat_history(&self, room_code: &str) -> Result<(), ServiceError> {
        let entry = self
            .store
            .get_room(room_code)
            .ok_or(ServiceError::RoomNotFound)?;
        let mut chat = entry
            .chat_messages
            .lock()
            .expect("chat messages lock poisoned");
        chat.clear();
        Ok(())
    }
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Helper to build default shared service for server runtime.
pub fn build_default_service() -> Arc<GameService<InMemoryRoomStore>> {
    Arc::new(GameService::new(InMemoryRoomStore::default()))
}
