//! Settings commands — read/write the JSON settings file.
//!
//! Phase 1 uses a simple in-memory store with no persistence path. The
//! frontend ships with defaults; `set_setting` is a no-op beyond logging.

use std::collections::HashMap;

use getagrip_settings::SettingsStore;

/// Tauri state handle for the settings store.
///
/// Wrapped in a newtype so we can attach it via `tauri::Builder::manage`.
pub struct SettingsState {
    pub inner: SettingsStore,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            inner: SettingsStore::new(),
        }
    }
}

/// Get all current settings as a JSON map.
///
/// We build this from `keys()` + `get()` since `values` is private.
#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, SettingsState>) -> Result<HashMap<String, serde_json::Value>, String> {
    let mut map = HashMap::new();
    for key in state.inner.keys() {
        if let Some(value) = state.inner.get::<serde_json::Value>(&key) {
            map.insert(key, value);
        }
    }
    Ok(map)
}

/// Set a single setting. Phase 1: logs and returns Ok without persisting.
#[tauri::command]
pub async fn set_setting(
    state: tauri::State<'_, SettingsState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    tracing::debug!("set_setting: {key} = {value}");
    state.inner.set(&key, &value).map_err(|e| format!("{e}"))?;
    Ok(())
}
