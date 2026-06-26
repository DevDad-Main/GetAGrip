//! Query execution history.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use atlas_core::id::Id;

use crate::scheduler::{QueryId, QueryStatus, TabId};

/// A single entry in the query execution history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique query identifier.
    pub query_id: Id<QueryId>,
    /// The editor tab that initiated this query.
    pub tab_id: Id<TabId>,
    /// The original SQL text.
    pub sql: String,
    /// Execution status.
    pub status: QueryStatus,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution completed (set after completion).
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of rows affected, if applicable.
    pub rows_affected: Option<u64>,
    /// Elapsed time in microseconds.
    pub elapsed_us: Option<u64>,
    /// Error message, if failed.
    pub error: Option<String>,
}

impl HistoryEntry {
    /// Whether this entry represents a completed successful execution.
    pub fn is_success(&self) -> bool {
        self.status == QueryStatus::Completed
    }

    /// Whether this entry represents a failure.
    pub fn is_failure(&self) -> bool {
        matches!(self.status, QueryStatus::Failed | QueryStatus::TimedOut)
    }
}

/// Thread-safe query execution history.
///
/// Stores all queries executed during a session. In a future phase this
/// will be persisted to disk and limited to a configurable maximum size.
pub struct QueryHistory {
    entries: RwLock<Vec<HistoryEntry>>,
}

impl QueryHistory {
    /// Create an empty history.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    /// Add an entry to the history.
    pub fn add(&self, entry: HistoryEntry) {
        self.entries.write().push(entry);
    }

    /// Update the status of an existing entry.
    pub fn update(&self, query_id: Id<QueryId>, status: QueryStatus, elapsed_us: u64, error: Option<String>) {
        let mut entries = self.entries.write();
        if let Some(entry) = entries.iter_mut().find(|e| e.query_id == query_id) {
            entry.status = status;
            entry.completed_at = Some(Utc::now());
            entry.elapsed_us = Some(elapsed_us);
            entry.error = error;
        }
    }

    /// Return all entries (most recent first).
    pub fn all(&self) -> Vec<HistoryEntry> {
        let entries = self.entries.read();
        let mut v: Vec<_> = entries.clone();
        v.reverse(); // most recent first
        v
    }

    /// Return the most recent `n` entries.
    pub fn recent(&self, n: usize) -> Vec<HistoryEntry> {
        let mut all = self.all();
        all.truncate(n);
        all
    }

    /// Total number of entries.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.entries.write().clear();
    }
}

impl Default for QueryHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_retrieve() {
        let history = QueryHistory::new();
        let entry = HistoryEntry {
            query_id: Id::<QueryId>::new_v7(),
            tab_id: Id::<TabId>::new_v7(),
            sql: "SELECT 1".into(),
            status: QueryStatus::Completed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            rows_affected: None,
            elapsed_us: Some(100),
            error: None,
        };
        history.add(entry.clone());
        assert_eq!(history.len(), 1);
        assert!(history.all()[0].is_success());
    }

    #[test]
    fn update_entry() {
        let history = QueryHistory::new();
        let qid = Id::<QueryId>::new_v7();
        history.add(HistoryEntry {
            query_id: qid,
            tab_id: Id::<TabId>::new_v7(),
            sql: "SELECT 1".into(),
            status: QueryStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            rows_affected: None,
            elapsed_us: None,
            error: None,
        });

        history.update(qid, QueryStatus::Completed, 500, None);
        let entry = &history.all()[0];
        assert_eq!(entry.status, QueryStatus::Completed);
        assert_eq!(entry.elapsed_us, Some(500));
    }

    #[test]
    fn recent_truncates() {
        let history = QueryHistory::new();
        for _ in 0..5 {
            history.add(HistoryEntry {
                query_id: Id::<QueryId>::new_v7(),
                tab_id: Id::<TabId>::new_v7(),
                sql: "SELECT 1".into(),
                status: QueryStatus::Completed,
                started_at: Utc::now(),
                completed_at: None,
                rows_affected: None,
                elapsed_us: None,
                error: None,
            });
        }
        assert_eq!(history.recent(3).len(), 3);
    }
}
