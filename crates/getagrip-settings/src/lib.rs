//! Settings management and persistence for GetAGrip.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use getagrip_core::{AtlasError, AtlasResult};

/// A key-value settings store with persistence.
#[derive(Debug, Default)]
pub struct SettingsStore {
    /// In-memory settings.
    values: RwLock<HashMap<String, serde_json::Value>>,
    /// Path to the settings file on disk.
    path: Option<PathBuf>,
}

impl SettingsStore {
    /// Create an empty in-memory settings store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load settings from a JSON file.
    pub fn load(path: PathBuf) -> AtlasResult<Self> {
        let values = if path.exists() {
            let data = fs::read_to_string(&path).map_err(AtlasError::from)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Ok(Self {
            values: RwLock::new(values),
            path: Some(path),
        })
    }

    /// Get a setting value by key.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.values
            .read()
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get a setting with a default fallback.
    pub fn get_or<T: for<'de> Deserialize<'de>>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// Set a setting value.
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> AtlasResult<()> {
        let json = serde_json::to_value(value).map_err(|e| AtlasError::Other {
            detail: e.to_string(),
            cause: None,
        })?;
        self.values.write().insert(key.to_owned(), json);
        Ok(())
    }

    /// Remove a setting.
    pub fn remove(&self, key: &str) -> bool {
        self.values.write().remove(key).is_some()
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.values.read().contains_key(key)
    }

    /// Return all setting keys.
    pub fn keys(&self) -> Vec<String> {
        self.values.read().keys().cloned().collect()
    }

    /// Persist settings to disk.
    pub fn save(&self) -> AtlasResult<()> {
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(AtlasError::from)?;
            }
            let data = serde_json::to_string_pretty(&*self.values.read())
                .map_err(|e| AtlasError::Other { detail: e.to_string(), cause: None })?;
            fs::write(path, data).map_err(AtlasError::from)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_roundtrip() {
        let store = SettingsStore::new();
        store.set("font_size", &14.0_f32).unwrap();
        let val: f32 = store.get("font_size").unwrap();
        assert!((val - 14.0).abs() < 0.01);
    }

    #[test]
    fn get_missing_returns_none() {
        let store = SettingsStore::new();
        assert_eq!(store.get::<String>("missing"), None);
    }

    #[test]
    fn get_or_fallback() {
        let store = SettingsStore::new();
        assert_eq!(store.get_or("theme", "dark".to_string()), "dark");
    }

    #[test]
    fn remove_and_contains() {
        let store = SettingsStore::new();
        store.set("key", &"val").unwrap();
        assert!(store.contains("key"));
        assert!(store.remove("key"));
        assert!(!store.contains("key"));
    }
}
