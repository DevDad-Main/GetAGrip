//! Query execution engine for GetAGrip.
//!
//! Manages query execution, scheduling, cancellation, and history.
//!
//! ## Architecture
//!
//! * [`scheduler`] — async query scheduler that runs queries against
//!   database connections.
//! * [`history`] — persisted query execution history with timing and
//!   result metadata.
//! * [`executor`] — single-query execution context with cancellation.

pub mod executor;
pub mod history;
pub mod scheduler;

pub use executor::{ExecutionResult, QueryExecutor};
pub use history::{HistoryEntry, QueryHistory};
pub use scheduler::{QueryRequest, QueryScheduler};
