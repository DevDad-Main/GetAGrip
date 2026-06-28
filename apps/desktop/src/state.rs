use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use getagrip_core::{ConnectionProfiles, EventBus, SecretsVault};
use getagrip_database::ConnectionManager;
use getagrip_intelligence::{LspManager, MetadataCache};
use getagrip_query_engine::QueryHistory;

pub struct AppState {
    pub profiles: RwLock<ConnectionProfiles>,
    pub vault: Arc<SecretsVault>,
    pub manager: Arc<ConnectionManager>,
    pub history: Arc<QueryHistory>,
    pub event_bus: Arc<EventBus>,
    pub metadata_cache: MetadataCache,
    pub lsp_manager: Arc<parking_lot::Mutex<LspManager>>,
    pub profiles_path: PathBuf,
    pub history_path: PathBuf,
}
