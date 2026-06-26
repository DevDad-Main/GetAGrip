//! Driver registry — maintains a collection of loaded database drivers.

use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use tg_core::traits::driver::DatabaseDriver;
use tg_core::types::connection::DatabaseKind;
use tracing::{debug, info, warn};

/// Registry of all loaded database drivers.
pub struct DriverRegistry {
    drivers: DashMap<DatabaseKind, Arc<dyn DatabaseDriver>>,
    /// Driver search paths for dynamic loading.
    search_paths: RwLock<Vec<String>>,
}

impl DriverRegistry {
    /// Create an empty driver registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            drivers: DashMap::new(),
            search_paths: RwLock::new(Vec::new()),
        }
    }

    /// Register a driver. If a driver for this kind already exists, it is replaced.
    pub fn register(&self, driver: impl DatabaseDriver + 'static) {
        let kind = driver.kind();
        info!(?kind, name = driver.name(), "Registering database driver");
        self.drivers.insert(kind, Arc::new(driver));
    }

    /// Get a driver for the given database kind.
    #[must_use]
    pub fn get(&self, kind: DatabaseKind) -> Option<Arc<dyn DatabaseDriver>> {
        self.drivers.get(&kind).map(|d| d.clone())
    }

    /// Check if a driver is registered for the given kind.
    #[must_use]
    pub fn has(&self, kind: DatabaseKind) -> bool {
        self.drivers.contains_key(&kind)
    }

    /// Get all registered drivers.
    #[must_use]
    pub fn all(&self) -> Vec<Arc<dyn DatabaseDriver>> {
        self.drivers.iter().map(|entry| entry.clone()).collect()
    }

    /// Get all supported database kinds.
    #[must_use]
    pub fn supported_kinds(&self) -> Vec<DatabaseKind> {
        self.drivers.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Add a directory to search for dynamic driver plugins.
    pub fn add_search_path(&self, path: &str) {
        debug!("Adding driver search path: {path}");
        self.search_paths.write().push(path.to_string());
    }

    /// Remove a driver from the registry.
    pub fn unregister(&self, kind: DatabaseKind) {
        info!(?kind, "Unregistering database driver");
        self.drivers.remove(&kind);
    }

    /// Unregister all drivers.
    pub fn clear(&self) {
        self.drivers.clear();
    }

    /// Attempt to load dynamic drivers from search paths.
    pub fn load_dynamic_drivers(&self) -> usize {
        let paths = self.search_paths.read().clone();
        let mut loaded = 0;

        for path in &paths {
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if is_dynamic_library(&path) {
                            debug!("Found dynamic library: {}", path.display());
                            // Dynamic driver loading via libloading would go here
                            // Currently a stub for future implementation
                            warn!("Dynamic driver loading not yet implemented: {}", path.display());
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read driver search path {path}: {e}");
                }
            }
        }

        loaded
    }
}

impl Default for DriverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a file path looks like a dynamic library.
fn is_dynamic_library(path: &std::path::Path) -> bool {
    let name = path.to_string_lossy().to_lowercase();
    #[cfg(target_os = "linux")]
    {
        name.ends_with(".so")
    }
    #[cfg(target_os = "macos")]
    {
        name.ends_with(".dylib")
    }
    #[cfg(target_os = "windows")]
    {
        name.ends_with(".dll")
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false
    }
}
