//! Plugin system trait definitions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A loaded GetAGrip plugin.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Unique plugin identifier.
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Plugin version.
    fn version(&self) -> &str;

    /// Plugin description.
    fn description(&self) -> &str;

    /// Initialize the plugin. Called once when loaded.
    async fn initialize(&self) -> anyhow::Result<()>;

    /// Clean up. Called when the plugin is unloaded.
    async fn shutdown(&self) -> anyhow::Result<()>;

    /// Return commands this plugin exposes (for the command palette).
    fn commands(&self) -> Vec<PluginCommand> {
        Vec::new()
    }

    /// Return keybindings this plugin wants to register.
    fn keybindings(&self) -> Vec<PluginKeybinding> {
        Vec::new()
    }

    /// Return configuration schema for this plugin.
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }
}

/// A command exposed by a plugin, invokable from the command palette.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginCommand {
    /// Command ID (e.g., "plugin.sql-formatter.format").
    pub id: String,
    /// Display label.
    pub label: String,
    /// Description shown in the palette.
    pub description: Option<String>,
    /// Category for grouping.
    pub category: Option<String>,
    /// Default keybinding (optional).
    pub default_keybinding: Option<String>,
}

/// A keybinding registered by a plugin.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginKeybinding {
    /// The command this keybinding triggers.
    pub command: String,
    /// Key sequence (e.g., "ctrl+shift+f").
    pub keys: String,
    /// Mode this binding applies in (Normal, Insert, Visual, etc.).
    pub mode: String,
}

/// Plugin manifest as loaded from plugin.toml.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Author.
    pub author: Option<String>,
    /// Description.
    pub description: String,
    /// Minimum GetAGrip version required.
    pub min_tg_version: Option<String>,
    /// Dependencies on other plugins.
    pub dependencies: HashMap<String, String>,
    /// Plugin type: native, lua, wasm.
    #[serde(default = "default_plugin_type")]
    pub plugin_type: PluginType,
}

fn default_plugin_type() -> PluginType {
    PluginType::Native
}

/// Type of plugin runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// Native Rust dynamic library.
    Native,
    /// Lua script.
    Lua,
    /// WASM module.
    Wasm,
}
