//! Plugin system for GetAGrip extensibility.

use std::collections::HashMap;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Plugin manifest loaded from a plugin directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub min_app_version: Option<String>,
    pub capabilities: Vec<PluginCapability>,
}

/// What a plugin can contribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCapability {
    /// A new database driver.
    DatabaseDriver,
    /// A custom panel in the UI.
    Panel,
    /// A command accessible via the command palette.
    Command,
    /// A custom theme.
    Theme,
    /// An AI provider.
    AiProvider,
    /// An export format.
    Exporter,
    /// A data transformer.
    DataTransformer,
    /// Editor diagnostics.
    Diagnostics,
    /// A visualization.
    Visualization,
}

impl std::fmt::Display for PluginCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseDriver => f.write_str("database-driver"),
            Self::Panel => f.write_str("panel"),
            Self::Command => f.write_str("command"),
            Self::Theme => f.write_str("theme"),
            Self::AiProvider => f.write_str("ai-provider"),
            Self::Exporter => f.write_str("exporter"),
            Self::DataTransformer => f.write_str("data-transformer"),
            Self::Diagnostics => f.write_str("diagnostics"),
            Self::Visualization => f.write_str("visualization"),
        }
    }
}

/// State of a loaded plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Discovered but not loaded.
    Discovered,
    /// Loaded and active.
    Active,
    /// Failed to load.
    Error,
    /// Explicitly disabled by the user.
    Disabled,
}

/// A loaded plugin instance.
#[derive(Debug, Clone)]
pub struct Plugin {
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub error: Option<String>,
}

/// The plugin registry — discovers, loads, and manages plugins.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a discovered plugin.
    pub fn register(&self, manifest: PluginManifest) {
        let name = manifest.name.clone();
        self.plugins.write().insert(name, Plugin {
            manifest,
            state: PluginState::Discovered,
            error: None,
        });
    }

    /// Enable a plugin by name.
    pub fn enable(&self, name: &str) -> bool {
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.get_mut(name) {
            plugin.state = PluginState::Active;
            true
        } else {
            false
        }
    }

    /// Disable a plugin by name.
    pub fn disable(&self, name: &str) -> bool {
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.get_mut(name) {
            plugin.state = PluginState::Disabled;
            true
        } else {
            false
        }
    }

    /// Mark a plugin as errored.
    pub fn mark_error(&self, name: &str, error: impl Into<String>) {
        let mut plugins = self.plugins.write();
        if let Some(plugin) = plugins.get_mut(name) {
            plugin.state = PluginState::Error;
            plugin.error = Some(error.into());
        }
    }

    /// List all registered plugins.
    pub fn all(&self) -> Vec<Plugin> {
        self.plugins.read().values().cloned().collect()
    }

    /// List active plugins.
    pub fn active(&self) -> Vec<Plugin> {
        self.plugins
            .read()
            .values()
            .filter(|p| p.state == PluginState::Active)
            .cloned()
            .collect()
    }

    /// Get a plugin by name.
    pub fn get(&self, name: &str) -> Option<Plugin> {
        self.plugins.read().get(name).cloned()
    }

    /// Count of registered plugins.
    pub fn count(&self) -> usize {
        self.plugins.read().len()
    }

    /// Count of active plugins.
    pub fn active_count(&self) -> usize {
        self.active().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> PluginManifest {
        PluginManifest {
            name: "my-plugin".into(),
            version: "1.0.0".into(),
            description: Some("A test plugin".into()),
            author: Some("Dev".into()),
            min_app_version: None,
            capabilities: vec![PluginCapability::Command, PluginCapability::Theme],
        }
    }

    #[test]
    fn register_and_enable() {
        let registry = PluginRegistry::new();
        registry.register(sample_manifest());
        assert_eq!(registry.count(), 1);
        assert_eq!(registry.active_count(), 0);

        assert!(registry.enable("my-plugin"));
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn disable_and_error() {
        let registry = PluginRegistry::new();
        registry.register(sample_manifest());
        registry.enable("my-plugin");
        registry.disable("my-plugin");
        assert_eq!(registry.active_count(), 0);

        registry.mark_error("my-plugin", "load failed");
        let p = registry.get("my-plugin").unwrap();
        assert_eq!(p.state, PluginState::Error);
    }
}
