//! Connection profiles and session management.
//!
//! The session module models the user's saved connection configurations,
//! organized into folders with optional tags, favorites, and environment
//! colors (matching the DataGrip UX).

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::Id;
use crate::secrets::SecretsVault;
use crate::AtlasResult;

// ---- Identifier tags ---------------------------------------------------

crate::id_tag!(pub ConnectionProfileId => "connection-profile");
crate::id_tag!(pub FolderId => "folder");

// ---- Connection driver --------------------------------------------------

/// Supported database drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionDriver {
    /// PostgreSQL
    Postgres,
    /// MySQL / MariaDB
    Mysql,
    /// SQLite
    Sqlite,
    /// Microsoft SQL Server
    Mssql,
    /// Oracle Database
    Oracle,
    /// MongoDB (via SQL translation layer)
    MongoDB,
    /// Redis
    Redis,
    /// Generic JDBC-compatible (future)
    Generic,
}

impl ConnectionDriver {
    /// Default port for the driver.
    pub fn default_port(&self) -> u16 {
        match self {
            Self::Postgres => 5432,
            Self::Mysql => 3306,
            Self::Sqlite => 0,
            Self::Mssql => 1433,
            Self::Oracle => 1521,
            Self::MongoDB => 27017,
            Self::Redis => 6379,
            Self::Generic => 0,
        }
    }

    /// Human-readable driver name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::Mysql => "MySQL",
            Self::Sqlite => "SQLite",
            Self::Mssql => "SQL Server",
            Self::Oracle => "Oracle",
            Self::MongoDB => "MongoDB",
            Self::Redis => "Redis",
            Self::Generic => "Generic",
        }
    }
}

// ---- Environment colors -------------------------------------------------

/// Environment color tags (matching DataGrip's UX).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentColor {
    /// Red — production.
    Red,
    /// Orange — staging.
    Orange,
    /// Yellow — QA.
    Yellow,
    /// Green — development.
    Green,
    /// Blue — testing.
    Blue,
    /// Purple — sandbox.
    Purple,
    /// No environment color.
    None,
}

impl Default for EnvironmentColor {
    fn default() -> Self {
        Self::None
    }
}

// ---- Credential types ---------------------------------------------------

/// How credentials are resolved for a connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Credential {
    /// Plain username/password (password stored in the vault).
    Password {
        /// Database username.
        username: String,
        /// Logical key in the [`SecretsVault`] for the password.
        vault_key: String,
    },
    /// No authentication (e.g. SQLite).
    None,
    /// TLS client certificate authentication.
    TlsCertificate {
        /// Database username.
        username: String,
        /// Path to the client certificate file.
        cert_path: PathBuf,
        /// Logical key in the vault for the private key passphrase.
        vault_key: Option<String>,
    },
    /// SSH agent forwarding + password.
    SshAgent {
        /// OS username for SSH.
        username: String,
        /// SSH host.
        host: String,
        /// SSH port.
        port: u16,
        /// Logical key in the vault for the SSH key passphrase.
        vault_key: Option<String>,
    },
}

/// Trait for resolving credentials from the vault.
pub trait CredentialStore: Send + Sync {
    /// Return the username, if any.
    fn username(&self) -> Option<&str>;

    /// Return the password, if applicable, by looking it up in the vault.
    fn resolve_password(&self, vault: &SecretsVault) -> AtlasResult<Option<String>>;

    /// Return the TLS certificate path (for TLS auth).
    fn tls_cert_path(&self) -> Option<&PathBuf>;
}

impl CredentialStore for Credential {
    fn username(&self) -> Option<&str> {
        match self {
            Self::Password { username, .. } => Some(username),
            Self::TlsCertificate { username, .. } => Some(username),
            Self::SshAgent { username, .. } => Some(username),
            Self::None => None,
        }
    }

