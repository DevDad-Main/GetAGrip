//! GetAGrip — core types, traits, and abstractions.
//!
//! This crate provides the foundational building blocks used by all other
//! GetAGrip crates. It contains no platform-specific or UI code.

pub mod cancel;
pub mod error;
pub mod id;
pub mod result;
pub mod time;
pub mod traits;
pub mod types;

/// Prelude module for convenient imports throughout the codebase.
pub mod prelude {
    pub use crate::cancel::CancellationToken;
    pub use crate::error::{CoreError, CoreResult};
    pub use crate::id::Id;
    pub use crate::result::{DataCell, DataRow, DataType, QueryResult, Value};
    pub use crate::time::Timestamp;
    pub use crate::types::column::ColumnInfo;
    pub use crate::types::connection::{
        ConnectionDriver, ConnectionId, ConnectionInfo, ConnectionStatus, DatabaseKind,
        SslMode, TunnelConfig,
    };
    pub use crate::types::database::{
        ColumnMetadata, ConstraintKind, DatabaseObject, DatabaseObjectKind, DbType, IndexInfo,
        ProcedureInfo, SchemaInfo, TableInfo, ViewInfo,
    };
    pub use crate::types::query::{
        ExecutionPlan, ExecutionPlanNode, ExplainFormat, Pagination, QueryId,
        QueryMetadata, QueryParams, QueryStatus,
    };
}
