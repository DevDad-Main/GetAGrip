//! Connection manager trait — manages the lifecycle of database connections.

use async_trait::async_trait;
use crate::cancel::CancellationToken;
use crate::error::CoreResult;
use crate::types::connection::ConnectionId;
use crate::traits::driver::Connection as DriverConnection;
use crate::types::connection::{ConnectionInfo, ConnectionStatus, DatabaseKind, DriverCapabilities};

/// Manages a collection of database connections and their lifecycles.
#[async_trait]
pub trait ConnectionManager: Send + Sync {
    /// Add a new connection configuration.
    async fn add_connection(&self, info: ConnectionInfo) -> CoreResult<ConnectionId>;

    /// Remove a connection by ID.
    async fn remove_connection(&self, id: ConnectionId) -> CoreResult<()>;

    /// Get a connection's configuration.
    async fn get_connection(&self, id: ConnectionId) -> CoreResult<ConnectionInfo>;

    /// List all saved connections.
    async fn list_connections(&self) -> CoreResult<Vec<ConnectionInfo>>;

    /// Connect to a database and return a live connection handle.
    async fn connect(
        &self,
        id: ConnectionId,
        cancel: CancellationToken,
    ) -> CoreResult<Box<dyn DriverConnection>>;

    /// Disconnect a live connection.
    async fn disconnect(&self, id: ConnectionId) -> CoreResult<()>;

    /// Get the current status of a connection.
    async fn connection_status(&self, id: ConnectionId) -> CoreResult<ConnectionStatus>;

    /// Test if a connection config is valid.
    async fn test_connection(&self, info: &ConnectionInfo) -> CoreResult<u64>;

    /// Reconnect a dropped connection.
    async fn reconnect(&self, id: ConnectionId, cancel: CancellationToken) -> CoreResult<()>;

    /// Duplicate an existing connection config.
    async fn duplicate_connection(&self, id: ConnectionId) -> CoreResult<ConnectionId>;

    /// Move a connection to a different folder.
    async fn move_to_folder(&self, id: ConnectionId, folder: Option<String>) -> CoreResult<()>;

    /// Toggle a connection as favorite.
    async fn toggle_favorite(&self, id: ConnectionId) -> CoreResult<bool>;

    /// Get all connections in a folder.
    async fn list_by_folder(&self, folder: Option<&str>) -> CoreResult<Vec<ConnectionInfo>>;

    /// Get all favorites.
    async fn list_favorites(&self) -> CoreResult<Vec<ConnectionInfo>>;

    /// Import connections from a file (JSON, TOML, YAML).
    async fn import_connections(&self, path: &str) -> CoreResult<Vec<ConnectionId>>;

    /// Export connections to a file.
    async fn export_connections(&self, path: &str, ids: &[ConnectionId]) -> CoreResult<()>;

    /// Get supported database kinds from loaded drivers.
    fn supported_kinds(&self) -> Vec<(DatabaseKind, DriverCapabilities)>;

    /// Search connections by name, host, or tags.
    async fn search(&self, query: &str) -> CoreResult<Vec<ConnectionInfo>>;
}
