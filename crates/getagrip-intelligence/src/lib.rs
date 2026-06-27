//! GetAGrip Intelligence Engine — metadata caching, SQL completion, and diagnostics.
//!
//! This crate serves as the "brain" of GetAGrip. It:
//! - Caches database metadata (schemas, tables, columns) in memory
//! - Analyses SQL context at the cursor (aliases, table references, scope)
//! - Generates context-aware completion suggestions
//! - Provides semantic SQL diagnostics

pub mod completion;
pub mod context;
pub mod diagnostics;
pub mod metadata;
pub mod types;

pub use metadata::MetadataCache;
pub use types::{
    CompletionItem, CompletionKind, CompletionRequest, CompletionResponse,
    DiagnosticItem, DiagnosticLevel, DiagnosticsRequest, DiagnosticsResponse,
    MetadataRefreshRequest,
};
