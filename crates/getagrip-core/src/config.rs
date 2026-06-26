//! Application and workspace configuration.
//!
//! GetAGrip Studio uses a layered config system built on [`figment`]:
//!
//! 1. Built-in defaults (shipped with the binary).
//! 2. Global user config (`$XDG_CONFIG_HOME/atlasdb/config.toml`).
//! 3. Workspace config (`.atlasdb/config.toml` in the project root).
//! 4. Environment variables (`ATLAS_` prefix).
//!
//! Lower layers override higher layers. All config is serialisable so that
//! the settings UI can round-trip changes.

use figment::{
    Figment,
    providers::{Env, Format, Toml, Yaml, Serialized},
};
use serde::{Deserialize, Serialize};

/// Root configuration for GetAGrip Studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// General application settings.
    #[serde(default)]
    pub general: GeneralConfig,

    /// Editor settings.
    #[serde(default)]
    pub editor: EditorConfig,

    /// Theme and appearance.
    #[serde(default)]
    pub appearance: AppearanceConfig,

    /// Connection manager settings.
    #[serde(default)]
    pub connections: ConnectionsConfig,

    /// Telemetry settings.
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            editor: EditorConfig::default(),
            appearance: AppearanceConfig::default(),
            connections: ConnectionsConfig::default(),
            telemetry: TelemetryConfig::default(),
        }
    }
}

impl WorkspaceConfig {
    /// Load configuration from all layers.
    ///
    /// Layers (lowest to highest priority):
    /// 1. Built-in defaults via `Default`.
    /// 2. `$XDG_CONFIG_HOME/atlasdb/config.toml`
    /// 3. `.atlasdb/config.toml` (workspace root)
    /// 4. Environment variables prefixed with `ATLAS_`
    pub fn load(workspace_root: Option<&str>) -> crate::AtlasResult<Self> {
        let mut figment = Figment::from(Serialized::defaults(Self::default()));

        // Global config
        if let Some(global_path) = Self::global_config_path() {
            figment = figment.merge(Toml::file(&global_path));
            // Also try YAML
            let yaml_path = global_path.with_extension("yaml");
            if yaml_path.exists() {
                figment = figment.merge(Yaml::file(&yaml_path));
            }
        }

        // Workspace config
        if let Some(root) = workspace_root {
            let workspace_toml = format!("{root}/.atlasdb/config.toml");
            figment = figment.merge(Toml::file(workspace_toml));
        }

        // Environment variables (ATLAS_GENERAL__STARTUP_WORKSPACE=...)
        figment = figment.merge(
            Env::prefixed("ATLAS_").split("__")
        );

        Ok(figment.extract()?)
    }

    /// Path to the global user config file.
    pub fn global_config_dir() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("atlasdb"))
    }

    /// Full path to the global config TOML file.
    pub fn global_config_path() -> Option<std::path::PathBuf> {
        Self::global_config_dir().map(|p| p.join("config.toml"))
    }

    /// Ensure the global config directory exists.
    pub fn ensure_config_dir() -> std::io::Result<std::path::PathBuf> {
        let dir = dirs::config_dir()
            .map(|p| p.join("atlasdb"))
            .unwrap_or_else(|| {
                std::path::PathBuf::from(".atlasdb")
            });
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}

// ---- Sub-config sections ------------------------------------------------

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Whether to restore the previous session on startup.
    #[serde(default = "default_true")]
    pub restore_session: bool,

    /// Default workspace path.
    pub default_workspace: Option<String>,

    /// Maximum number of recent connections to show.
    #[serde(default = "default_recent_limit")]
    pub recent_connections_limit: usize,

    /// Auto-save interval in seconds (0 = disabled).
    #[serde(default)]
    pub autosave_interval_secs: u64,

    /// Language for the UI (ISO 639-1 code).
    #[serde(default = "default_lang")]
    pub language: String,
}

fn default_true() -> bool {
    true
}

fn default_recent_limit() -> usize {
    20
}

fn default_lang() -> String {
    "en".into()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            restore_session: true,
            default_workspace: None,
            recent_connections_limit: 20,
            autosave_interval_secs: 300,
            language: "en".into(),
        }
    }
}

