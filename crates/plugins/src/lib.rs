//! GetAGrip plugin system.
//!
//! Supports loading plugins from Rust dynamic libraries, Lua scripts,
//! and WASM modules. Plugins can extend the application with new drivers,
//! themes, commands, AI providers, panels, and exporters.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use tg_core::traits::plugin::{Plugin, PluginManifest, PluginType};
use tracing::{debug, error, info, warn};

/// The plugin host manages loading, unloading, and lifecycle of all plugins.
pub struct PluginHost {
    plugins: DashMap<String, Arc<dyn Plugin>>,
    manifests: RwLock<HashMap<String, PluginManifest>>,
    plugin_dir: PathBuf,
}

impl PluginHost {
    /// Create a new plugin host.
    #[must_use]
    pub fn new() -> Self {
        let plugin_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("getagrip")
            .join("plugins");

        std::fs::create_dir_all(&plugin_dir).ok();

        Self {
            plugins: DashMap::new(),
            manifests: RwLock::new(HashMap::new()),
            plugin_dir,
        }
    }

    /// Set the plugin directory.
    pub fn set_plugin_dir(&mut self, path: PathBuf) {
        self.plugin_dir = path;
    }

    /// Load all plugins from the plugin directory.
    pub async fn load_all(&self) -> usize {
        let mut loaded = 0;

        info!(dir = %self.plugin_dir.display(), "Loading plugins");

        let entries = match std::fs::read_dir(&self.plugin_dir) {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read plugin directory: {e}");
                return 0;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    match self.load_plugin_from_dir(&path).await {
                        Ok(_) => loaded += 1,
                        Err(e) => warn!("Failed to load plugin from {}: {e}", path.display()),
                    }
                }
            }
        }

        info!("Loaded {loaded} plugins");
        loaded
    }

    /// Load a single plugin from a directory containing plugin.toml.
    async fn load_plugin_from_dir(
        &self,
        dir: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manifest_path = dir.join("plugin.toml");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_content)?;

        info!(id = %manifest.id, name = %manifest.name, version = %manifest.version, "Loading plugin");

        match manifest.plugin_type {
            PluginType::Native => {
                self.load_native_plugin(dir, &manifest).await?;
            }
            PluginType::Lua => {
                self.load_lua_plugin(dir, &manifest).await?;
            }
            PluginType::Wasm => {
                self.load_wasm_plugin(dir, &manifest).await?;
            }
        }

        self.manifests
            .write()
            .insert(manifest.id.clone(), manifest);

        Ok(())
    }

    async fn load_native_plugin(
        &self,
        dir: &std::path::Path,
        manifest: &PluginManifest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "linux")]
        let lib_name = format!("lib{}.so", manifest.id);
        #[cfg(target_os = "macos")]
        let lib_name = format!("lib{}.dylib", manifest.id);
        #[cfg(target_os = "windows")]
        let lib_name = format!("{}.dll", manifest.id);
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let lib_name = format!("lib{}.so", manifest.id);

        let lib_path = dir.join(&lib_name);
        if !lib_path.exists() {
            warn!("Native plugin library not found: {}", lib_path.display());
            return Ok(());
        }

        debug!("Loading native plugin: {}", lib_path.display());

        // Dynamic library loading via libloading
        // This is a stub — full implementation requires a C ABI for plugin communication
        let _ = lib_path;
        warn!("Native plugin loading not yet fully implemented for {}", manifest.id);

        Ok(())
    }

    async fn load_lua_plugin(
        &self,
        dir: &std::path::Path,
        manifest: &PluginManifest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let lua_path = dir.join("init.lua");
        if !lua_path.exists() {
            warn!("Lua plugin init.lua not found: {}", lua_path.display());
            return Ok(());
        }

        debug!("Loading Lua plugin: {}", lua_path.display());

        // Lua plugin loading via mlua
        // Stub for future implementation
        let _ = lua_path;
        warn!("Lua plugin loading not yet fully implemented for {}", manifest.id);

        Ok(())
    }

    async fn load_wasm_plugin(
        &self,
        dir: &std::path::Path,
        manifest: &PluginManifest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let wasm_path = dir.join("plugin.wasm");
        if !wasm_path.exists() {
            warn!("WASM plugin not found: {}", wasm_path.display());
            return Ok(());
        }

        debug!("Loading WASM plugin: {}", wasm_path.display());

        // WASM plugin loading via wasmtime
        // Stub for future implementation
        let _ = wasm_path;
        warn!("WASM plugin loading not yet fully implemented for {}", manifest.id);

        Ok(())
    }

    /// Register an already-instantiated plugin.
    pub fn register_plugin(&self, plugin: Arc<dyn Plugin>) {
        info!(id = %plugin.id(), name = %plugin.name(), "Registering plugin");
        self.plugins.insert(plugin.id().to_string(), plugin);
    }

    /// Unregister and shutdown a plugin.
    pub async fn unregister_plugin(&self, id: &str) -> Result<(), anyhow::Error> {
        if let Some((_, plugin)) = self.plugins.remove(id) {
            info!(%id, "Shutting down plugin");
            plugin.shutdown().await?;
            self.manifests.write().remove(id);
        }
        Ok(())
    }

    /// Get a plugin by ID.
    #[must_use]
    pub fn get_plugin(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(id).map(|p| p.clone())
    }

    /// List all loaded plugin IDs.
    #[must_use]
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.iter().map(|e| e.key().clone()).collect()
    }

    /// Get all registered commands from all plugins.
    #[must_use]
    pub fn all_commands(&self) -> Vec<tg_core::traits::plugin::PluginCommand> {
        self.plugins
            .iter()
            .flat_map(|entry| entry.value().commands())
            .collect()
    }

    /// Get all registered keybindings from all plugins.
    #[must_use]
    pub fn all_keybindings(&self) -> Vec<tg_core::traits::plugin::PluginKeybinding> {
        self.plugins
            .iter()
            .flat_map(|entry| entry.value().keybindings())
            .collect()
    }

    /// Shutdown all plugins.
    pub async fn shutdown_all(&self) {
        for entry in self.plugins.iter() {
            if let Err(e) = entry.value().shutdown().await {
                error!(id = %entry.key(), "Error shutting down plugin: {e}");
            }
        }
        self.plugins.clear();
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}
