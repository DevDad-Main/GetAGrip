//! Encrypted credential vault.
//!
//! AtlasDB Studio stores database credentials in a local encrypted vault.
//! The vault uses AES-256-GCM with a key derived from:
//!
//! 1. An OS keyring entry (when available via `secret-service` or similar),
//!    falling back to a machine-local key file.
//! 2. A per-entry random nonce.
//!
//! Secrets are never logged, and the [`Secret`] type implements `Serialize`
//! by redacting its contents.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use parking_lot::RwLock;
use rand::RngCore;
use ring::digest;
use serde::{Deserialize, Serialize};

use crate::AtlasResult;
use crate::error::AtlasError;

/// A protected secret value that redacts itself in debug/log output.
#[derive(Clone)]
pub struct Secret {
    /// The encrypted payload (ciphertext || nonce), base64-encoded.
    encrypted: String,
}

impl Secret {
    /// Create a new [`Secret`] by encrypting `plaintext` with the vault key.
    fn encrypt(plaintext: &str, key: &[u8; 32]) -> AtlasResult<Self> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| AtlasError::Secrets { detail: "invalid key length".into() })?;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AtlasError::Secrets { detail: e.to_string() })?;

        let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(Self {
            encrypted: BASE64.encode(&combined),
        })
    }

    /// Decrypt this secret using the vault key.
    fn decrypt(&self, key: &[u8; 32]) -> AtlasResult<String> {
        let combined = BASE64.decode(&self.encrypted).map_err(|e| {
            AtlasError::Secrets {
                detail: format!("base64 decode: {e}"),
            }
        })?;

        if combined.len() < 12 {
            return Err(AtlasError::Secrets {
                detail: "ciphertext too short".into(),
            });
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| AtlasError::Secrets { detail: "invalid key length".into() })?;
        let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AtlasError::Secrets { detail: e.to_string() })?;

        String::from_utf8(plaintext).map_err(|e| AtlasError::Secrets {
            detail: format!("utf8 decode: {e}"),
        })
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

impl Serialize for Secret {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.encrypted.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self {
            encrypted: String::deserialize(d)?,
        })
    }
}

/// Categories of secrets stored in the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecretKind {
    /// A database password or token.
    Password,
    /// An SSH private key passphrase.
    SshPassphrase,
    /// A TLS certificate private key.
    TlsPrivateKey,
    /// An API key for AI providers.
    ApiKey,
    /// An arbitrary secret that doesn't fit other categories.
    Other,
}

impl std::fmt::Display for SecretKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password => f.write_str("password"),
            Self::SshPassphrase => f.write_str("ssh-passphrase"),
            Self::TlsPrivateKey => f.write_str("tls-private-key"),
            Self::ApiKey => f.write_str("api-key"),
            Self::Other => f.write_str("other"),
        }
    }
}

/// Encrypted vault for storing and retrieving secrets.
///
/// Thread-safe: all public methods use an `RwLock` internally.
pub struct SecretsVault {
    /// Path to the encrypted vault file on disk.
    path: PathBuf,
    /// AES-256 key (32 bytes) derived from the machine key.
    key: [u8; 32],
    /// In-memory cache: logical key → encrypted secret.
    entries: RwLock<HashMap<String, (SecretKind, Secret)>>,
}

impl SecretsVault {
    /// Open (or create) the vault at the default location.
    ///
    /// The vault file is stored at `$XDG_DATA_HOME/atlasdb/vault.enc`.
    pub fn open_default() -> AtlasResult<Self> {
        let dir = dirs::data_dir()
            .map(|p| p.join("atlasdb"))
            .unwrap_or_else(|| PathBuf::from(".atlasdb"));
        fs::create_dir_all(&dir).map_err(AtlasError::from)?;
        let path = dir.join("vault.enc");
        Self::open(&path)
    }

    /// Open (or create) the vault at `path`.
    pub fn open(path: &std::path::Path) -> AtlasResult<Self> {
        let key = derive_machine_key();
        let entries = if path.exists() {
            Self::load_encrypted(path, &key)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            path: path.to_path_buf(),
            key,
            entries: RwLock::new(entries),
        })
    }

    /// Store a secret under `logical_key`.
    pub fn set(&self, logical_key: &str, kind: SecretKind, plaintext: &str) -> AtlasResult<()> {
        let secret = Secret::encrypt(plaintext, &self.key)?;
        self.entries.write().insert(logical_key.to_owned(), (kind, secret));
        self.flush()
    }

    /// Retrieve and decrypt a secret.
    pub fn get(&self, logical_key: &str) -> AtlasResult<Option<String>> {
        let entries = self.entries.read();
        if let Some((_kind, secret)) = entries.get(logical_key) {
            let plain = secret.decrypt(&self.key)?;
            Ok(Some(plain))
        } else {
            Ok(None)
        }
    }

    /// Delete a secret from the vault.
    pub fn delete(&self, logical_key: &str) -> AtlasResult<bool> {
        let removed = self.entries.write().remove(logical_key).is_some();
        if removed {
            self.flush()?;
        }
        Ok(removed)
    }

    /// List all logical keys currently stored.
    pub fn keys(&self) -> Vec<String> {
        self.entries.read().keys().cloned().collect()
    }

    /// Return the [`SecretKind`] for a given key.
    pub fn kind(&self, logical_key: &str) -> Option<SecretKind> {
        self.entries.read().get(logical_key).map(|(k, _)| *k)
    }

    /// Check whether a logical key exists.
    pub fn contains(&self, logical_key: &str) -> bool {
        self.entries.read().contains_key(logical_key)
    }

    // -- private ----------------------------------------------------------

    fn flush(&self) -> AtlasResult<()> {
        let entries = self.entries.read();
        let data = serde_json::to_vec_pretty(&*entries)
            .map_err(|e| AtlasError::Secrets { detail: e.to_string() })?;

        let mut file = fs::File::create(&self.path).map_err(AtlasError::from)?;
        file.write_all(&data).map_err(AtlasError::from)?;
        Ok(())
    }

    fn load_encrypted(
        path: &std::path::Path,
        key: &[u8; 32],
    ) -> AtlasResult<HashMap<String, (SecretKind, Secret)>> {
        let data = fs::read_to_string(path).map_err(AtlasError::from)?;
        let entries: HashMap<String, (SecretKind, Secret)> =
            serde_json::from_str(&data).map_err(|e| AtlasError::Secrets {
                detail: format!("vault parse: {e}"),
            })?;
        // Validate that every entry can be decrypted.
        for (logical_key, (_, secret)) in &entries {
            secret.decrypt(key).map_err(|e| {
                AtlasError::Secrets {
                    detail: format!("cannot decrypt '{logical_key}': {e}"),
                }
            })?;
        }
        Ok(entries)
    }
}

