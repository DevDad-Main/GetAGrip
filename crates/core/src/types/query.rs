//! Query execution types.

use serde::{Deserialize, Serialize};
use crate::types::connection::ConnectionId;
use std::collections::HashMap;

/// A unique identifier for a query.
pub use crate::id::QueryTag as QueryIdTag;
pub type QueryId = crate::id::Id<QueryIdTag>;

/// The status of a running query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryStatus {
    /// Query is queued but not yet executing.
    Pending,
    /// Query is executing.
    Running,
    /// Query completed successfully.
    Completed,
    /// Query failed with an error.
    Failed,
    /// Query was cancelled by the user.
    Cancelled,
    /// Query timed out.
    TimedOut,
}

/// Named or positional query parameters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QueryParams {
    /// Named parameters (e.g., :name, @name).
    pub named: HashMap<String, serde_json::Value>,
    /// Positional parameters ($1, $2, ...).
    pub positional: Vec<serde_json::Value>,
}

impl QueryParams {
    /// Create empty params.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a named parameter.
    pub fn with_named(mut self, name: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.named.insert(name.into(), value.into());
        self
    }

    /// Add a positional parameter.
    pub fn with_positional(mut self, value: impl Into<serde_json::Value>) -> Self {
        self.positional.push(value.into());
        self
    }
}

/// Pagination configuration for query results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pagination {
    /// Offset in rows.
    pub offset: u64,
    /// Maximum rows to return.
    pub limit: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 1000,
        }
    }
}

/// Metadata associated with a query.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query identifier.
    pub id: QueryId,
    /// The SQL text.
    pub sql: String,
    /// The connection used.
    pub connection_id: ConnectionId,
    /// Execution status.
    pub status: QueryStatus,
    /// When the query started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the query finished, if complete.
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Execution duration in milliseconds.
    pub elapsed_ms: Option<u64>,
    /// Number of rows returned.
    pub row_count: Option<u64>,
    /// Number of rows affected.
    pub rows_affected: Option<u64>,
    /// Error message, if failed.
    pub error: Option<String>,
    /// Whether the query was run with EXPLAIN.
    pub was_explained: bool,
    /// User-provided label.
    pub label: Option<String>,
    /// The database/schema context.
    pub context: Option<QueryContext>,
    /// Tags for filtering history.
    pub tags: Vec<String>,
}

/// Database context for a query.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryContext {
    /// Database name.
    pub database: Option<String>,
    /// Schema name.
    pub schema: Option<String>,
}

/// The format for EXPLAIN output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplainFormat {
    /// Tabular text output.
    Text,
    /// JSON format.
    Json,
    /// YAML format.
    Yaml,
    /// XML format.
    Xml,
    /// Visual graph representation.
    Graph,
}

/// A node in an execution plan tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlanNode {
    /// The operation type (e.g., "Seq Scan", "Index Scan", "Hash Join").
    pub operation: String,
    /// Human-readable description of what this node does.
    pub description: Option<String>,
    /// Startup cost.
    pub startup_cost: Option<f64>,
    /// Total cost.
    pub total_cost: Option<f64>,
    /// Estimated rows.
    pub plan_rows: Option<u64>,
    /// Estimated row width in bytes.
    pub plan_width: Option<u64>,
    /// Actual time in milliseconds (from EXPLAIN ANALYZE).
    pub actual_time_ms: Option<f64>,
    /// Actual rows (from EXPLAIN ANALYZE).
    pub actual_rows: Option<u64>,
    /// Memory used.
    pub memory_kb: Option<u64>,
    /// Additional properties from the planner.
    pub properties: HashMap<String, String>,
    /// Child nodes.
    pub children: Vec<ExecutionPlanNode>,
    /// Warnings or issues with this node.
    pub warnings: Vec<String>,
    /// Optimization suggestions.
    pub suggestions: Vec<String>,
}

/// A complete execution plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// The root node of the plan tree.
    pub root: ExecutionPlanNode,
    /// Total planning time in milliseconds.
    pub planning_time_ms: Option<f64>,
    /// Total execution time in milliseconds.
    pub execution_time_ms: Option<f64>,
    /// Overall triggers fired.
    pub triggers: Option<u64>,
    /// The original raw plan text.
    pub raw_text: Option<String>,
    /// The output format this plan represents.
    pub format: ExplainFormat,
}
