use std::sync::Arc;

use service::{GameService, InMemoryRoomStore};

pub type SharedService = Arc<GameService<InMemoryRoomStore>>;

#[derive(Clone)]
pub struct AppState {
    pub service: SharedService,
}