    fn resolve_password(&self, vault: &SecretsVault) -> AtlasResult<Option<String>> {
        match self {
            Self::Password { vault_key, .. } => vault.get(vault_key),
            Self::SshAgent { vault_key, .. } => {
                if let Some(key) = vault_key {
                    vault.get(key)
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn tls_cert_path(&self) -> Option<&PathBuf> {
        match self {
            Self::TlsCertificate { cert_path, .. } => Some(cert_path),
            _ => None,
        }
    }
}

// ---- Connection profile -------------------------------------------------

/// A saved connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    /// Stable identifier.
    pub id: Id<ConnectionProfileId>,

    /// Human-readable display name.
    pub name: String,

    /// Database driver.
    pub driver: ConnectionDriver,

    /// Hostname or IP address.
    pub host: String,

    /// Port number.
    pub port: u16,

    /// Default database name.
    pub database: Option<String>,

    /// Credential configuration.
    pub credential: Credential,

    /// Whether to use TLS/SSL.
    pub use_tls: bool,

    /// Custom connection parameters (key-value).
    #[serde(default)]
    pub parameters: HashMap<String, String>,

    /// Parent folder id (for tree organisation).
    pub folder_id: Option<Id<FolderId>>,

    /// Environment color tag.
    #[serde(default)]
    pub environment: EnvironmentColor,

    /// User-assigned tags.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Whether this connection is pinned in the favorites list.
    #[serde(default)]
    pub favorite: bool,

    /// Connection comment / notes.
    #[serde(default)]
    pub notes: String,

    /// When the profile was created.
    pub created_at: DateTime<Utc>,

    /// When the profile was last modified.
    pub updated_at: DateTime<Utc>,

    /// When the profile was last successfully connected.
    pub last_connected_at: Option<DateTime<Utc>>,
}

impl ConnectionProfile {
    /// Create a new profile with sensible defaults.
    pub fn new(name: impl Into<String>, driver: ConnectionDriver, host: impl Into<String>) -> Self {
        let host = host.into();
        let port = driver.default_port();
        let now = Utc::now();
        Self {
            id: Id::new_v7(),
            name: name.into(),
            driver,
            host,
            port,
            database: None,
            credential: Credential::None,
            use_tls: false,
            parameters: HashMap::new(),
            folder_id: None,
            environment: EnvironmentColor::None,
            tags: Vec::new(),
            favorite: false,
            notes: String::new(),
            created_at: now,
            updated_at: now,
            last_connected_at: None,
        }
    }

    /// Build a connection URL string (suitable for display, not for secrets).
    pub fn display_url(&self) -> String {
        match self.driver {
            ConnectionDriver::Sqlite => {
                format!("sqlite://{}", self.host)
            }
            _ => {
                let db = self.database.as_deref().unwrap_or("");
                format!("{}://{}@{}:{}/{}",
                    self.driver_name(),
                    self.credential.username().unwrap_or(""),
                    self.host,
                    self.port,
                    db,
                )
            }
        }
    }

    fn driver_name(&self) -> &'static str {
        match self.driver {
            ConnectionDriver::Postgres => "postgres",
            ConnectionDriver::Mysql => "mysql",
            ConnectionDriver::Sqlite => "sqlite",
            ConnectionDriver::Mssql => "mssql",
            ConnectionDriver::Oracle => "oracle",
            ConnectionDriver::MongoDB => "mongodb",
            ConnectionDriver::Redis => "redis",
            ConnectionDriver::Generic => "generic",
        }
    }

    /// Mark this profile as having successfully connected.
    pub fn mark_connected(&mut self) {
        self.last_connected_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Toggle the favorite flag.
    pub fn toggle_favorite(&mut self) {
        self.favorite = !self.favorite;
        self.updated_at = Utc::now();
    }
}

// ---- Folder -------------------------------------------------------------

/// A folder for organising connection profiles in the explorer tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Stable identifier.
    pub id: Id<FolderId>,

    /// Display name.
    pub name: String,

    /// Parent folder (for nesting).
    pub parent_id: Option<Id<FolderId>>,

    /// Sort order within the parent.
    #[serde(default)]
    pub sort_order: u32,

    /// Whether the folder is collapsed in the tree.
    #[serde(default)]
    pub collapsed: bool,
}

impl Folder {
    /// Create a new folder.
    pub fn new(name: impl Into<String>, parent_id: Option<Id<FolderId>>) -> Self {
        Self {
            id: Id::new_v7(),
            name: name.into(),
            parent_id,
            sort_order: 0,
            collapsed: false,
        }
    }
}

// ---- Save rule ----------------------------------------------------------

/// Controls how passwords are persisted for a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveRule {
    /// Never save — prompt every time.
    Never,
    /// Save only for the current session.
    UntilExit,
    /// Save permanently to the vault.
    Forever,
}

impl Default for SaveRule {
    fn default() -> Self {
        Self::Forever
    }
}

// ---- Connection profiles collection -------------------------------------

/// A collection of connection profiles and folders.
///
/// This is the in-memory representation of the user's saved connections.
/// It is typically loaded from disk and persisted on change.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionProfiles {
    /// All saved connection profiles, keyed by id.
    #[serde(default)]
    pub profiles: HashMap<String, ConnectionProfile>,

    /// All saved folders, keyed by id.
    #[serde(default)]
    pub folders: HashMap<String, Folder>,
}

impl ConnectionProfiles {
    /// Create an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a profile to the collection.
    pub fn add(&mut self, profile: ConnectionProfile) {
        let key = profile.id.to_string();
        self.profiles.insert(key, profile);
    }

    /// Remove a profile by id.
    pub fn remove(&mut self, id: Id<ConnectionProfileId>) -> Option<ConnectionProfile> {
        self.profiles.remove(&id.to_string())
    }

    /// Look up a profile by id.
    pub fn get(&self, id: Id<ConnectionProfileId>) -> Option<&ConnectionProfile> {
        self.profiles.get(&id.to_string())
    }

