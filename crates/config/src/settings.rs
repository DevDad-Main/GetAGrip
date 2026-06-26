//! Application settings — the central configuration struct.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level application settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    /// General application settings.
    #[serde(default)]
    pub general: GeneralSettings,
    /// Editor settings.
    #[serde(default)]
    pub editor: EditorSettings,
    /// Keybinding settings.
    #[serde(default)]
    pub keybindings: KeybindingSettings,
    /// Theme settings.
    #[serde(default)]
    pub theme: ThemeSettings,
    /// Connection defaults.
    #[serde(default)]
    pub connections: ConnectionDefaults,
    /// AI provider configuration.
    #[serde(default)]
    pub ai: AiSettings,
    /// Telemetry and logging.
    #[serde(default)]
    pub telemetry: TelemetrySettings,
    /// Plugin settings.
    #[serde(default)]
    pub plugins: PluginSettings,
    /// Data export defaults.
    #[serde(default)]
    pub export: ExportSettings,
    /// Workspace settings.
    #[serde(default)]
    pub workspace: WorkspaceSettings,
}

impl Settings {
    /// Load settings from a file, creating defaults if the file doesn't exist.
    ///
    /// # Errors
    /// Returns an error if the file exists but cannot be parsed.
    pub fn load(path: &Path) -> Result<Self, tg_core::error::CoreError> {
        if path.exists() {
            let content = std::fs::read_to_string(path).map_err(|e| {
                tg_core::error::CoreError::config(format!(
                    "Failed to read config file {}: {e}",
                    path.display()
                ))
            })?;

            toml::from_str(&content).map_err(|e| {
                tg_core::error::CoreError::config(format!(
                    "Failed to parse config file {}: {e}",
                    path.display()
                ))
            })
        } else {
            Ok(Self::default())
        }
    }

    /// Save settings to a file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), tg_core::error::CoreError> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            tg_core::error::CoreError::config(format!("Failed to serialize config: {e}"))
        })?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                tg_core::error::CoreError::config(format!(
                    "Failed to create config directory: {e}"
                ))
            })?;
        }

        std::fs::write(path, content).map_err(|e| {
            tg_core::error::CoreError::config(format!(
                "Failed to write config file {}: {e}",
                path.display()
            ))
        })
    }
}

/// General application settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneralSettings {
    /// Startup workspace to load.
    #[serde(default)]
    pub startup_workspace: Option<String>,
    /// Confirm before quitting.
    #[serde(default)]
    pub confirm_quit: bool,
    /// Auto-update check interval in hours (0 = disabled).
    #[serde(default)]
    pub update_check_hours: u64,
    /// Maximum number of recent files/connections to remember.
    #[serde(default = "default_max_recent")]
    pub max_recent: usize,
    /// Language for the UI (ISO 639-1 code).
    #[serde(default)]
    pub language: String,
    /// Whether to send anonymous usage statistics.
    #[serde(default = "default_true")]
    pub telemetry_enabled: bool,
    /// Whether to check for updates on startup.
    #[serde(default = "default_true")]
    pub check_updates: bool,
}

fn default_max_recent() -> usize {
    50
}

fn default_true() -> bool {
    true
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            startup_workspace: None,
            confirm_quit: false,
            update_check_hours: 24,
            max_recent: 50,
            language: "en".into(),
            telemetry_enabled: true,
            check_updates: true,
        }
    }
}

