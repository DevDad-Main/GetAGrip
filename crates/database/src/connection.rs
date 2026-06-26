//! Connection manager — manages saved connections and their lifecycles.

use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use tg_core::cancel::CancellationToken;
use tg_core::error::{CoreError, CoreResult};
use tg_core::types::connection::ConnectionId;
use tg_core::time;
use tg_core::traits::driver::Connection as DriverConnection;
use tg_core::types::connection::{
    ConnectionInfo, ConnectionStatus, DatabaseKind, DriverCapabilities,
};
use tracing::{debug, error, info, warn};
use super::registry::DriverRegistry;

/// Manages the full lifecycle of database connections.
pub struct ConnectionManager {
    connections: DashMap<ConnectionId, ConnectionState>,
    registry: Arc<DriverRegistry>,
    history: RwLock<Vec<ConnectionId>>,
}

struct ConnectionState {
    info: ConnectionInfo,
    live: Option<Box<dyn DriverConnection>>,
}

impl ConnectionManager {
    /// Create a new connection manager with the given driver registry.
    #[must_use]
    pub fn new(registry: Arc<DriverRegistry>) -> Self {
        Self {
            connections: DashMap::new(),
            registry,
            history: RwLock::new(Vec::new()),
        }
    }

    /// Add a connection configuration without connecting.
    pub fn add_connection(&self, info: ConnectionInfo) -> CoreResult<ConnectionId> {
        let id = info.id;
        debug!(%id, name = %info.name, "Adding connection");

        self.connections.insert(
            id,
            ConnectionState {
                info,
                live: None,
            },
        );

        Ok(id)
    }

    /// Remove a connection by ID.
    pub fn remove_connection(&self, id: ConnectionId) -> CoreResult<()> {
        if let Some((_, state)) = self.connections.remove(&id) {
            info!(%id, "Removed connection");

            // Disconnect if live
            if let Some(conn) = state.live {
                tokio::spawn(async move {
                    if let Err(e) = conn.close().await {
                        warn!(%id, "Error closing connection during removal: {e}");
                    }
                });
            }
        }

        Ok(())
    }

    /// Get a connection's configuration.
    pub fn get_connection(&self, id: ConnectionId) -> CoreResult<ConnectionInfo> {
        self.connections
            .get(&id)
            .map(|state| state.info.clone())
            .ok_or_else(|| CoreError::connection(format!("Connection not found: {id}")))
    }

    /// List all saved connections.
    pub fn list_connections(&self) -> CoreResult<Vec<ConnectionInfo>> {
        let mut infos: Vec<ConnectionInfo> = self
            .connections
            .iter()
            .map(|entry| entry.info.clone())
            .collect();
        infos.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(infos)
    }

    /// Connect to a database and return a live connection handle.
    pub async fn connect(
        &self,
        id: ConnectionId,
        cancel: CancellationToken,
    ) -> CoreResult<Box<dyn DriverConnection>> {
        // Check if already connected
        if let Some(mut state) = self.connections.get_mut(&id) {
            if let Some(ref mut live) = state.live {
                if live.is_alive().await {
                    return Err(CoreError::connection("Already connected"));
                }
            }
        }

        let info = self.get_connection(id)?;

        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }

        let driver = self.registry.get(info.kind.clone()).ok_or_else(|| {
            CoreError::unsupported(format!("No driver registered for {}", info.kind.clone()))
        })?;

        info!(%id, name = %info.name, kind = %info.kind.clone(), "Connecting to database");

        let conn = driver.connect(&info, cancel.clone()).await.map_err(|e| {
            error!(%id, "Connection failed: {e}");
            CoreError::Connection { message: format!("Failed to connect to {}", info.name), source: Some(Box::new(e)) }
        })?;

        // Store the live connection
        if let Some(mut state) = self.connections.get_mut(&id) {
            state.info.status = ConnectionStatus::Connected;
            state.info.last_used = Some(time::now());
            // Move the connection into state — this requires some care
            // Since Connection is not Clone, we need to return a reference
            // For now, we store it and return a second connection
            state.live = None;
        }

