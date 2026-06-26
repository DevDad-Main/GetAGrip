//! Global application state.

use std::sync::Arc;
use tg_config::Settings;
use tg_core::error::CoreResult;

/// The central application state shared across all components.
pub struct AppState {
    /// Application settings.
    pub settings: Arc<parking_lot::RwLock<Settings>>,
    /// Theme manager.
    pub theme_manager: Arc<tg_themes::ThemeManager>,
    /// Connection manager.
    pub connection_manager: Arc<tg_database::ConnectionManager>,
    /// Query engine.
    pub query_engine: Arc<tg_query_engine::QueryEngine>,
    /// Plugin host.
    pub plugin_host: Arc<tg_plugins::PluginHost>,
    /// Whether the application is quitting.
    pub quitting: parking_lot::RwLock<bool>,
    /// Current focused panel.
    pub focused_panel: parking_lot::RwLock<FocusedPanel>,
    /// Open tabs.
    pub tabs: dashmap::DashMap<tg_core::id::Id<tg_core::id::TabTag>, TabState>,
    /// Active tab ID.
    pub active_tab: parking_lot::RwLock<Option<tg_core::id::Id<tg_core::id::TabTag>>>,
    /// Command palette state.
    pub command_palette_open: parking_lot::RwLock<bool>,
    /// Command palette query text.
    pub command_palette_query: parking_lot::RwLock<String>,
    /// Command palette selected index.
    pub palette_selected: parking_lot::RwLock<usize>,
    /// Explorer tree state.
    pub explorer: parking_lot::RwLock<ExplorerState>,
    /// Sidebar width in columns.
    pub sidebar_width: parking_lot::RwLock<u16>,
    /// Editor/results split percentage (0-100).
    pub editor_split_pct: parking_lot::RwLock<u16>,
    /// Current notification message.
    pub notification: parking_lot::RwLock<Option<Notification>>,
}

/// Which panel currently has focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusedPanel {
    /// The query editor.
    Editor,
    /// The results grid.
    Results,
    /// The object explorer (sidebar).
    Explorer,
    /// The connection panel.
    Connections,
    /// The command palette.
    CommandPalette,
    /// Nothing focused.
    None,
}

/// A single editor tab.
#[derive(Clone, Debug)]
pub struct TabState {
    /// Display title.
    pub title: String,
    /// The SQL content.
    pub content: String,
    /// Whether this tab is pinned.
    pub pinned: bool,
    /// Whether the content has been modified since last save.
    pub dirty: bool,
    /// Associated connection ID.
    pub connection_id: Option<tg_core::types::connection::ConnectionId>,
    /// Cursor position (row, col).
    pub cursor: (usize, usize),
    /// Scroll position.
    pub scroll: usize,
}

/// A temporary user notification.
#[derive(Clone, Debug)]
pub struct Notification {
    /// Message text.
    pub message: String,
    /// Notification level.
    pub level: NotificationLevel,
    /// When the notification was created.
    pub created: std::time::Instant,
}

/// Notification severity level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Informational.
    Info,
    /// Success.
    Success,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

/// Explorer tree state for the sidebar.
#[derive(Clone, Debug)]
pub struct ExplorerState {
    /// The flat list of visible tree items.
    pub items: Vec<ExplorerItem>,
    /// Currently selected index.
    pub selected: usize,
}

/// A single node in the explorer tree.
#[derive(Clone, Debug)]
pub struct ExplorerItem {
    /// Display label.
    pub label: String,
    /// Indentation depth (0 = root).
    pub depth: u8,
    /// Whether this node is expanded.
    pub expanded: bool,
    /// The kind of item.
    pub kind: ExplorerItemKind,
    /// Associated connection ID (for connection nodes).
    pub connection_id: Option<tg_core::types::connection::ConnectionId>,
    /// Database name (for schema/table nodes).
    pub database: Option<String>,
    /// Schema name.
    pub schema: Option<String>,
    /// Table name (for column nodes).
    pub table: Option<String>,
}

/// Kind of explorer tree item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerItemKind {
    /// A connection entry.
    Connection,
    /// A database within a connection.
    Database,
    /// A schema.
    Schema,
    /// A table.
    Table,
    /// A view.
    View,
    /// A column.
    Column,
    /// A section header.
    Header,
}

impl AppState {
    /// Create a new application state.
    ///
    /// # Errors
    /// Returns an error if initialization fails.
    pub fn new(
        settings: Settings,
        theme_manager: Arc<tg_themes::ThemeManager>,
        connection_manager: Arc<tg_database::ConnectionManager>,
        query_engine: Arc<tg_query_engine::QueryEngine>,
        plugin_host: Arc<tg_plugins::PluginHost>,
    ) -> CoreResult<Self> {
        // Create a default query tab
        let tab_id = tg_core::id::Id::new();
        let tabs = dashmap::DashMap::new();
        tabs.insert(
            tab_id,
            TabState {
                title: "Query 1".into(),
                content: String::new(),
                pinned: false,
                dirty: false,
                connection_id: None,
                cursor: (0, 0),
                scroll: 0,
            },
        );

        Ok(Self {
            settings: Arc::new(parking_lot::RwLock::new(settings)),
            theme_manager,
            connection_manager,
            query_engine,
            plugin_host,
            quitting: parking_lot::RwLock::new(false),
            focused_panel: parking_lot::RwLock::new(FocusedPanel::Editor),
            tabs,
            active_tab: parking_lot::RwLock::new(Some(tab_id)),
            command_palette_open: parking_lot::RwLock::new(false),
            command_palette_query: parking_lot::RwLock::new(String::new()),
            palette_selected: parking_lot::RwLock::new(0),
            explorer: parking_lot::RwLock::new(ExplorerState {
                items: Vec::new(),
                selected: 0,
            }),
            sidebar_width: parking_lot::RwLock::new(38),
            editor_split_pct: parking_lot::RwLock::new(50),
            notification: parking_lot::RwLock::new(None),
        })
    }

    /// Get the currently active tab state.
    #[must_use]
    pub fn active_tab_state(&self) -> Option<dashmap::mapref::one::Ref<tg_core::id::Id<tg_core::id::TabTag>, TabState>> {
        let active = *self.active_tab.read();
        active.and_then(|id| self.tabs.get(&id))
    }

    /// Post a notification to the user.
    pub fn notify(&self, message: impl Into<String>, level: NotificationLevel) {
        *self.notification.write() = Some(Notification {
            message: message.into(),
            level,
            created: std::time::Instant::now(),
        });
    }

    /// Clear any stale notifications (older than 5 seconds).
    pub fn cleanup_notifications(&self) {
        let mut notif = self.notification.write();
        if let Some(ref n) = *notif {
            if n.created.elapsed().as_secs() > 5 {
                *notif = None;
            }
        }
    }

    /// Rebuild the explorer tree from the connection manager.
    pub fn refresh_explorer(&self) {
        let conns = self.connection_manager.list_connections().unwrap_or_default();
        let mut explorer = self.explorer.write();
        explorer.items.clear();

        for conn in &conns {
            let status_icon = match conn.status {
                tg_core::types::connection::ConnectionStatus::Connected => "●",
                _ => "○",
            };
            explorer.items.push(ExplorerItem {
                label: format!("{status_icon} {}", conn.name),
                depth: 0,
                expanded: false,
                kind: ExplorerItemKind::Connection,
                connection_id: Some(conn.id),
                database: conn.database.clone(),
                schema: conn.schema.clone(),
                table: None,
            });
        }

        if explorer.selected >= explorer.items.len() {
            explorer.selected = explorer.items.len().saturating_sub(1);
        }
    }
}
