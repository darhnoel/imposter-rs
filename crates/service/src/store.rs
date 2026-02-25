use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use domain::{GameSnapshot, RoomState};
use tokio::sync::broadcast;

use crate::{ChatMessage, ServiceError};

/// Internal room entry with per-room serialization and subscriptions.
#[derive(Debug)]
pub struct RoomEntry {
    pub state: Mutex<RoomState>,
    pub game_updated_tx: broadcast::Sender<GameSnapshot>,
    pub chat_messages: Mutex<Vec<ChatMessage>>,
    pub chat_tx: broadcast::Sender<ChatMessage>,
    pub token_index: Mutex<HashMap<String, String>>,
    pub created_at: DateTime<Utc>,
    pub is_public: bool,
}

impl RoomEntry {
    fn new(room_state: RoomState, is_public: bool) -> Self {
        let (tx, _) = broadcast::channel(64);
        let (chat_tx, _) = broadcast::channel(128);
        Self {
            state: Mutex::new(room_state),
            game_updated_tx: tx,
            chat_messages: Mutex::new(Vec::new()),
            chat_tx,
            token_index: Mutex::new(HashMap::new()),
            created_at: Utc::now(),
            is_public,
        }
    }
}

/// Room store abstraction for migration-ready service boundary.
pub trait RoomStore: Send + Sync {
    fn create_room(
        &self,
        code: &str,
        room_state: RoomState,
        is_public: bool,
    ) -> Result<Arc<RoomEntry>, ServiceError>;
    fn get_room(&self, code: &str) -> Option<Arc<RoomEntry>>;
    fn list_rooms(&self) -> Vec<Arc<RoomEntry>>;
    fn with_room_lock<T, F>(&self, code: &str, f: F) -> Result<T, ServiceError>
    where
        F: FnOnce(
            &mut RoomState,
            &mut HashMap<String, String>,
            &broadcast::Sender<GameSnapshot>,
        ) -> Result<T, ServiceError>;
}

/// In-memory room store with per-room lock semantics.
#[derive(Debug, Default)]
pub struct InMemoryRoomStore {
    rooms: Mutex<HashMap<String, Arc<RoomEntry>>>,
}

impl RoomStore for InMemoryRoomStore {
    fn create_room(
        &self,
        code: &str,
        room_state: RoomState,
        is_public: bool,
    ) -> Result<Arc<RoomEntry>, ServiceError> {
        let mut rooms = self.rooms.lock().expect("rooms lock poisoned");
        if rooms.contains_key(code) {
            return Err(ServiceError::RoomAlreadyExists);
        }
        let entry = Arc::new(RoomEntry::new(room_state, is_public));
        rooms.insert(code.to_string(), entry.clone());
        Ok(entry)
    }

    fn get_room(&self, code: &str) -> Option<Arc<RoomEntry>> {
        let rooms = self.rooms.lock().expect("rooms lock poisoned");
        rooms.get(code).cloned()
    }

    fn list_rooms(&self) -> Vec<Arc<RoomEntry>> {
        let rooms = self.rooms.lock().expect("rooms lock poisoned");
        rooms.values().cloned().collect()
    }

    fn with_room_lock<T, F>(&self, code: &str, f: F) -> Result<T, ServiceError>
    where
        F: FnOnce(
            &mut RoomState,
            &mut HashMap<String, String>,
            &broadcast::Sender<GameSnapshot>,
        ) -> Result<T, ServiceError>,
    {
        let entry = self.get_room(code).ok_or(ServiceError::RoomNotFound)?;
        let mut state = entry.state.lock().expect("room state lock poisoned");
        let mut token_index = entry.token_index.lock().expect("token index lock poisoned");
        f(&mut state, &mut token_index, &entry.game_updated_tx)
    }
}