        // Record in history
        self.history.write().push(id);

        Ok(conn)
    }

    /// Disconnect a live connection.
    pub async fn disconnect(&self, id: ConnectionId) -> CoreResult<()> {
        if let Some(mut state) = self.connections.get_mut(&id) {
            if let Some(conn) = state.live.take() {
                info!(%id, "Disconnecting");
                conn.close().await.map_err(|e| {
                    CoreError::Connection { message: "Failed to disconnect".into(), source: Some(Box::new(e)) }
                })?;
            }
            state.info.status = ConnectionStatus::Disconnected;
        }
        Ok(())
    }

    /// Get the current status of a connection.
    pub fn connection_status(&self, id: ConnectionId) -> CoreResult<ConnectionStatus> {
        self.connections
            .get(&id)
            .map(|state| state.info.status)
            .ok_or_else(|| CoreError::connection(format!("Connection not found: {id}")))
    }

    /// Test if a connection config is valid (ping latency in ms).
    pub async fn test_connection(&self, info: &ConnectionInfo) -> CoreResult<u64> {
        let driver = self.registry.get(info.kind.clone()).ok_or_else(|| {
            CoreError::unsupported(format!("No driver registered for {}", info.kind.clone()))
        })?;

        driver.ping(info).await
    }

    /// Get supported database kinds from loaded drivers.
    pub fn supported_kinds(&self) -> Vec<(DatabaseKind, DriverCapabilities)> {
        self.registry
            .all()
            .into_iter()
            .map(|d| (d.kind(), d.capabilities()))
            .collect()
    }

    /// Search connections by name, host, or tags.
    pub fn search(&self, query: &str) -> CoreResult<Vec<ConnectionInfo>> {
        let query = query.to_lowercase();
        let mut results: Vec<ConnectionInfo> = self
            .connections
            .iter()
            .filter(|entry| {
                let info = &entry.info;
                info.name.to_lowercase().contains(&query)
                    || info.host.to_lowercase().contains(&query)
                    || info.database.as_ref().is_some_and(|d| d.to_lowercase().contains(&query))
                    || info.tags.iter().any(|t| t.to_lowercase().contains(&query))
            })
            .map(|entry| entry.info.clone())
            .collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }

    /// Get connection history (most recently used first).
    pub fn get_history(&self) -> Vec<ConnectionInfo> {
        let history = self.history.read();
        history
            .iter()
            .rev()
            .filter_map(|id| self.connections.get(id).map(|s| s.info.clone()))
            .collect()
    }

    /// Move a connection to a different folder.
    pub fn move_to_folder(&self, id: ConnectionId, folder: Option<String>) -> CoreResult<()> {
        let mut state = self
            .connections
            .get_mut(&id)
            .ok_or_else(|| CoreError::connection("Connection not found"))?;
        state.info.folder = folder;
        Ok(())
    }

    /// Toggle a connection as favorite.
    pub fn toggle_favorite(&self, id: ConnectionId) -> CoreResult<bool> {
        let mut state = self
            .connections
            .get_mut(&id)
            .ok_or_else(|| CoreError::connection("Connection not found"))?;
        state.info.favorite = !state.info.favorite;
        Ok(state.info.favorite)
    }

    /// Get all favorites.
    pub fn list_favorites(&self) -> Vec<ConnectionInfo> {
        self.connections
            .iter()
            .filter(|e| e.info.favorite)
            .map(|e| e.info.clone())
            .collect()
    }

    /// Get connections in a specific folder.
    pub fn list_by_folder(&self, folder: Option<&str>) -> Vec<ConnectionInfo> {
        self.connections
            .iter()
            .filter(|e| {
                match (folder, &e.info.folder) {
                    (None, None) => true,
                    (Some(f), Some(info_f)) => f == info_f,
                    _ => false,
                }
            })
            .map(|e| e.info.clone())
            .collect()
    }
}
