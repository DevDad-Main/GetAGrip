//! Settings commands — read/write the JSON settings file.

use std::collections::HashMap;
use std::path::PathBuf;

use getagrip_settings::SettingsStore;

/// Tauri state handle for the settings store.
pub struct SettingsState {
    pub inner: SettingsStore,
    pub path: Option<PathBuf>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            inner: SettingsStore::new(),
            path: None,
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let path2 = path.clone();
        Self {
            inner: SettingsStore::load(path).unwrap_or_default(),
            path: Some(path2),
        }
    }
}

/// Get all current settings as a JSON map.
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

/// Set a single setting and persist to disk.
#[tauri::command]
pub async fn set_setting(
    state: tauri::State<'_, SettingsState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    tracing::debug!("set_setting: {key} = {value}");
    state.inner.set(&key, &value).map_err(|e| format!("{e}"))?;
    state.inner.save().map_err(|e| format!("{e}"))?;
    Ok(())
}

/// Get the path to the settings JSON file on disk.
#[tauri::command]
pub async fn get_settings_path(state: tauri::State<'_, SettingsState>) -> Result<String, String> {
    state
        .path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "settings path not set".to_string())
}
