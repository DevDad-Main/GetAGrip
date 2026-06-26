//! GetAGrip configuration management.
//!
//! Handles loading, merging, and watching configuration files
//! from multiple locations with layered overrides.

mod settings;

pub use settings::Settings;

use std::path::PathBuf;
use tg_core::error::CoreResult;
use tracing::{debug, info};

/// Configuration directory name.
const CONFIG_DIR: &str = "getagrip";

/// Main configuration file name.
const CONFIG_FILE: &str = "config.toml";

/// Get the default configuration directory.
#[must_use]
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIR)
}

/// Get the default data directory.
#[must_use]
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIR)
}

/// Get the default cache directory.
#[must_use]
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIR)
}

/// Ensure all required directories exist.
///
/// # Errors
/// Returns an error if directories cannot be created.
pub fn ensure_directories() -> CoreResult<()> {
    let dirs = [config_dir(), data_dir(), cache_dir()];
    for dir in &dirs {
        std::fs::create_dir_all(dir).map_err(|e| {
            tg_core::error::CoreError::config(format!(
                "Failed to create directory {}: {e}",
                dir.display()
            ))
        })?;
    }
    Ok(())
}

/// Load the application configuration from all sources.
///
/// # Errors
/// Returns an error if configuration files cannot be read or parsed.
pub fn load_config() -> CoreResult<Settings> {
    ensure_directories()?;

    let config_path = config_dir().join(CONFIG_FILE);
    info!(path = %config_path.display(), "Loading configuration");

    let settings = Settings::load(&config_path)?;
    debug!(?settings, "Configuration loaded");

    Ok(settings)
}
