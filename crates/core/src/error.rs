//! Unified error types for GetAGrip.

use miette::Diagnostic;
use thiserror::Error;

/// The primary error type for the entire application.
#[derive(Debug, Error, Diagnostic)]
pub enum CoreError {
    /// A database error occurred.
    #[error("database error: {message}")]
    #[diagnostic(code("tg::db"))]
    Database {
        /// Human-readable message.
        message: String,
        /// Optional underlying error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A connection-related error occurred.
    #[error("connection error: {message}")]
    #[diagnostic(code("tg::conn"))]
    Connection {
        /// Human-readable message.
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A query execution error occurred.
    #[error("query error: {message}")]
    #[diagnostic(code("tg::query"))]
    Query {
        /// Human-readable message.
        message: String,
        /// The SQL that caused the error, if relevant.
        sql: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The operation was cancelled.
    #[error("operation cancelled")]
    #[diagnostic(code("tg::cancelled"))]
    Cancelled,

    /// A timeout occurred.
    #[error("timeout after {duration_ms}ms: {operation}")]
    #[diagnostic(code("tg::timeout"))]
    Timeout {
        /// What timed out.
        operation: String,
        /// Duration in milliseconds.
        duration_ms: u64,
    },

    /// Configuration is invalid or missing.
    #[error("configuration error: {message}")]
    #[diagnostic(code("tg::config"))]
    Config {
        /// Human-readable message.
        message: String,
    },

    /// An IO error occurred.
    #[error("io error: {message}")]
    #[diagnostic(code("tg::io"))]
    Io {
        /// Human-readable message.
        message: String,
        #[source]
        source: Option<std::io::Error>,
    },

    /// A serialization error occurred.
    #[error("serialization error: {message}")]
    #[diagnostic(code("tg::serde"))]
    Serialization {
        /// Human-readable message.
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An unsupported operation was attempted.
    #[error("unsupported operation: {message}")]
    #[diagnostic(code("tg::unsupported"))]
    Unsupported {
        /// Human-readable message.
        message: String,
    },

    /// An internal error that should never happen.
    #[error("internal error: {message}")]
    #[diagnostic(code("tg::internal"))]
    Internal {
        /// Human-readable message.
        message: String,
    },

    /// A plugin error occurred.
    #[error("plugin error: {message}")]
    #[diagnostic(code("tg::plugin"))]
    Plugin {
        /// Human-readable message.
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A theme error occurred.
    #[error("theme error: {message}")]
    #[diagnostic(code("tg::theme"))]
    Theme {
        /// Human-readable message.
        message: String,
    },

    /// Catch-all for external errors.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Convenience result type alias.
pub type CoreResult<T> = Result<T, CoreError>;

impl CoreError {
    /// Create a database error.
    #[must_use]
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
            source: None,
        }
    }

    /// Create a database error with a source.
    #[must_use]
    pub fn database_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Database {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a connection error.
    #[must_use]
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a query error.
    #[must_use]
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query {
            message: message.into(),
            sql: None,
            source: None,
        }
    }

    /// Create a query error with the offending SQL.
    #[must_use]
    pub fn query_with_sql(message: impl Into<String>, sql: impl Into<String>) -> Self {
        Self::Query {
            message: message.into(),
            sql: Some(sql.into()),
            source: None,
        }
    }

    /// Create a config error.
    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create an unsupported error.
    #[must_use]
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported {
            message: message.into(),
        }
    }

    /// Create an internal error.
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Returns `true` if this error means the operation was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }
}
