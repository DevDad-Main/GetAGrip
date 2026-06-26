//! Query result types used throughout GetAGrip.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a single typed value from a database cell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// SQL NULL.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Signed 8-bit integer.
    I8(i8),
    /// Signed 16-bit integer.
    I16(i16),
    /// Signed 32-bit integer.
    I32(i32),
    /// Signed 64-bit integer.
    I64(i64),
    /// Unsigned 64-bit integer.
    U64(u64),
    /// 32-bit floating point.
    F32(f32),
    /// 64-bit floating point.
    F64(f64),
    /// String value.
    String(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
    /// A UUID.
    Uuid(uuid::Uuid),
    /// Date (no time component).
    Date(chrono::NaiveDate),
    /// Time (no date component).
    Time(chrono::NaiveTime),
    /// Date and time (no timezone).
    DateTime(chrono::NaiveDateTime),
    /// Date and time with timezone.
    DateTimeTz(chrono::DateTime<chrono::Utc>),
    /// A JSON value (stored as a string representation).
    Json(String),
    /// An array of values.
    Array(Vec<Value>),
    /// A generic representation (display only).
    Display(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "NULL"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::I8(v) => write!(f, "{v}"),
            Self::I16(v) => write!(f, "{v}"),
            Self::I32(v) => write!(f, "{v}"),
            Self::I64(v) => write!(f, "{v}"),
            Self::U64(v) => write!(f, "{v}"),
            Self::F32(v) => write!(f, "{v}"),
            Self::F64(v) => write!(f, "{v}"),
            Self::String(s) | Self::Json(s) | Self::Display(s) => write!(f, "{s}"),
            Self::Bytes(b) => write!(f, "{}", hex_encode(b)),
            Self::Uuid(u) => write!(f, "{u}"),
            Self::Date(d) => write!(f, "{d}"),
            Self::Time(t) => write!(f, "{t}"),
            Self::DateTime(dt) => write!(f, "{dt}"),
            Self::DateTimeTz(dt) => write!(f, "{dt}"),
            Self::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// The database type of a column.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    /// Unknown or unsupported type.
    Unknown,
    /// NULL.
    Null,
    /// Boolean.
    Boolean,
    /// 8-bit signed integer.
    TinyInt,
    /// 16-bit signed integer.
    SmallInt,
    /// 32-bit signed integer.
    Integer,
    /// 64-bit signed integer.
    BigInt,
    /// Arbitrary precision numeric.
    Numeric { precision: Option<u32>, scale: Option<u32> },
    /// 32-bit float.
    Float,
    /// 64-bit float.
    Double,
    /// A fixed-length string.
    Char(usize),
    /// A variable-length string.
    VarChar(Option<usize>),
    /// Unlimited text.
    Text,
    /// Raw binary data.
    Binary,
    /// A UUID.
    Uuid,
    /// Date only.
    Date,
    /// Time only.
    Time,
    /// Timestamp without timezone.
    Timestamp,
    /// Timestamp with timezone.
    TimestampTz,
    /// Interval.
    Interval,
    /// JSON.
    Json,
    /// JSONB.
    Jsonb,
    /// Array of a specific type.
    Array(Box<DataType>),
    /// Enum with possible values.
    Enum(Vec<String>),
    /// A database-specific type.
    Custom(String),
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Null => write!(f, "null"),
            Self::Boolean => write!(f, "boolean"),
            Self::TinyInt => write!(f, "tinyint"),
            Self::SmallInt => write!(f, "smallint"),
            Self::Integer => write!(f, "integer"),
            Self::BigInt => write!(f, "bigint"),
            Self::Numeric { precision, scale } => {
                write!(f, "numeric")?;
                if let Some(p) = precision {
                    write!(f, "({p}")?;
                    if let Some(s) = scale {
                        write!(f, ",{s}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Self::Float => write!(f, "float"),
            Self::Double => write!(f, "double"),
            Self::Char(n) => write!(f, "char({n})"),
            Self::VarChar(None) => write!(f, "varchar"),
            Self::VarChar(Some(n)) => write!(f, "varchar({n})"),
            Self::Text => write!(f, "text"),
            Self::Binary => write!(f, "binary"),
            Self::Uuid => write!(f, "uuid"),
            Self::Date => write!(f, "date"),
            Self::Time => write!(f, "time"),
            Self::Timestamp => write!(f, "timestamp"),
            Self::TimestampTz => write!(f, "timestamptz"),
            Self::Interval => write!(f, "interval"),
            Self::Json => write!(f, "json"),
            Self::Jsonb => write!(f, "jsonb"),
            Self::Array(inner) => write!(f, "{inner}[]"),
            Self::Enum(variants) => {
                write!(f, "enum({})", variants.join(", "))
            }
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

/// A single cell in a result row.
pub type DataCell = Value;

/// A single row in a query result.
pub type DataRow = Vec<DataCell>;

/// The complete result of a query execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResult {
    /// Column metadata.
    pub columns: Vec<ColumnMetadata>,
    /// Row data.
    pub rows: Vec<DataRow>,
    /// Total row count (may exceed `rows.len()` if paginated).
    pub total_rows: Option<u64>,
    /// Execution duration in milliseconds.
    pub elapsed_ms: u64,
    /// Number of rows affected (for INSERT/UPDATE/DELETE).
    pub rows_affected: Option<u64>,
    /// Whether more rows are available (for streaming).
    pub has_more: bool,
    /// Execution plan, if requested.
    pub explain_plan: Option<String>,
    /// Arbitrary driver-specific metadata.
    pub metadata: serde_json::Value,
}

/// Column metadata for a query result.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnMetadata {
    /// Column name.
    pub name: String,
    /// Column data type.
    pub data_type: DataType,
    /// Whether the column is nullable.
    pub nullable: bool,
    /// Whether this is a primary key column.
    pub is_primary_key: bool,
    /// Whether this is a foreign key column.
    pub is_foreign_key: bool,
    /// Original table name (if from a table).
    pub table_name: Option<String>,
    /// Original schema name.
    pub schema_name: Option<String>,
    /// Database name.
    pub database_name: Option<String>,
    /// Column ordinal position (1-indexed).
    pub ordinal: usize,
    /// Default value expression, if any.
    pub default_value: Option<String>,
    /// Character maximum length, if applicable.
    pub char_max_length: Option<u64>,
    /// Numeric precision, if applicable.
    pub numeric_precision: Option<u32>,
    /// Numeric scale, if applicable.
    pub numeric_scale: Option<u32>,
}