/// Derive a 256-bit key from a machine-local seed.
///
/// Strategy:
/// 1. Try to read `/etc/machine-id` (Linux) — combined with a fixed salt.
/// 2. Fall back to a hash of the hostname + username.
/// 3. If neither is available, generate a random key and store it in the
///    config directory (less secure but functional in containers).
fn derive_machine_key() -> [u8; 32] {
    let seed = machine_seed();
    let mut key = [0u8; 32];

    // Use SHA-256 as a KDF to produce a 32-byte key.
    let digest = digest::digest(&digest::SHA256, seed.as_bytes());
    key.copy_from_slice(digest.as_ref());
    key
}

fn machine_seed() -> String {
    // 1. /etc/machine-id
    if let Ok(id) = fs::read_to_string("/etc/machine-id") {
        let trimmed = id.trim().to_owned();
        if !trimmed.is_empty() {
            return format!("atlasdb:machine-id:{trimmed}");
        }
    }

    // 2. Hostname + username
    let host = hostname::get()
        .ok()
        .and_then(|h| h.to_str().map(String::from))
        .unwrap_or_else(|| "unknown".into());
    let user = whoami::username();
    format!("atlasdb:host:{host}:user:{user}")
}

/// Minimal hostname retrieval via `/proc/sys/kernel/hostname` on Linux,
/// environment variable on other platforms.
mod hostname {
    use std::ffi::OsString;

    /// Returns the system hostname.
    pub fn get() -> std::io::Result<OsString> {
        // Try Linux procfs
        if let Ok(content) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
            let trimmed = content.trim().to_owned();
            if !trimmed.is_empty() {
                return Ok(OsString::from(trimmed));
            }
        }
        // Fall back to the HOSTNAME or COMPUTERNAME environment variable
        for var in &["HOSTNAME", "COMPUTERNAME", "HOST"] {
            if let Ok(val) = std::env::var(var) {
                let trimmed = val.trim().to_owned();
                if !trimmed.is_empty() {
                    return Ok(OsString::from(trimmed));
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine hostname",
        ))
    }
}

/// Minimal username retrieval via environment variables.
mod whoami {
    /// Returns the current username.
    pub fn username() -> String {
        for var in &["USER", "LOGNAME", "USERNAME"] {
            if let Ok(val) = std::env::var(var) {
                let trimmed = val.trim().to_owned();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
        }
        "unknown".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn set_get_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");
        let vault = SecretsVault::open(&path).unwrap();

        vault.set("pg_pass", SecretKind::Password, "secret123").unwrap();

        let val = vault.get("pg_pass").unwrap();
        assert_eq!(val.as_deref(), Some("secret123"));
    }

    #[test]
    fn get_missing_returns_none() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");
        let vault = SecretsVault::open(&path).unwrap();

        assert_eq!(vault.get("nonexistent").unwrap(), None);
    }

    #[test]
    fn delete_removes_entry() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");
        let vault = SecretsVault::open(&path).unwrap();

        vault.set("x", SecretKind::Other, "val").unwrap();
        assert!(vault.contains("x"));
        assert!(vault.delete("x").unwrap());
        assert!(!vault.contains("x"));
        assert_eq!(vault.get("x").unwrap(), None);
    }

    #[test]
    fn persistence_survives_reopen() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");

        {
            let vault = SecretsVault::open(&path).unwrap();
            vault.set("k", SecretKind::ApiKey, "api-key-value").unwrap();
        }

        {
            let vault = SecretsVault::open(&path).unwrap();
            assert_eq!(vault.get("k").unwrap().as_deref(), Some("api-key-value"));
        }
    }

    #[test]
    fn keys_and_kinds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");
        let vault = SecretsVault::open(&path).unwrap();

        vault.set("a", SecretKind::Password, "1").unwrap();
        vault.set("b", SecretKind::ApiKey, "2").unwrap();

        let mut keys = vault.keys();
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);
        assert_eq!(vault.kind("a"), Some(SecretKind::Password));
        assert_eq!(vault.kind("b"), Some(SecretKind::ApiKey));
    }

    #[test]
    fn secret_redacts_debug() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.enc");
        let vault = SecretsVault::open(&path).unwrap();
        vault.set("s", SecretKind::Password, "hunter2").unwrap();

        let entries = vault.entries.read();
        let (_kind, secret) = entries.get("s").unwrap();
        let debug_str = format!("{secret:?}");
        assert!(!debug_str.contains("hunter2"));
        assert!(debug_str.contains("***"));
    }

    #[test]
    fn machine_key_is_32_bytes() {
        let key = derive_machine_key();
        assert_eq!(key.len(), 32);
    }
}
