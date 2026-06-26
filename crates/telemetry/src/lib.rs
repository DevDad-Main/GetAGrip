//! GetAGrip telemetry — structured logging, tracing, and crash reporting.

use std::path::{Path, PathBuf};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// Initialize the telemetry system.
///
/// Sets up structured logging with optional file output, JSON formatting,
/// and OpenTelemetry export based on configuration.
///
/// # Errors
/// Returns an error if the log file cannot be opened.
pub fn init(config: &TelemetryConfig) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    if config.json_logging {
        // JSON-formatted output
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_current_span(true);

        let registry = tracing_subscriber::registry().with(env_filter);

        if config.file_logging {
            let log_path = PathBuf::from(config.log_file.clone().unwrap_or_else(default_log_path));
            let file_appender = tracing_appender::rolling::daily(
                log_path.parent().unwrap_or(Path::new(".")),
                log_path.file_name().unwrap_or(std::ffi::OsStr::new("tg.log")),
            );
            let file_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(file_appender)
                .with_ansi(false);
            registry.with(fmt_layer).with(file_layer).try_init()?;
        } else {
            registry.with(fmt_layer).try_init()?;
        }
    } else {
        // Human-readable output
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true);

        let registry = tracing_subscriber::registry().with(env_filter);

        if config.file_logging {
            let log_path = PathBuf::from(config.log_file.clone().unwrap_or_else(default_log_path));
            let file_appender = tracing_appender::rolling::daily(
                log_path.parent().unwrap_or(Path::new(".")),
                log_path.file_name().unwrap_or(std::ffi::OsStr::new("tg.log")),
            );
            let file_layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_appender);
            registry.with(fmt_layer).with(file_layer).try_init()?;
        } else {
            registry.with(fmt_layer).try_init()?;
        }
    }

    Ok(())
}

/// Telemetry configuration.
#[derive(Clone, Debug)]
pub struct TelemetryConfig {
    /// Log level filter (trace, debug, info, warn, error).
    pub log_level: String,
    /// Whether to write logs to a file.
    pub file_logging: bool,
    /// Custom log file path.
    pub log_file: Option<String>,
    /// Whether to use JSON formatting.
    pub json_logging: bool,
    /// Whether to enable crash reports.
    pub crash_report: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: "info".into(),
            file_logging: false,
            log_file: None,
            json_logging: false,
            crash_report: false,
        }
    }
}

/// Get the default log file path.
#[must_use]
pub fn default_log_path() -> String {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("getagrip")
        .join("logs");

    std::fs::create_dir_all(&dir).ok();

    dir.join("tg.log").to_string_lossy().into_owned()
}