/// Editor configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorSettings {
    /// Font family (for GUI mode).
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Font size.
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    /// Tab width in spaces.
    #[serde(default = "default_tab_size")]
    pub tab_size: u8,
    /// Whether to use spaces instead of tabs.
    #[serde(default = "default_true")]
    pub insert_spaces: bool,
    /// Whether to show line numbers.
    #[serde(default = "default_true")]
    pub line_numbers: bool,
    /// Whether to show the minimap.
    #[serde(default)]
    pub minimap: bool,
    /// Whether to highlight the current line.
    #[serde(default = "default_true")]
    pub highlight_current_line: bool,
    /// Whether to auto-close brackets.
    #[serde(default = "default_true")]
    pub auto_close_brackets: bool,
    /// Whether to format on save.
    #[serde(default)]
    pub format_on_save: bool,
    /// Whether to show whitespace characters.
    #[serde(default)]
    pub show_whitespace: bool,
    /// Word wrap mode (off, word, column).
    #[serde(default)]
    pub word_wrap: String,
    /// Maximum undo history depth.
    #[serde(default = "default_undo_depth")]
    pub undo_depth: usize,
    /// Whether to enable bracket pair colorization.
    #[serde(default = "default_true")]
    pub rainbow_brackets: bool,
    /// Whether to show breadcrumbs above the editor.
    #[serde(default = "default_true")]
    pub breadcrumbs: bool,
    /// Autosave interval in seconds (0 = disabled).
    #[serde(default)]
    pub autosave_interval_secs: u64,
    /// Maximum number of open tabs.
    #[serde(default = "default_max_tabs")]
    pub max_tabs: usize,
    /// Whether to enable sticky tab headers.
    #[serde(default = "default_true")]
    pub sticky_headers: bool,
}

fn default_font_family() -> String {
    "monospace".into()
}
fn default_font_size() -> u16 {
    14
}
fn default_tab_size() -> u8 {
    2
}
fn default_undo_depth() -> usize {
    1000
}
fn default_max_tabs() -> usize {
    50
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            tab_size: default_tab_size(),
            insert_spaces: true,
            line_numbers: true,
            minimap: false,
            highlight_current_line: true,
            auto_close_brackets: true,
            format_on_save: false,
            show_whitespace: false,
            word_wrap: String::new(),
            undo_depth: default_undo_depth(),
            rainbow_brackets: true,
            breadcrumbs: true,
            autosave_interval_secs: 0,
            max_tabs: default_max_tabs(),
            sticky_headers: true,
        }
    }
}

/// Keybinding mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KeybindingMode {
    /// Standard keybindings (like VS Code).
    #[default]
    Standard,
    /// Vim-style modal editing.
    Vim,
    /// Emacs-style keybindings.
    Emacs,
    /// Fully custom keybindings.
    Custom,
}

/// Keybinding settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeybindingSettings {
    /// The keybinding mode.
    #[serde(default)]
    pub mode: KeybindingMode,
    /// Leader key for vim mode.
    #[serde(default)]
    pub leader_key: Option<String>,
    /// Custom keybinding overrides (command -> key sequence).
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, String>,
    /// Whether to show keybinding hints.
    #[serde(default = "default_true")]
    pub show_hints: bool,
}

impl Default for KeybindingSettings {
    fn default() -> Self {
        Self {
            mode: KeybindingMode::Standard,
            leader_key: None,
            overrides: std::collections::HashMap::new(),
            show_hints: true,
        }
    }
}

/// Theme settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeSettings {
    /// Active theme name.
    #[serde(default = "default_theme")]
    pub active: String,
    /// Font size override for the theme.
    pub font_size: Option<u16>,
    /// Line height multiplier.
    #[serde(default = "default_line_height")]
    pub line_height: f32,
}

fn default_theme() -> String {
    "catppuccin-mocha".into()
}
fn default_line_height() -> f32 {
    1.0
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            active: default_theme(),
            font_size: None,
            line_height: default_line_height(),
        }
    }
}

/// Default connection parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionDefaults {
    /// Default connection timeout in seconds.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,
    /// Default query timeout in seconds (0 = no timeout).
    #[serde(default)]
    pub query_timeout_secs: u64,
    /// Default SSL mode.
    #[serde(default)]
    pub ssl_mode: String,
    /// Whether to automatically reconnect dropped connections.
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts.
    #[serde(default = "default_max_reconnect")]
    pub max_reconnect_attempts: u32,
    /// Whether to verify TLS certificates.
    #[serde(default = "default_true")]
    pub verify_certs: bool,
    /// Connection pool defaults.
    #[serde(default)]
    pub pool: ConnectionPoolDefaults,
}

fn default_connect_timeout() -> u64 {
    30
}
fn default_max_reconnect() -> u32 {
    5
}

impl Default for ConnectionDefaults {
    fn default() -> Self {
        Self {
            connect_timeout_secs: default_connect_timeout(),
            query_timeout_secs: 0,
            ssl_mode: "disable".into(),
            auto_reconnect: true,
            max_reconnect_attempts: default_max_reconnect(),
            verify_certs: true,
            pool: ConnectionPoolDefaults::default(),
        }
    }
}

