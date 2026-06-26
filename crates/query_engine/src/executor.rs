//! Single-query execution context.
//!
//! A [`QueryExecutor`] wraps a query request and provides a future that
//! resolves to an [`ExecutionResult`]. It supports cancellation via a
//! watch channel.

use chrono::{DateTime, Utc};
use tokio::sync::watch;

use crate::scheduler::{QueryRequest, QueryStatus};

/// The result of a query execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The query that was executed.
    pub request: QueryRequest,
    /// Final status.
    pub status: QueryStatus,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution completed.
    pub completed_at: DateTime<Utc>,
    /// Elapsed time in microseconds.
    pub elapsed_us: u64,
    /// Number of rows affected (for mutations).
    pub rows_affected: Option<u64>,
    /// Error message, if failed.
    pub error: Option<String>,
}

/// A handle to a running query, used for cancellation.
pub struct QueryHandle {
    /// Send `true` to request cancellation.
    cancel_tx: watch::Sender<bool>,
    /// Unique query id.
    query_id: String,
}

impl QueryHandle {
    /// Request cancellation of the query.
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }

    /// The query ID.
    pub fn query_id(&self) -> &str {
        &self.query_id
    }
}

/// Executes a single query against a database connection.
///
/// This is the core execution primitive. It is typically spawned as a
/// Tokio task by the [`super::scheduler::QueryScheduler`].
pub struct QueryExecutor {
    request: QueryRequest,
    cancel_rx: watch::Receiver<bool>,
}

impl QueryExecutor {
    /// Create a new executor for a query request.
    pub fn new(request: QueryRequest) -> (Self, QueryHandle) {
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let query_id = request.query_id.to_string();
        (
            Self { request, cancel_rx },
            QueryHandle { cancel_tx, query_id },
        )
    }

    /// Check whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        *self.cancel_rx.borrow()
    }

    /// The underlying request.
    pub fn request(&self) -> &QueryRequest {
        &self.request
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{QueryId, QueryRequest, TabId};
    use atlas_core::id::Id;

    #[test]
    fn cancel_signals_executor() {
        let query_id = Id::<QueryId>::new_v7();
        let tab_id = Id::<TabId>::new_v7();
        let request = QueryRequest {
            query_id,
            tab_id,
            sql: "SELECT 1".into(),
            connection_id: "c1".into(),
        };
        let (executor, handle) = QueryExecutor::new(request);

        assert!(!executor.is_cancelled());
        handle.cancel();
        assert!(executor.is_cancelled());
    }

    #[test]
    fn handle_query_id_matches() {
        let query_id = Id::<QueryId>::new_v7();
        let tab_id = Id::<TabId>::new_v7();
        let request = QueryRequest {
            query_id,
            tab_id,
            sql: "SELECT 1".into(),
            connection_id: "c1".into(),
        };
        let (_executor, handle) = QueryExecutor::new(request);
        assert_eq!(handle.query_id(), query_id.to_string());
    }
}
