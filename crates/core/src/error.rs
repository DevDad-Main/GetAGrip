//! Unified error types for AtlasDB Studio.
//!
//! Every crate in the workspace returns [`AtlasError`] (or its [`AtlasResult`]
//! alias). Errors are:
//!
//! * **Structured** — every variant carries enough context to render a
//!   user-facing diagnostic (a stable error code, a backtrace when enabled,
//!   and optional source-chain metadata).
//! * **Miette-compatible** — the [`miette::Diagnostic`] impl powers fancy
//!   terminal and in-app error reports with source snippets.
//! * **Serializable** — `serde::Serialize` is implemented so errors can be
//!   sent across the EventBus, persisted to logs, or returned from plugin
//!   sandbox boundaries.

use std::fmt;
use std::io;
use std::sync::Arc;

use miette::Diagnostic;
use serde::Serialize;
use thiserror::Error;

/// The canonical result type for the entire workspace.
///
/// Prefer this over `std::result::Result<T, AtlasError>` in public APIs.
pub type AtlasResult<T> = std::result::Result<T, AtlasError>;

/// Root error enum for AtlasDB Studio.
///
/// Variants are grouped by subsystem. The `with_source` helper lets callers
/// attach lower-level errors (e.g. `sqlparser` or `tokio::io`) without losing
/// the structured top-level context.
///
/// # Examples
///
/// ```
/// use atlas_core::AtlasError;
///
/// fn connect() -> atlas_core::AtlasResult<()> {
///     Err(AtlasError::Connection {
///         source: "postgres://…".into(),
///         reason: "timeout".into(),
///         cause: None,
///     })
/// }
/// ```
#[derive(Debug, Error, Diagnostic, Serialize)]
#[error("E{code:04}: {message}")]
pub enum AtlasError {
    /// An operation was cancelled by the user or a supervisor.
    #[error("E0001: operation cancelled")]
    Cancelled,

    /// A connection could not be established, authenticated, or was lost.
    #[error("E0100: connection to {source} failed: {reason}")]
    Connection {
        /// Connection string or display name (credentials redacted).
        source: String,
        /// Human-readable reason.
        reason: String,
        /// Optional lower-level error.
        #[serde(skip)]
        #[source]
        cause: Option<Box<AtlasError>>,
    },

    /// A query failed to parse.
    #[error("E0200: parse error at line {line}: {detail}")]
    #[diagnostic(help("Check the SQL syntax around the highlighted range."))]
    Parse {
        /// 1-indexed line number.
        line: u32,
        /// 1-indexed column number.
        column: u32,
        /// Detail message from the parser.
        detail: String,
        /// Optional source snippet.
        #[serde(skip)]
        snippet: Option<Arc<str>>,
    },

    /// A query was rejected by the engine (syntax valid, semantics invalid).
    #[error("E0210: query rejected: {detail}")]
    Query {
        /// Engine-returned error code, when available.
        code: Option<String>,
        /// Human-readable rejection reason.
        detail: String,
    },

    /// A query timed out.
    #[error("E0220: query timed out after {limit_ms}ms")]
    Timeout {
        /// Timeout limit in milliseconds.
        limit_ms: u64,
    },

    /// An operation referenced an entity that does not exist.
    #[error("E0300: not found: {kind} '{name}'")]
    NotFound {
        /// The kind of entity (e.g. "table", "schema", "connection").
        kind: String,
        /// The name that was searched for.
        name: String,
    },

    /// The user is not authorized to perform an operation.
    #[error("E0400: unauthorized: {reason}")]
    Unauthorized {
        /// Explanation of why access was denied.
        reason: String,
    },

    /// A secret could not be read, written, or decrypted.
    #[error("E0500: secrets error: {detail}")]
    Secrets {
        /// Description of the secrets failure.
        detail: String,
    },

    /// Configuration is invalid or missing.
    #[error("E0600: config error: {detail}")]
    Config {
        /// Description of the configuration problem.
        detail: String,
    },

    /// An I/O error from the standard library.
    #[error("E0900: I/O error: {detail}")]
    Io {
        /// Human-readable description.
        detail: String,
        /// The underlying I/O error.
        #[serde(skip)]
        #[source]
        cause: io::Error,
    },

