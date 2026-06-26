//! Structured logging, tracing, and diagnostics for GetAGrip.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use tracing::Level;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use atlas_core::AtlasResult;

static INIT: OnceLock<()> = OnceLock::new();

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable, colourised output.
    Pretty,
    /// JSON lines (suitable for ingestion by Loki, ELK, etc.).
    Json,
}

/// Telemetry initialisation options.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Minimum log level.
    pub level: Level,
    /// Output format.
    pub format: LogFormat,
    /// Optional file path for persistent logs.
    pub log_file: Option<PathBuf>,
    /// Whether to emit spans for function entry/exit (expensive).
    pub span_events: bool,
    /// Custom env-filter directive (overrides `RUST_LOG` when set).
    pub filter_directive: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            format: LogFormat::Pretty,
            log_file: None,
            span_events: false,
            filter_directive: None,
        }
    }
}

/// Initialise the telemetry subsystem.
///
/// Idempotent: subsequent calls are no-ops.
pub fn init(config: TelemetryConfig) -> AtlasResult<()> {
    // Use get_or_init which is stable on 1.86+.
    let mut result = Ok(());

    INIT.get_or_init(|| {
        result = try_init(config);
    });

    result
}

fn try_init(config: TelemetryConfig) -> AtlasResult<()> {
    let filter = build_filter(&config);

    match config.format {
        LogFormat::Pretty => {
            let layer = fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(true)
                .with_level(true);

            let layer = if config.span_events {
                layer.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            } else {
                layer
            };

            if let Some(log_path) = config.log_file.as_ref() {
                ensure_log_dir(log_path)?;
                let file_appender = tracing_appender::rolling::never(
                    log_path.parent().unwrap_or(std::path::Path::new(".")),
                    log_path.file_name().and_then(|n| n.to_str()).unwrap_or("getagrip.log"),
                );
                let file_layer = fmt::layer()
                    .with_writer(file_appender)
                    .with_ansi(false)
                    .json();

                tracing_subscriber::registry()
                    .with(filter)
                    .with(layer)
                    .with(file_layer)
                    .try_init()
                    .map_err(|e| atlas_core::err_msg(format!("tracing init: {e}")))?;
            } else {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(layer)
                    .try_init()
                    .map_err(|e| atlas_core::err_msg(format!("tracing init: {e}")))?;
            }
        }
        LogFormat::Json => {
            let layer = fmt::layer()
                .json()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(true)
                .with_current_span(false)
                .with_span_list(false);

            let layer = if config.span_events {
                layer.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            } else {
                layer
            };

            if let Some(log_path) = config.log_file.as_ref() {
                ensure_log_dir(log_path)?;
                let file_appender = tracing_appender::rolling::never(
                    log_path.parent().unwrap_or(std::path::Path::new(".")),
                    log_path.file_name().and_then(|n| n.to_str()).unwrap_or("getagrip.log"),
                );
                let file_layer = fmt::layer()
                    .with_writer(file_appender)
                    .with_ansi(false)
                    .json();

                tracing_subscriber::registry()
                    .with(filter)
                    .with(layer)
                    .with(file_layer)
                    .try_init()
                    .map_err(|e| atlas_core::err_msg(format!("tracing init: {e}")))?;
            } else {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(layer)
                    .try_init()
                    .map_err(|e| atlas_core::err_msg(format!("tracing init: {e}")))?;
            }
        }
    }

    install_panic_hook();

    tracing::info!(
        "telemetry initialised: level={}, format={:?}",
        config.level,
        config.format,
    );

    Ok(())
}

fn ensure_log_dir(log_path: &PathBuf) -> AtlasResult<()> {
    if let Some(parent) = log_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                atlas_core::AtlasError::Io {
                    detail: format!("cannot create log dir: {e}"),
                    cause: e,
                }
            })?;
        }
    }
    Ok(())
}

fn build_filter(config: &TelemetryConfig) -> EnvFilter {
    if let Some(directive) = &config.filter_directive {
        if let Ok(filter) = EnvFilter::try_new(directive.clone()) {
            return filter;
        }
    }

    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = match config.level {
            Level::TRACE => "trace",
            Level::DEBUG => "debug",
            Level::INFO => "info",
            Level::WARN => "warn",
            Level::ERROR => "error",
        };
        EnvFilter::new(format!("{level},slint=warn,winit=warn,tokio=warn,hyper=warn"))
    })
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{l}"))
            .unwrap_or_else(|| "<unknown>".into());

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".into()
        };

        tracing::error!(
            location = %location,
            payload = %payload,
            "CRASH: GetAGrip panicked"
        );

        default_hook(info);
    }));
}

/// Convenience: initialise with sensible defaults.
pub fn init_default() -> AtlasResult<()> {
    init(TelemetryConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let cfg = TelemetryConfig::default();
        assert_eq!(cfg.level, Level::INFO);
        assert_eq!(cfg.format, LogFormat::Pretty);
        assert!(cfg.log_file.is_none());
        assert!(!cfg.span_events);
    }

    #[test]
    fn init_is_idempotent() {
        let r1 = init(TelemetryConfig {
            level: Level::WARN,
            ..Default::default()
        });
        assert!(r1.is_ok());

        let r2 = init(TelemetryConfig::default());
        assert!(r2.is_ok());
    }
}
