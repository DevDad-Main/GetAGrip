//! Settings commands — read/write the JSON settings file.

use std::collections::HashMap;
use std::path::PathBuf;

use getagrip_settings::SettingsStore;

/// Tauri state handle for the settings store.
pub struct SettingsState {
    pub inner: SettingsStore,
    pub path: Option<PathBuf>,
}

/// Default settings written to disk on first launch so users can see every option.
fn default_settings() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("fontSize", serde_json::Value::Number(13.into())),
        ("fontFamily", serde_json::Value::String("JetBrains Mono, Fira Code, Menlo, Consolas, monospace".into())),
        ("wordWrap", serde_json::Value::Bool(true)),
        ("minimap", serde_json::Value::Bool(true)),
        ("tabSize", serde_json::Value::Number(4.into())),
        ("lineNumbers", serde_json::Value::Bool(true)),
        ("autoSave", serde_json::Value::Bool(true)),
        ("autoSaveDelay", serde_json::Value::Number(30.into())),
        ("formatOnSave", serde_json::Value::Bool(false)),
        ("theme", serde_json::Value::String("darcula".into())),
        ("sidebarWidth", serde_json::Value::Number(260.into())),
    ]
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            inner: SettingsStore::new(),
            path: None,
        }
    }

    /// Load settings from disk, writing defaults if the file doesn't exist.
    pub fn load(path: PathBuf) -> Self {
        let exists = path.exists();
        let inner = SettingsStore::load(path.clone()).unwrap_or_default();

        if !exists {
            // First launch — seed the file with defaults
            for (key, value) in default_settings() {
                let _ = inner.set(key, &value);
            }
            let _ = inner.save();
        }

        Self {
            inner,
            path: Some(path),
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