    /// Look up a mutable profile by id.
    pub fn get_mut(&mut self, id: Id<ConnectionProfileId>) -> Option<&mut ConnectionProfile> {
        self.profiles.get_mut(&id.to_string())
    }

    /// Return profiles sorted by name.
    pub fn sorted(&self) -> Vec<&ConnectionProfile> {
        let mut v: Vec<_> = self.profiles.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Return favorited profiles.
    pub fn favorites(&self) -> Vec<&ConnectionProfile> {
        self.profiles.values().filter(|p| p.favorite).collect()
    }

    /// Return profiles in a given folder.
    pub fn in_folder(&self, folder_id: Id<FolderId>) -> Vec<&ConnectionProfile> {
        let id_str = folder_id.to_string();
        self.profiles
            .values()
            .filter(|p| p.folder_id.as_ref().map(|f| f.to_string()) == Some(id_str.clone()))
            .collect()
    }

    /// Add a folder.
    pub fn add_folder(&mut self, folder: Folder) {
        let key = folder.id.to_string();
        self.folders.insert(key, folder);
    }

    /// Remove a folder by id.
    pub fn remove_folder(&mut self, id: Id<FolderId>) -> Option<Folder> {
        self.folders.remove(&id.to_string())
    }

    /// Return top-level folders (no parent).
    pub fn root_folders(&self) -> Vec<&Folder> {
        self.folders.values().filter(|f| f.parent_id.is_none()).collect()
    }

    /// Return child folders of a given parent.
    pub fn child_folders(&self, parent_id: Id<FolderId>) -> Vec<&Folder> {
        let id_str = parent_id.to_string();
        self.folders
            .values()
            .filter(|f| f.parent_id.as_ref().map(|p| p.to_string()) == Some(id_str.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_connection_profile() {
        let profile = ConnectionProfile::new("My PG", ConnectionDriver::Postgres, "localhost");
        assert_eq!(profile.name, "My PG");
        assert_eq!(profile.port, 5432);
        assert_eq!(profile.driver, ConnectionDriver::Postgres);
        assert!(!profile.favorite);
    }

    #[test]
    fn display_url_redacts_password() {
        let mut profile = ConnectionProfile::new("pg", ConnectionDriver::Postgres, "db.example.com");
        profile.database = Some("mydb".into());
        profile.credential = Credential::Password {
            username: "admin".into(),
            vault_key: "pg_pass".into(),
        };
        let url = profile.display_url();
        // The URL should NOT contain the actual password.
        assert!(url.contains("postgres://admin@db.example.com:5432/mydb"));
    }

    #[test]
    fn connection_profiles_add_and_get() {
        let mut profiles = ConnectionProfiles::new();
        let p = ConnectionProfile::new("test", ConnectionDriver::Sqlite, ":memory:");
        let id = p.id;
        profiles.add(p);
        assert!(profiles.get(id).is_some());
    }

    #[test]
    fn connection_profiles_remove() {
        let mut profiles = ConnectionProfiles::new();
        let p = ConnectionProfile::new("test", ConnectionDriver::Sqlite, ":memory:");
        let id = p.id;
        profiles.add(p);
        assert!(profiles.remove(id).is_some());
        assert!(profiles.get(id).is_none());
    }

    #[test]
    fn favorites_filter() {
        let mut profiles = ConnectionProfiles::new();
        let mut p1 = ConnectionProfile::new("a", ConnectionDriver::Postgres, "h1");
        p1.favorite = true;
        let p2 = ConnectionProfile::new("b", ConnectionDriver::Postgres, "h2");
        profiles.add(p1);
        profiles.add(p2);
        assert_eq!(profiles.favorites().len(), 1);
    }

    #[test]
    fn folder_hierarchy() {
        let mut profiles = ConnectionProfiles::new();
        let root = Folder::new("Production", None);
        let root_id = root.id;
        profiles.add_folder(root);

        let child = Folder::new("EU", Some(root_id));
        profiles.add_folder(child);

        assert_eq!(profiles.root_folders().len(), 1);
        assert_eq!(profiles.child_folders(root_id).len(), 1);
    }

    #[test]
    fn mark_connected_updates_timestamp() {
        let mut profile = ConnectionProfile::new("pg", ConnectionDriver::Postgres, "localhost");
        assert!(profile.last_connected_at.is_none());
        profile.mark_connected();
        assert!(profile.last_connected_at.is_some());
    }

    #[test]
    fn driver_default_ports() {
        assert_eq!(ConnectionDriver::Postgres.default_port(), 5432);
        assert_eq!(ConnectionDriver::Mysql.default_port(), 3306);
        assert_eq!(ConnectionDriver::Sqlite.default_port(), 0);
        assert_eq!(ConnectionDriver::Mssql.default_port(), 1433);
        assert_eq!(ConnectionDriver::Oracle.default_port(), 1521);
    }
}
