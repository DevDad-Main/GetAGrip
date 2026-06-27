use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use getagrip_core::{ConnectionProfiles, EventBus, SecretsVault};
use getagrip_database::ConnectionManager;
use getagrip_query_engine::QueryHistory;

pub struct AppState {
    pub profiles: RwLock<ConnectionProfiles>,
    pub vault: Arc<SecretsVault>,
    pub manager: Arc<ConnectionManager>,
    pub history: Arc<QueryHistory>,
    pub event_bus: Arc<EventBus>,
    pub profiles_path: PathBuf,
    pub history_path: PathBuf,
}
