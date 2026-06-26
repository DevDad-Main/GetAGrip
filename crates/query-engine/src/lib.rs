//! GetAGrip SQL query engine.
//!
//! Provides SQL parsing via tree-sitter, query execution orchestration,
//! EXPLAIN plan analysis, and SQL formatting capabilities.

pub mod completion;
pub mod executor;
pub mod explain;
pub mod formatter;
pub mod parser;

use std::sync::Arc;
use parking_lot::RwLock;
use tg_core::cancel::CancellationToken;
use tg_core::error::CoreResult;
use tg_core::types::query::QueryId;
use tg_core::result::QueryResult;
use tg_core::traits::driver::Connection;
use tg_core::types::query::{ExecutionPlan, ExplainFormat, Pagination, QueryParams, QueryStatus};

/// The query engine orchestrates parsing, analysis, execution, and formatting.
pub struct QueryEngine {
    /// Active queries being tracked.
    active_queries: dashmap::DashMap<QueryId, Arc<RwLock<QueryState>>>,
}

#[derive(Debug)]
struct QueryState {
    status: QueryStatus,
    sql: String,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl QueryEngine {
    /// Create a new query engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            active_queries: dashmap::DashMap::new(),
        }
    }

    /// Execute a query against a connection.
    pub async fn execute(
        &self,
        conn: &dyn Connection,
        sql: &str,
        params: Option<QueryParams>,
        pagination: Option<Pagination>,
        cancel: CancellationToken,
    ) -> CoreResult<QueryResult> {
        let query_id = QueryId::new();
        let started_at = chrono::Utc::now();

        // Track the query
        self.active_queries.insert(
            query_id,
            Arc::new(RwLock::new(QueryState {
                status: QueryStatus::Running,
                sql: sql.to_string(),
                started_at,
                finished_at: None,
            })),
        );

        let result = conn.execute(sql, params, pagination, cancel).await;

        // Update tracking
        if let Some(state) = self.active_queries.get(&query_id) {
            let mut state = state.write();
            state.status = if result.is_ok() {
                QueryStatus::Completed
            } else {
                QueryStatus::Failed
            };
            state.finished_at = Some(chrono::Utc::now());
        }

        result
    }

    /// Get an EXPLAIN plan for a query.
    pub async fn explain(
        &self,
        conn: &dyn Connection,
        sql: &str,
        format: ExplainFormat,
        analyze: bool,
        cancel: CancellationToken,
    ) -> CoreResult<ExecutionPlan> {
        conn.explain(sql, format, analyze, cancel).await
    }

    /// Cancel a running query by its ID.
    pub async fn cancel(&self, query_id: QueryId) -> CoreResult<()> {
        if let Some(state) = self.active_queries.get(&query_id) {
            state.write().status = QueryStatus::Cancelled;
        }
        Ok(())
    }

    /// Get the current status of a query.
    pub fn query_status(&self, query_id: QueryId) -> Option<QueryStatus> {
        self.active_queries
            .get(&query_id)
            .map(|s| s.read().status)
    }

    /// List all tracked query IDs with their status.
    pub fn list_queries(&self) -> Vec<(QueryId, QueryStatus, String)> {
        self.active_queries
            .iter()
            .map(|entry| {
                let state = entry.value().read();
                (*entry.key(), state.status, state.sql.clone())
            })
            .collect()
    }

    /// Clean up completed/cancelled queries older than the given duration.
    pub fn cleanup(&self, _older_than: std::time::Duration) -> usize {
        let mut removed = 0;
        self.active_queries.retain(|_, state| {
            let s = state.read();
            let keep = s.status == QueryStatus::Running || s.status == QueryStatus::Pending;
            if !keep {
                removed += 1;
            }
            keep
        });
        removed
    }

    /// Parse SQL and return the syntax tree (for LSP-like features).
    pub fn parse_sql(&self, sql: &str) -> CoreResult<parser::SqlAst> {
        parser::parse(sql)
    }

    /// Get completions at a given position in SQL text.
    pub fn completions_at(
        &self,
        sql: &str,
        cursor_offset: usize,
        conn: Option<&dyn Connection>,
    ) -> Vec<completion::CompletionItem> {
        completion::get_completions(sql, cursor_offset, conn)
    }

    /// Format SQL text.
    pub fn format_sql(&self, sql: &str) -> CoreResult<String> {
        formatter::format(sql)
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new()
    }
}
