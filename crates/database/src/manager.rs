//! Connection manager: maps [`ConnectionProfile`]s to live connections.
//!
//! The manager owns one [`ConnectionPool`] per active connection and tracks
//! connection state. It is the single source of truth for "what is connected
//! right now."

use std::sync::Arc;

use dashmap::DashMap;

use atlas_core::id::Id;
use atlas_core::session::{ConnectionDriver, ConnectionProfile};
use atlas_core::{AtlasError, AtlasResult};

use crate::pool::{ConnectionPool, PoolConfig};
use crate::driver::DatabaseDriver;

/// The state of a managed connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// No connection attempt has been made.
    Disconnected,
    /// A connection attempt is in progress.
    Connecting,
    /// Connected and ready.
    Connected,
    /// The connection failed with an error.
    Error,
    /// The connection was closed by the user.
    Closed,
}

/// A connection managed by the [`ConnectionManager`].
#[derive(Clone)]
pub struct ManagedConnection {
    /// The profile this connection was created from.
    pub profile: ConnectionProfile,
    /// Current state.
    pub state: ConnectionState,
    /// The connection pool (if connected).
    pub pool: Option<Arc<ConnectionPool>>,
    /// Last error message, if any.
    pub last_error: Option<String>,
}

/// Manages all live database connections.
///
/// Thread-safe: uses `DashMap` for concurrent access.
pub struct ConnectionManager {
    /// Active connections keyed by profile ID.
    connections: DashMap<String, ManagedConnection>,
}

impl ConnectionManager {
    /// Create an empty connection manager.
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    /// Connect using a profile. Returns the managed connection handle.
    ///
    /// If a connection for this profile already exists, it is returned
    /// as-is (unless in `Error` state, in which case a reconnect is
    /// attempted).
    pub async fn connect(
        &self,
        profile: &ConnectionProfile,
        driver: Arc<dyn DatabaseDriver>,
        pool_config: PoolConfig,
    ) -> AtlasResult<ManagedConnection> {
        let key = profile.id.to_string();

        // Check if we already have a live connection.
        if let Some(existing) = self.connections.get(&key) {
            if existing.state == ConnectionState::Connected {
                return Ok(existing.clone());
            }
            if existing.state == ConnectionState::Connecting {
                return Err(AtlasError::Connection {
                    source: profile.name.clone(),
                    reason: "connection already in progress".into(),
                    cause: None,
                });
            }
        }

        // Mark as connecting.
        self.connections.insert(
            key.clone(),
            ManagedConnection {
                profile: profile.clone(),
                state: ConnectionState::Connecting,
                pool: None,
                last_error: None,
            },
        );

        // Build connection URL.
        let url = build_url(profile)?;

        // Attempt connection.
        match driver.connect(&url).await {
            Ok(mut conn) => {
                conn.ping().await?;
                let pool = Arc::new(ConnectionPool::new(driver, url, pool_config));
                let managed = ManagedConnection {
                    profile: profile.clone(),
                    state: ConnectionState::Connected,
                    pool: Some(pool),
                    last_error: None,
                };
                self.connections.insert(key, managed.clone());
                Ok(managed)
            }
            Err(e) => {
                let managed = ManagedConnection {
                    profile: profile.clone(),
                    state: ConnectionState::Error,
                    pool: None,
                    last_error: Some(e.to_string()),
                };
                self.connections.insert(key, managed.clone());
                Err(e)
            }
        }
    }

    /// Disconnect a profile.
    pub async fn disconnect(&self, profile_id: Id<atlas_core::session::ConnectionProfileId>) {
        let key = profile_id.to_string();
        if let Some((_, conn)) = self.connections.remove(&key) {
            if let Some(pool) = conn.pool {
                pool.close();
            }
        }
    }

    /// Get a managed connection by profile ID.
    pub fn get(&self, profile_id: Id<atlas_core::session::ConnectionProfileId>) -> Option<ManagedConnection> {
        self.connections.get(&profile_id.to_string()).map(|r| r.clone())
    }

    /// List all managed connections.
    pub fn all(&self) -> Vec<ManagedConnection> {
        self.connections.iter().map(|r| r.clone()).collect()
    }

    /// Number of active connections.
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// Whether the manager has no connections.
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }

    /// Disconnect all profiles.
    pub async fn disconnect_all(&self) {
        let keys: Vec<String> = self.connections.iter().map(|r| r.key().clone()).collect();
        for key in keys {
            if let Some((_, conn)) = self.connections.remove(&key) {
                if let Some(pool) = conn.pool {
                    pool.close();
                }
            }
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a database URL from a connection profile.
fn build_url(profile: &ConnectionProfile) -> AtlasResult<String> {
    match profile.driver {
        ConnectionDriver::Sqlite => {
            Ok(format!("sqlite://{}", profile.host))
        }
        ConnectionDriver::Postgres => {
            Ok(format!(
                "postgres://{}:{}/{}",
                profile.host, profile.port,
                profile.database.as_deref().unwrap_or("postgres"),
            ))
        }
        ConnectionDriver::Mysql => {
            Ok(format!(
                "mysql://{}:{}/{}",
                profile.host, profile.port,
                profile.database.as_deref().unwrap_or("mysql"),
            ))
        }
        ConnectionDriver::Mssql => {
            Ok(format!(
                "mssql://{}:{}/{}",
                profile.host, profile.port,
                profile.database.as_deref().unwrap_or("master"),
            ))
        }
        _ => Ok(format!(
            "{}://{}:{}/{}",
            profile.driver.display_name().to_lowercase(),
            profile.host,
            profile.port,
            profile.database.as_deref().unwrap_or(""),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::session::ConnectionProfile;

    #[test]
    fn manager_starts_empty() {
        let mgr = ConnectionManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
        assert!(mgr.all().is_empty());
    }

    #[test]
    fn build_url_postgres() {
        let profile = ConnectionProfile::new("pg", ConnectionDriver::Postgres, "db.example.com");
        let url = build_url(&profile).unwrap();
        assert_eq!(url, "postgres://db.example.com:5432/postgres");
    }

    #[test]
    fn build_url_sqlite() {
        let profile = ConnectionProfile::new("sql", ConnectionDriver::Sqlite, ":memory:");
        let url = build_url(&profile).unwrap();
        assert_eq!(url, "sqlite://:memory:");
    }
}
