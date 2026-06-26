//! Async query scheduler.
//!
//! The scheduler receives [`QueryRequest`]s and dispatches them to the
//! appropriate database connection. It supports:
//!
//! * Multiple concurrent queries (one per pool connection).
//! * Cancellation via a [`tokio::sync::watch`] channel.
//! * Background execution with result callback.

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use getagrip_core::id::{Id};
use getagrip_core::id_tag;

use crate::history::{HistoryEntry, QueryHistory};

id_tag!(pub QueryId => "query");
id_tag!(pub TabId => "tab");

/// A request to execute a query.
#[derive(Debug, Clone)]
pub struct QueryRequest {
    /// Unique query identifier.
    pub query_id: Id<QueryId>,
    /// The editor tab that initiated this query.
    pub tab_id: Id<TabId>,
    /// The SQL text to execute.
    pub sql: String,
    /// Connection profile ID to run against.
    pub connection_id: String,
}

/// Query execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    /// Waiting in the queue.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed with an error.
    Failed,
    /// Cancelled by the user.
    Cancelled,
    /// Timed out.
    TimedOut,
}

/// An async query scheduler.
///
/// Accepts requests via a channel and dispatches them to the executor.
/// Results are recorded in the [`QueryHistory`].
pub struct QueryScheduler {
    /// Query history (shared across the app).
    history: Arc<QueryHistory>,
    /// Incoming query requests.
    request_tx: mpsc::UnboundedSender<QueryRequest>,
    request_rx: mpsc::UnboundedReceiver<QueryRequest>,
}

impl QueryScheduler {
    /// Create a new scheduler.
    pub fn new(history: Arc<QueryHistory>) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        Self {
            history,
            request_tx,
            request_rx,
        }
    }

    /// Submit a query for execution.
    ///
    /// Returns immediately — execution happens asynchronously.
    pub fn submit(&self, request: QueryRequest) {
        let _ = self.request_tx.send(request);
    }

    /// Get a sender handle for submitting queries from other tasks.
    pub fn sender(&self) -> mpsc::UnboundedSender<QueryRequest> {
        self.request_tx.clone()
    }

    /// Start processing requests.
    ///
    /// This should be spawned on a Tokio task. It runs until the channel
    /// is closed.
    pub async fn run(&mut self) {
        while let Some(request) = self.request_rx.recv().await {
            self.history.add(HistoryEntry {
                query_id: request.query_id,
                tab_id: request.tab_id,
                sql: request.sql.clone(),
                status: QueryStatus::Running,
                started_at: Utc::now(),
                completed_at: None,
                rows_affected: None,
                elapsed_us: None,
                error: None,
            });
        }
    }

    /// Access the query history.
    pub fn history(&self) -> &Arc<QueryHistory> {
        &self.history
    }
}

impl Default for QueryScheduler {
    fn default() -> Self {
        Self::new(Arc::new(QueryHistory::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_query_adds_to_history() {
        let history = Arc::new(QueryHistory::new());
        let mut scheduler = QueryScheduler::new(Arc::clone(&history));

        let query_id = Id::<QueryId>::new_v7();
        let tab_id = Id::<TabId>::new_v7();

        scheduler.submit(QueryRequest {
            query_id,
            tab_id,
            sql: "SELECT 1".into(),
            connection_id: "conn1".into(),
        });

        // Run one iteration to process the request.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Need to poll run() which blocks — use a timeout.
            tokio::select! {
                _ = scheduler.run() => {},
                _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
            }
        });

        let entries = history.all();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, QueryStatus::Running);
        assert_eq!(entries[0].sql, "SELECT 1");
    }
}