    /// A subsystem returned an opaque error.
    ///
    /// This is the catch-all for third-party errors we haven't mapped yet.
    /// New code should prefer a specific variant; this exists so that
    /// `?` conversions from `anyhow::Error` never lose the source chain.
    #[error("E0999: {detail}")]
    Other {
        /// Human-readable description.
        detail: String,
        /// Optional lower-level error for diagnostics.
        #[serde(skip)]
        #[source]
        cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
}

impl AtlasError {
    /// Machine-readable error code (e.g. `0100` for connection errors).
    pub fn code(&self) -> u16 {
        match self {
            Self::Cancelled => 1,
            Self::Connection { .. } => 100,
            Self::Parse { .. } => 200,
            Self::Query { .. } => 210,
            Self::Timeout { .. } => 220,
            Self::NotFound { .. } => 300,
            Self::Unauthorized { .. } => 400,
            Self::Secrets { .. } => 500,
            Self::Config { .. } => 600,
            Self::Io { .. } => 900,
            Self::Other { .. } => 999,
        }
    }

    /// Returns `true` if the error is likely transient and the user should
    /// be offered a "Retry" affordance.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Connection { .. } | Self::Timeout { .. } | Self::Io { .. }
        )
    }

    /// Attach a lower-level cause, returning a new error.
    pub fn with_source<E: std::error::Error + Send + Sync + 'static>(
        self,
        cause: E,
    ) -> Self {
        match self {
            Self::Connection {
                source, reason, ..
            } => Self::Connection {
                source,
                reason,
                cause: Some(Box::new(AtlasError::Other {
                    detail: cause.to_string(),
                    cause: Some(Box::new(cause)),
                })),
            },
            _ => self,
        }
    }
}

// Conversions from common error types. These are the *only* places in the
// workspace where `?` on a non-AtlasError is allowed.

impl From<io::Error> for AtlasError {
    fn from(e: io::Error) -> Self {
        Self::Io {
            detail: e.to_string(),
            cause: e,
        }
    }
}

impl From<serde_json::Error> for AtlasError {
    fn from(e: serde_json::Error) -> Self {
        Self::Other {
            detail: e.to_string(),
            cause: Some(Box::new(e)),
        }
    }
}

impl From<figment::Error> for AtlasError {
    fn from(e: figment::Error) -> Self {
        Self::Config {
            detail: e.to_string(),
        }
    }
}

/// Helper to build an [`AtlasError::Other`] from any displayable message.
pub fn err_msg(msg: impl fmt::Display) -> AtlasError {
    AtlasError::Other {
        detail: msg.to_string(),
        cause: None,
    }
}

/// Extension trait: `Result<T, E>` → `Result<T, AtlasError>`.
///
/// Use this in crates that don't want to import the full [`AtlasError`] enum
/// but still want `?` to work.
pub trait IntoAtlas<T, E> {
    /// Convert the error into an [`AtlasError::Other`] with `E`'s `Display`.
    fn into_atlas(self) -> AtlasResult<T>;
}

impl<T, E: fmt::Display> IntoAtlas<T, E> for std::result::Result<T, E> {
    fn into_atlas(self) -> AtlasResult<T> {
        self.map_err(|e| AtlasError::Other {
            detail: e.to_string(),
            cause: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_is_stable() {
        let e = AtlasError::Connection {
            source: "pg://localhost".into(),
            reason: "refused".into(),
            cause: None,
        };
        assert_eq!(e.code(), 100);
        assert!(e.is_retryable());
    }

    #[test]
    fn io_conversion_preserves_message() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let e = AtlasError::from(io);
        match &e {
            AtlasError::Io { detail, .. } => assert!(detail.contains("gone")),
            other => panic!("expected Io variant, got {other:?}"),
        }
    }

    #[test]
    fn err_msg_helper() {
        let e = err_msg("boom");
        assert_eq!(e.code(), 999);
    }

    #[test]
    fn into_atlas_maps_display() {
        let r: Result<(), String> = Err("oops".into());
        let e = r.into_atlas().unwrap_err();
        assert_eq!(e.code(), 999);
    }
}