/// Editor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Font family for the editor.
    #[serde(default = "default_editor_font")]
    pub font_family: String,

    /// Font size in points.
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// Tab width in spaces.
    #[serde(default = "default_tab_width")]
    pub tab_width: u8,

    /// Whether to show line numbers.
    #[serde(default = "default_true")]
    pub line_numbers: bool,

    /// Whether to show the minimap.
    #[serde(default)]
    pub minimap: bool,

    /// Whether to show indent guides.
    #[serde(default = "default_true")]
    pub indent_guides: bool,

    /// Whether to highlight bracket pairs.
    #[serde(default = "default_true")]
    pub bracket_matching: bool,

    /// Auto-format on save.
    #[serde(default)]
    pub format_on_save: bool,

    /// Maximum number of editor tabs.
    #[serde(default = "default_tab_limit")]
    pub max_tabs: usize,
}

fn default_editor_font() -> String {
    "JetBrains Mono".into()
}

fn default_font_size() -> f32 {
    14.0
}

fn default_tab_width() -> u8 {
    4
}

fn default_tab_limit() -> usize {
    50
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".into(),
            font_size: 14.0,
            tab_width: 4,
            line_numbers: true,
            minimap: false,
            indent_guides: true,
            bracket_matching: true,
            format_on_save: false,
            max_tabs: 50,
        }
    }
}

/// Appearance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// Theme name (e.g. "catppuccin-mocha").
    #[serde(default = "default_theme")]
    pub theme: String,

    /// UI scale factor.
    #[serde(default = "default_scale")]
    pub scale: f32,

    /// Reduce UI motion.
    #[serde(default)]
    pub reduced_motion: bool,

    /// High-contrast mode.
    #[serde(default)]
    pub high_contrast: bool,

    /// Show the status bar.
    #[serde(default = "default_true")]
    pub status_bar: bool,

    /// Show the toolbar.
    #[serde(default = "default_true")]
    pub toolbar: bool,
}

fn default_theme() -> String {
    "catppuccin-mocha".into()
}

fn default_scale() -> f32 {
    1.0
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "catppuccin-mocha".into(),
            scale: 1.0,
            reduced_motion: false,
            high_contrast: false,
            status_bar: true,
            toolbar: true,
        }
    }
}

/// Connection manager settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsConfig {
    /// Connection timeout in seconds.
    #[serde(default = "default_conn_timeout")]
    pub timeout_secs: u64,

    /// Maximum pool size (per connection).
    #[serde(default = "default_pool_size")]
    pub max_pool_size: u32,

    /// Whether to auto-reconnect dropped connections.
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,

    /// Delay between reconnection attempts in seconds.
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u64,

    /// Maximum reconnection attempts before giving up.
    #[serde(default = "default_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
}

fn default_conn_timeout() -> u64 {
    15
}

fn default_pool_size() -> u32 {
    5
}

fn default_reconnect_delay() -> u64 {
    3
}

fn default_reconnect_attempts() -> u32 {
    5
}

impl Default for ConnectionsConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 15,
            max_pool_size: 5,
            auto_reconnect: true,
            reconnect_delay_secs: 3,
            max_reconnect_attempts: 5,
        }
    }
}

/// Telemetry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled at all.
    #[serde(default)]
    pub enabled: bool,

    /// Log level filter (e.g. "info,getagrip_core=debug").
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Path to the log file (relative to config dir if not absolute).
    pub log_file: Option<String>,

    /// Whether to emit structured JSON logs.
    #[serde(default)]
    pub json_logs: bool,
}

fn default_log_level() -> String {
    "info".into()
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_level: "info".into(),
            log_file: None,
            json_logs: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let cfg = WorkspaceConfig::default();
        assert_eq!(cfg.general.language, "en");
        assert_eq!(cfg.editor.font_size, 14.0);
        assert_eq!(cfg.editor.tab_width, 4);
        assert!(cfg.editor.line_numbers);
        assert_eq!(cfg.appearance.theme, "catppuccin-mocha");
        assert_eq!(cfg.connections.timeout_secs, 15);
        assert!(!cfg.telemetry.enabled);
    }

    #[test]
    fn config_is_serializable() {
        let cfg = WorkspaceConfig::default();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let back: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.general.language, back.general.language);
    }
}
