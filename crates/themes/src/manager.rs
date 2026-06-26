//! Theme manager — loads, caches, and switches themes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use tg_core::traits::theme::Theme;
use tracing::{debug, info};

/// Manages the theme lifecycle.
pub struct ThemeManager {
    themes: RwLock<HashMap<String, Arc<Theme>>>,
    active: RwLock<Arc<Theme>>,
    user_theme_dir: PathBuf,
}

impl ThemeManager {
    /// Create a new theme manager, loading built-in themes.
    #[must_use]
    pub fn new() -> Self {
        let builtins = super::builtin_themes();
        let mut themes: HashMap<String, Arc<Theme>> = HashMap::new();

        // Default to Catppuccin Mocha
        let default = Arc::new(builtins.first().cloned().unwrap_or_else(|| {
            // Last resort fallback
            super::builtin::catppuccin_mocha()
        }));

        for theme in builtins {
            debug!("Loaded built-in theme: {}", theme.metadata.name);
            themes.insert(theme.metadata.name.clone(), Arc::new(theme));
        }

        let user_theme_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("getagrip")
            .join("themes");

        let manager = Self {
            themes: RwLock::new(themes),
            active: RwLock::new(default),
            user_theme_dir,
        };

        // Load user themes
        let _ = manager.load_user_themes();

        manager
    }

    /// Get the currently active theme.
    #[must_use]
    pub fn active(&self) -> Arc<Theme> {
        self.active.read().clone()
    }

    /// Switch to a different theme by name.
    pub fn set_theme(&self, name: &str) -> Result<(), tg_core::error::CoreError> {
        let themes = self.themes.read();
        let theme = themes.get(name).cloned().ok_or_else(|| {
            tg_core::error::CoreError::Theme { message: format!("Theme not found: {name}") }
        })?;
        drop(themes);

        info!("Switching to theme: {name}");
        *self.active.write() = theme;
        Ok(())
    }

    /// List all available theme names.
    #[must_use]
    pub fn list_themes(&self) -> Vec<String> {
        let mut names: Vec<String> = self.themes.read().keys().cloned().collect();
        names.sort();
        names
    }

    /// Check if a theme exists.
    #[must_use]
    pub fn has_theme(&self, name: &str) -> bool {
        self.themes.read().contains_key(name)
    }

    /// Register a new theme (e.g., from a plugin).
    pub fn register(&self, theme: Theme) {
        info!("Registering theme: {}", theme.metadata.name);
        self.themes
            .write()
            .insert(theme.metadata.name.clone(), Arc::new(theme));
    }

    /// Load user-installed themes from the themes directory.
    fn load_user_themes(&self) -> Result<(), tg_core::error::CoreError> {
        if !self.user_theme_dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.user_theme_dir).map_err(|e| {
            tg_core::error::CoreError::config(format!(
                "Failed to read theme directory {}: {e}",
                self.user_theme_dir.display()
            ))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml" || ext == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let theme_result = if path.extension().is_some_and(|ext| ext == "json") {
                        serde_json::from_str::<Theme>(&content).map_err(|e| e.to_string())
                    } else {
                        toml::from_str::<Theme>(&content).map_err(|e| e.to_string())
                    };

                    match theme_result {
                        Ok(t) => {
                            info!("Loaded user theme: {} from {}", t.metadata.name, path.display());
                            self.themes.write().insert(t.metadata.name.clone(), Arc::new(t));
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load theme {}: {e}", path.display());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}
