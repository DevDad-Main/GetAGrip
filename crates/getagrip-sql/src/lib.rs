//! SQL parsing, analysis, and intelligence for GetAGrip.
//!
//! Wraps the `sqlparser` crate to provide:
//!
//! * Parse a SQL string into a syntax tree.
//! * Classify the query type (SELECT, INSERT, UPDATE, DELETE, DDL, etc.).
//! * Extract table and column references.
//! * Detect syntax errors with line/column positions.
//! * Format SQL via a configurable pretty-printer.
//!
//! ## Architecture
//!
//! * [`parser`] — thin wrapper over `sqlparser` with project error types.
//! * [`diagnostics`] — error detection and severity classification.
//! * [`formatter`] — configurable SQL pretty-printer.

pub mod diagnostics;
pub mod formatter;
pub mod parser;

pub use diagnostics::{Diagnostic, DiagnosticSeverity, Diagnostics};
pub use formatter::{FormatConfig, format};
pub use parser::{ParsedQuery, QueryType, parse, parse_with_diagnostics};
