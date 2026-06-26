//! Connection configuration types.

use serde::{Deserialize, Serialize};
use crate::id::{ConnectionTag, Id};

/// A unique identifier for a saved connection.
pub type ConnectionId = Id<ConnectionTag>;

/// The kind of database this connection targets.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseKind {
    /// PostgreSQL.
    Postgres,
    /// MySQL.
    Mysql,
    /// MariaDB.
    MariaDb,
    /// SQLite.
    Sqlite,
    /// DuckDB.
    DuckDb,
    /// Microsoft SQL Server.
    SqlServer,
    /// Oracle Database.
    Oracle,
    /// ClickHouse.
    ClickHouse,
    /// Snowflake.
    Snowflake,
    /// Google BigQuery.
    BigQuery,
    /// Amazon Redshift.
    Redshift,
    /// MongoDB (read-only).
    MongoDb,
    /// Redis.
    Redis,
    /// CockroachDB.
    CockroachDb,
    /// Trino.
    Trino,
    /// Presto.
    Presto,
    /// A custom/unknown driver.
    Custom(String),
}

impl DatabaseKind {
    /// The default port for this database kind.
    #[must_use]
    pub fn default_port(self) -> u16 {
        match self {
            Self::Postgres | Self::CockroachDb | Self::Redshift => 5432,
            Self::Mysql | Self::MariaDb => 3306,
            Self::SqlServer => 1433,
            Self::Oracle => 1521,
            Self::ClickHouse => 8123,
            Self::MongoDb => 27017,
            Self::Redis => 6379,
            Self::Trino | Self::Presto => 8080,
            Self::Sqlite | Self::DuckDb | Self::Snowflake | Self::BigQuery | Self::Custom(_) => 0,
        }
    }

    /// Whether this kind supports the EXPLAIN command.
    #[must_use]
    pub fn supports_explain(self) -> bool {
        matches!(
            self,
            Self::Postgres
                | Self::Mysql
                | Self::MariaDb
                | Self::Sqlite
                | Self::DuckDb
                | Self::CockroachDb
                | Self::Redshift
                | Self::ClickHouse
        )
    }

    /// Human-readable display name.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::Mysql => "MySQL",
            Self::MariaDb => "MariaDB",
            Self::Sqlite => "SQLite",
            Self::DuckDb => "DuckDB",
            Self::SqlServer => "SQL Server",
            Self::Oracle => "Oracle",
            Self::ClickHouse => "ClickHouse",
            Self::Snowflake => "Snowflake",
            Self::BigQuery => "BigQuery",
            Self::Redshift => "Redshift",
            Self::MongoDb => "MongoDB",
            Self::Redis => "Redis",
            Self::CockroachDb => "CockroachDB",
            Self::Trino => "Trino",
            Self::Presto => "Presto",
            Self::Custom(name) => {
                // Leaking is fine here since it's a static string for display
                // In production, use a different approach
                let _ = name;
                "Custom"
            }
        }
    }
}

impl std::fmt::Display for DatabaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(name) => write!(f, "{name}"),
            other => write!(f, "{}", other.display_name()),
        }
    }
}

/// SSL/TLS connection mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SslMode {
    /// No SSL.
    #[default]
    Disable,
    /// Prefer SSL but allow non-SSL.
    Prefer,
    /// Require SSL.
    Require,
    /// Verify the CA certificate.
    VerifyCa,
    /// Verify the full certificate chain.
    VerifyFull,
}

/// SSH tunnel configuration for connecting through a bastion host.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// SSH host.
    pub host: String,
    /// SSH port.
    pub port: u16,
    /// SSH username.
    pub user: String,
    /// Authentication method.
    pub auth: TunnelAuth,
    /// Keep-alive interval in seconds.
    pub keepalive_secs: Option<u64>,
}

/// SSH authentication methods.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum TunnelAuth {
    /// Password authentication.
    Password { password: String },
    /// SSH key authentication.
    KeyFile {
        /// Path to the private key file.
        key_path: String,
        /// Optional passphrase.
        passphrase: Option<String>,
    },
    /// SSH agent forwarding.
    Agent,
}

