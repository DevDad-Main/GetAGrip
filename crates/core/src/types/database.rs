//! Database object metadata types.

use serde::{Deserialize, Serialize};
pub use crate::result::ColumnMetadata;
pub use crate::result::DataType;
pub use crate::result::DataType as DbType;
pub use crate::types::column::ColumnInfo;

/// Metadata for a database schema.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Schema name.
    pub name: String,
    /// Schema owner.
    pub owner: Option<String>,
    /// Whether this is the default schema.
    pub is_default: bool,
    /// Schema comment/description.
    pub comment: Option<String>,
}

/// Metadata for a table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table name.
    pub name: String,
    /// Schema the table belongs to.
    pub schema: Option<String>,
    /// Database name.
    pub database: Option<String>,
    /// Table type (TABLE, VIEW, etc.).
    pub table_type: TableType,
    /// Estimated row count.
    pub estimated_rows: Option<u64>,
    /// Table size in bytes (approximate).
    pub size_bytes: Option<u64>,
    /// Columns in this table.
    pub columns: Vec<ColumnInfo>,
    /// Primary key columns.
    pub primary_key: Vec<String>,
    /// Indexes defined on this table.
    pub indexes: Vec<IndexInfo>,
    /// Foreign keys.
    pub foreign_keys: Vec<ForeignKeyInfo>,
    /// Table comment.
    pub comment: Option<String>,
    /// Whether the table is temporary.
    pub is_temporary: bool,
    /// Whether the table is partitioned.
    pub is_partitioned: bool,
}

/// Table type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableType {
    /// Regular table.
    Table,
    /// View.
    View,
    /// Materialized view.
    MaterializedView,
    /// External table.
    External,
    /// Temporary table.
    Temporary,
    /// System table.
    System,
}

/// Metadata for a view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewInfo {
    /// View name.
    pub name: String,
    /// Schema.
    pub schema: Option<String>,
    /// The view definition SQL.
    pub definition: Option<String>,
    /// Whether the view is updatable.
    pub is_updatable: bool,
    /// Whether this is a materialized view.
    pub is_materialized: bool,
    /// Columns exposed by the view.
    pub columns: Vec<ColumnInfo>,
    /// Comment.
    pub comment: Option<String>,
}

/// Index metadata.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndexInfo {
    /// Index name.
    pub name: String,
    /// Columns indexed (with optional sort order).
    pub columns: Vec<String>,
    /// Whether this is a unique index.
    pub is_unique: bool,
    /// Whether this is the primary key index.
    pub is_primary: bool,
    /// Index method (btree, hash, gist, etc.).
    pub method: Option<String>,
    /// Index definition SQL.
    pub definition: Option<String>,
    /// Whether the index is partial.
    pub is_partial: bool,
    /// Partial index predicate, if applicable.
    pub predicate: Option<String>,
}

/// Foreign key metadata.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    /// Constraint name.
    pub name: String,
    /// Source columns (in this table).
    pub columns: Vec<String>,
    /// Referenced table.
    pub referenced_table: String,
    /// Referenced schema.
    pub referenced_schema: Option<String>,
    /// Referenced columns.
    pub referenced_columns: Vec<String>,
    /// ON DELETE action.
    pub on_delete: Option<ForeignKeyAction>,
    /// ON UPDATE action.
    pub on_update: Option<ForeignKeyAction>,
}

/// Foreign key referential actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForeignKeyAction {
    /// RESTRICT.
    Restrict,
    /// CASCADE.
    Cascade,
    /// SET NULL.
    SetNull,
    /// SET DEFAULT.
    SetDefault,
    /// NO ACTION.
    NoAction,
}

/// Stored procedure / function metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureInfo {
    /// Routine name.
    pub name: String,
    /// Schema.
    pub schema: Option<String>,
    /// Routine type (function or procedure).
    pub routine_type: RoutineType,
    /// Return type, if a function.
    pub return_type: Option<DataType>,
    /// Parameter definitions.
    pub parameters: Vec<ProcedureParameter>,
    /// Routine body / definition SQL.
    pub definition: Option<String>,
    /// The language (SQL, plpgsql, python, etc.).
    pub language: Option<String>,
    /// Comment.
    pub comment: Option<String>,
}

/// Routine type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineType {
    /// A function (returns a value).
    Function,
    /// A procedure (no return value).
    Procedure,
    /// An aggregate function.
    Aggregate,
    /// A window function.
    Window,
}

/// A stored procedure parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcedureParameter {
    /// Parameter name.
    pub name: Option<String>,
    /// Data type.
    pub data_type: DataType,
    /// Parameter mode.
    pub mode: ParameterMode,
    /// Default value, if any.
    pub default_value: Option<String>,
    /// Ordinal position.
    pub ordinal: u32,
}

/// Parameter mode (IN, OUT, INOUT).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterMode {
    /// Input parameter.
    In,
    /// Output parameter.
    Out,
    /// Input/output parameter.
    InOut,
    /// Variadic parameter.
    Variadic,
}

/// The kind of a database object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseObjectKind {
    /// A server/instance.
    Server,
    /// A database.
    Database,
    /// A schema.
    Schema,
    /// A table.
    Table,
    /// A view.
    View,
    /// A materialized view.
    MaterializedView,
    /// A column.
    Column,
    /// An index.
    Index,
    /// A foreign key constraint.
    ForeignKey,
    /// A check constraint.
    CheckConstraint,
    /// A unique constraint.
    UniqueConstraint,
    /// A stored procedure.
    Procedure,
    /// A function.
    Function,
    /// A trigger.
    Trigger,
    /// A sequence.
    Sequence,
    /// A type.
    Type,
    /// An extension.
    Extension,
    /// A user/role.
    Role,
    /// A tablespace.
    Tablespace,
    /// A partition.
    Partition,
}

/// A generic database object reference.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabaseObject {
    /// Object kind.
    pub kind: DatabaseObjectKind,
    /// Object name.
    pub name: String,
    /// Schema name, if applicable.
    pub schema: Option<String>,
    /// Database name.
    pub database: Option<String>,
    /// Parent object name, if applicable.
    pub parent: Option<String>,
    /// Comment / description.
    pub comment: Option<String>,
}

/// Constraint kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintKind {
    /// Primary key.
    PrimaryKey,
    /// Foreign key.
    ForeignKey,
    /// Unique constraint.
    Unique,
    /// Check constraint.
    Check,
    /// Not null constraint.
    NotNull,
    /// Exclusion constraint.
    Exclusion,
}