/// Connection pool configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionPoolDefaults {
    /// Minimum connections in pool.
    #[serde(default = "default_min_conns")]
    pub min_connections: u32,
    /// Maximum connections in pool.
    #[serde(default = "default_max_conns")]
    pub max_connections: u32,
    /// Connection idle timeout in seconds.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
    /// Maximum connection lifetime in seconds.
    #[serde(default = "default_max_lifetime")]
    pub max_lifetime_secs: u64,
}

fn default_min_conns() -> u32 {
    0
}
fn default_max_conns() -> u32 {
    10
}
fn default_idle_timeout() -> u64 {
    300
}
fn default_max_lifetime() -> u64 {
    1800
}

impl Default for ConnectionPoolDefaults {
    fn default() -> Self {
        Self {
            min_connections: default_min_conns(),
            max_connections: default_max_conns(),
            idle_timeout_secs: default_idle_timeout(),
            max_lifetime_secs: default_max_lifetime(),
        }
    }
}

/// AI provider settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiSettings {
    /// Whether AI features are enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Provider type.
    #[serde(default)]
    pub provider: String,
    /// API endpoint.
    #[serde(default)]
    pub endpoint: String,
    /// API key.
    pub api_key: Option<String>,
    /// Model name.
    #[serde(default)]
    pub model: String,
    /// Maximum tokens in responses.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.1
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "ollama".into(),
            endpoint: "http://localhost:11434/v1".into(),
            api_key: None,
            model: "codellama".into(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

/// Telemetry and logging settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetrySettings {
    /// Log level (trace, debug, info, warn, error).
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Whether to write logs to a file.
    #[serde(default = "default_true")]
    pub file_logging: bool,
    /// Log file path.
    pub log_file: Option<String>,
    /// Whether to output JSON-formatted logs.
    #[serde(default)]
    pub json_logging: bool,
    /// Whether to enable OpenTelemetry export.
    #[serde(default)]
    pub opentelemetry: bool,
    /// Whether to capture crash reports.
    #[serde(default = "default_true")]
    pub crash_report: bool,
}

fn default_log_level() -> String {
    "info".into()
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            file_logging: true,
            log_file: None,
            json_logging: false,
            opentelemetry: false,
            crash_report: true,
        }
    }
}

/// Plugin system settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginSettings {
    /// Whether plugins are enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Plugin search paths.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Plugins to disable by ID.
    #[serde(default)]
    pub disabled: Vec<String>,
    /// Whether to allow network access for plugins.
    #[serde(default)]
    pub allow_network: bool,
    /// Maximum plugin memory in MB.
    #[serde(default = "default_plugin_memory")]
    pub max_memory_mb: u64,
}

fn default_plugin_memory() -> u64 {
    512
}

impl Default for PluginSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            paths: Vec::new(),
            disabled: Vec::new(),
            allow_network: false,
            max_memory_mb: default_plugin_memory(),
        }
    }
}

/// Data export default settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportSettings {
    /// Default export format.
    #[serde(default)]
    pub default_format: String,
    /// Default delimiter for CSV.
    #[serde(default)]
    pub csv_delimiter: String,
    /// Include headers by default.
    #[serde(default = "default_true")]
    pub include_headers: bool,
    /// Null representation string.
    #[serde(default = "default_null_str")]
    pub null_string: String,
}

fn default_null_str() -> String {
    "NULL".into()
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            default_format: "csv".into(),
            csv_delimiter: ",".into(),
            include_headers: true,
            null_string: default_null_str(),
        }
    }
}

/// Workspace settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// Whether to restore the previous workspace on startup.
    #[serde(default = "default_true")]
    pub restore_on_startup: bool,
    /// Whether to save workspace state on exit.
    #[serde(default = "default_true")]
    pub save_on_exit: bool,
    /// Workspace data directory.
    pub data_dir: Option<String>,
    /// Whether to auto-save workspaces.
    #[serde(default = "default_true")]
    pub auto_save: bool,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            restore_on_startup: true,
            save_on_exit: true,
            data_dir: None,
            auto_save: true,
        }
    }
}