/// Runtime status of a connection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// Not connected.
    #[default]
    Disconnected,
    /// Connecting in progress.
    Connecting,
    /// Connected and ready.
    Connected,
    /// Connection is broken.
    Error,
}

/// A complete connection configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Unique identifier.
    pub id: ConnectionId,
    /// Display name.
    pub name: String,
    /// Database kind.
    pub kind: DatabaseKind,
    /// Host address.
    pub host: String,
    /// Port number.
    pub port: u16,
    /// Database name.
    pub database: Option<String>,
    /// Username.
    pub user: Option<String>,
    /// Schema to use.
    pub schema: Option<String>,
    /// SSL mode.
    pub ssl_mode: SslMode,
    /// SSH tunnel configuration.
    pub tunnel: Option<TunnelConfig>,
    /// Connection color (for visual identification).
    pub color: Option<String>,
    /// Whether this is a favorite.
    pub favorite: bool,
    /// User-assigned folder path for organization.
    pub folder: Option<String>,
    /// Read-only mode.
    pub read_only: bool,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Query timeout in seconds (0 = no timeout).
    pub query_timeout_secs: u64,
    /// Current runtime status.
    #[serde(skip)]
    pub status: ConnectionStatus,
    /// When the connection was last used.
    #[serde(skip)]
    pub last_used: Option<crate::time::Timestamp>,
    /// User-defined tags.
    pub tags: Vec<String>,
    /// Arbitrary extra parameters for the driver.
    pub extra_params: std::collections::HashMap<String, String>,
}

impl ConnectionInfo {
    /// Create a new connection with minimal required fields.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: DatabaseKind, host: impl Into<String>, port: u16) -> Self {
        Self {
            id: ConnectionId::new(),
            name: name.into(),
            kind,
            host: host.into(),
            port,
            database: None,
            user: None,
            schema: None,
            ssl_mode: SslMode::default(),
            tunnel: None,
            color: None,
            favorite: false,
            folder: None,
            read_only: false,
            connect_timeout_secs: 30,
            query_timeout_secs: 0,
            status: ConnectionStatus::Disconnected,
            last_used: None,
            tags: Vec::new(),
            extra_params: std::collections::HashMap::new(),
        }
    }

    /// Build a connection URL string for this connection.
    #[must_use]
    pub fn connection_url(&self) -> String {
        match self.kind {
            DatabaseKind::Sqlite | DatabaseKind::DuckDb => {
                self.database.clone().unwrap_or_default()
            }
            DatabaseKind::Redis => {
                format!("redis://{}:{}/{}",
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("0"))
            }
            _ => {
                let db = self.database.as_deref().unwrap_or("");
                format!("{}://{}:{}/{}",
                    self.kind,
                    self.host,
                    self.port,
                    db)
            }
        }
    }
}

/// Minimal driver info returned by the driver registry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionDriver {
    /// The database kind this driver handles.
    pub kind: DatabaseKind,
    /// Human-readable name.
    pub name: String,
    /// Driver version.
    pub version: String,
    /// Whether the driver is loaded and ready.
    pub loaded: bool,
    /// Capability flags.
    pub capabilities: DriverCapabilities,
}

/// Bitfield of driver capabilities.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct DriverCapabilities {
    /// Supports EXPLAIN / EXPLAIN ANALYZE.
    pub explain: bool,
    /// Supports transactions.
    pub transactions: bool,
    /// Supports prepared statements.
    pub prepared_statements: bool,
    /// Supports streaming results.
    pub streaming: bool,
    /// Supports canceling running queries.
    pub cancel_query: bool,
    /// Supports multiple active result sets.
    pub multiple_result_sets: bool,
    /// Supports DDL introspection.
    pub introspection: bool,
    /// Supports write operations.
    pub read_write: bool,
    /// Supports SSH tunneling through this driver.
    pub ssh_tunneling: bool,
}
