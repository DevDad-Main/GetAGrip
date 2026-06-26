//! The [`DatabaseDriver`] trait and associated types.
//!
//! Every database driver (PostgreSQL, MySQL, SQLite, etc.) implements this
//! trait. The trait is async-first, designed to be spawned on a Tokio
//! runtime. Implementations must be `Send + Sync` so they can be shared
//! across tasks.

use std::collections::HashMap;
use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use atlas_core::AtlasResult;

// ---- Value ---------------------------------------------------------------

/// A database cell value.
///
/// This is deliberately not a 1:1 mapping to SQL types — it's the
/// lowest-common-denominator set that every driver can produce.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    /// `NULL`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A signed 64-bit integer.
    Int(i64),
    /// A 64-bit floating-point number.
    Float(f64),
    /// A UTF-8 string.
    String(String),
    /// Arbitrary bytes (e.g. `BYTEA`, `BLOB`).
    Bytes(Vec<u8>),
    /// A date/time value.
    DateTime(DateTime<Utc>),
    /// A UUID.
    Uuid(uuid::Uuid),
    /// A JSON value (stored as a string representation).
    Json(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("NULL"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::String(s) => f.write_str(s),
            Self::Bytes(b) => write!(f, "{}", base64_encode(b)),
            Self::DateTime(dt) => write!(f, "{dt}"),
            Self::Uuid(u) => write!(f, "{u}"),
            Self::Json(j) => f.write_str(j),
        }
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

// ---- Column types --------------------------------------------------------

/// Broad SQL type classification for a column.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    /// `BOOLEAN` / `BOOL`.
    Boolean,
    /// `SMALLINT` / `INTEGER` / `BIGINT`.
    Integer,
    /// `REAL` / `FLOAT` / `DOUBLE PRECISION` / `NUMERIC`.
    Float,
    /// `CHAR` / `VARCHAR` / `TEXT` / `CLOB`.
    String,
    /// `BYTEA` / `BLOB` / `BINARY`.
    Binary,
    /// `DATE` / `TIME` / `TIMESTAMP`.
    DateTime,
    /// `UUID`.
    Uuid,
    /// `JSON` / `JSONB`.
    Json,
    /// Any type not captured above.
    Other(String),
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean => f.write_str("boolean"),
            Self::Integer => f.write_str("integer"),
            Self::Float => f.write_str("float"),
            Self::String => f.write_str("string"),
            Self::Binary => f.write_str("binary"),
            Self::DateTime => f.write_str("datetime"),
            Self::Uuid => f.write_str("uuid"),
            Self::Json => f.write_str("json"),
            Self::Other(s) => f.write_str(s.as_str()),
        }
    }
}

// ---- Column info ---------------------------------------------------------

/// Metadata about a result-set column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// Column name (as returned by the database).
    pub name: String,
    /// Broad type classification.
    pub col_type: ColumnType,
    /// Original database-specific type name (e.g. `"character varying(255)"`).
    pub db_type: String,
    /// Whether the column is nullable.
    pub nullable: bool,
    /// Column ordinal position (0-based).
    pub ordinal: u16,
    /// Display size hint (characters), if known.
    pub size_hint: Option<u16>,
}

// ---- Result row ----------------------------------------------------------

/// A single row in a query result set.
///
/// Values are stored in column order and can be accessed by index or name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultRow {
    columns: Vec<ColumnInfo>,
    values: Vec<Value>,
    /// Name → index cache, built lazily.
    #[serde(skip)]
    name_index: Option<HashMap<String, usize>>,
}

impl ResultRow {
    /// Create a new row.
    pub fn new(columns: Vec<ColumnInfo>, values: Vec<Value>) -> Self {
        Self {
            columns,
            values,
            name_index: None,
        }
    }

    /// Number of columns.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the row has no columns.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get a value by column index.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Get a value by column name.
    pub fn get_by_name(&self, name: &str) -> Option<&Value> {
        let mut name_index = self.name_index.clone();
        let idx = name_index.get_or_insert_with(|| {
            self.columns
                .iter()
                .enumerate()
                .map(|(i, c)| (c.name.clone(), i))
                .collect()
        })
        .get(name)
        .copied();
        idx.and_then(|i| self.values.get(i))
    }

    /// Iterate over (column_info, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&ColumnInfo, &Value)> {
        self.columns.iter().zip(self.values.iter())
    }

    /// Access all raw values.
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Access column metadata.
    pub fn columns(&self) -> &[ColumnInfo] {
        &self.columns
    }
}

// ---- Query result --------------------------------------------------------

/// The result of executing a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Columns returned (empty for non-SELECT statements).
    pub columns: Vec<ColumnInfo>,
    /// Rows returned (empty for non-SELECT statements).
    pub rows: Vec<ResultRow>,
    /// Number of rows affected (for INSERT/UPDATE/DELETE).
    pub rows_affected: u64,
    /// Execution time from the database perspective (microseconds).
    pub elapsed_us: u64,
    /// Any warnings returned by the database.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl QueryResult {
    /// Whether this result set has data rows.
    pub fn has_rows(&self) -> bool {
        !self.rows.is_empty()
    }

    /// Whether this was a mutation (affected > 0 rows).
    pub fn is_mutation(&self) -> bool {
        self.rows_affected > 0
    }
}

// ---- Connection info -----------------------------------------------------

/// Information about a live database connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Database product name (e.g. "PostgreSQL").
    pub product_name: String,
    /// Database version string.
    pub version: ServerVersion,
    /// Current database name.
    pub database: String,
    /// Current schema / namespace.
    pub schema: String,
    /// Current user.
    pub user: String,
    /// Server process ID, if available.
    pub server_pid: Option<i64>,
    /// Whether the connection is read-only.
    pub read_only: bool,
}

/// Server version information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVersion {
    /// Major version number.
    pub major: u16,
    /// Minor version number.
    pub minor: u16,
    /// Patch version number.
    pub patch: u16,
    /// Raw version string.
    pub raw: String,
}

impl fmt::Display for ServerVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ---- Driver capabilities -------------------------------------------------

bitflags::bitflags! {
    /// Capabilities advertised by a database driver.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DriverCapability: u32 {
        /// Supports `SELECT` queries.
        const SELECT = 1 << 0;
        /// Supports DDL (`CREATE`, `ALTER`, `DROP`).
        const DDL = 1 << 1;
        /// Supports DML (`INSERT`, `UPDATE`, `DELETE`).
        const DML = 1 << 2;
        /// Supports transactions.
        const TRANSACTIONS = 1 << 3;
        /// Supports `EXPLAIN` plans.
        const EXPLAIN = 1 << 4;
        /// Supports prepared statements.
        const PREPARED = 1 << 5;
        /// Supports cancellable queries.
        const CANCEL = 1 << 6;
        /// Supports streaming (row-by-row) results.
        const STREAMING = 1 << 7;
        /// Supports multiple active result sets per connection.
        const MULTI_RESULT = 1 << 8;
        /// Supports schema introspection via `information_schema`.
        const INTROSPECTION = 1 << 9;
    }
}

// ---- The driver trait ----------------------------------------------------

/// The core trait every database driver must implement.
///
/// Drivers are expected to be `Send + Sync` and all methods are async so
/// they integrate cleanly with the Tokio runtime.
#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    /// Return the driver's human-readable name (e.g. "PostgreSQL").
    fn name(&self) -> &'static str;

    /// Return the driver's capabilities.
    fn capabilities(&self) -> DriverCapability;

    /// Establish a new connection.
    async fn connect(&self, url: &str) -> AtlasResult<Box<dyn DriverConnection>>;

    /// Test whether a URL is reachable without fully connecting.
    async fn test_connection(&self, url: &str) -> AtlasResult<()> {
        let mut conn = self.connect(url).await?;
        conn.ping().await?;
        Ok(())
    }
}

/// Delegate `DriverConnection` through a `Box<dyn DriverConnection>`.
#[async_trait]
impl<T: DriverConnection + ?Sized> DriverConnection for Box<T> {
    async fn execute(&mut self, sql: &str) -> AtlasResult<QueryResult> {
        (**self).execute(sql).await
    }

    async fn execute_streaming(
        &mut self,
        sql: &str,
    ) -> AtlasResult<Box<dyn RowStream>> {
        (**self).execute_streaming(sql).await
    }

    async fn cancel(&mut self) -> AtlasResult<()> {
        (**self).cancel().await
    }

    async fn ping(&mut self) -> AtlasResult<()> {
        (**self).ping().await
    }

    async fn info(&self) -> AtlasResult<ConnectionInfo> {
        (**self).info().await
    }

    async fn begin(&mut self) -> AtlasResult<()> {
        (**self).begin().await
    }

    async fn commit(&mut self) -> AtlasResult<()> {
        (**self).commit().await
    }

    async fn rollback(&mut self) -> AtlasResult<()> {
        (**self).rollback().await
    }

    async fn close(&mut self) -> AtlasResult<()> {
        (**self).close().await
    }
}

/// A live connection to a specific database.
///
/// Each call to [`DatabaseDriver::connect`] returns a new implementation.
/// Connections are not required to be `Clone` — use a pool for sharing.
#[async_trait]
pub trait DriverConnection: Send + Sync {
    /// Execute a query and return the full result set.
    async fn execute(&mut self, sql: &str) -> AtlasResult<QueryResult>;

    /// Execute a query and stream rows one at a time.
    ///
    /// The default implementation calls `execute` and yields all rows.
    async fn execute_streaming(
        &mut self,
        sql: &str,
    ) -> AtlasResult<Box<dyn RowStream>> {
        let result = self.execute(sql).await?;
        Ok(Box::new(BufferedStream {
            columns: result.columns,
            rows: result.rows,
            pos: 0,
        }))
    }

    /// Cancel the currently-running query (if the driver supports it).
    async fn cancel(&mut self) -> AtlasResult<()> {
        Ok(())
    }

    /// Ping the server to verify the connection is alive.
    async fn ping(&mut self) -> AtlasResult<()>;

    /// Get connection metadata.
    async fn info(&self) -> AtlasResult<ConnectionInfo>;

    /// Begin a transaction.
    async fn begin(&mut self) -> AtlasResult<()>;

    /// Commit the current transaction.
    async fn commit(&mut self) -> AtlasResult<()>;

    /// Roll back the current transaction.
    async fn rollback(&mut self) -> AtlasResult<()>;

    /// Close the connection gracefully.
    async fn close(&mut self) -> AtlasResult<()>;
}

// ---- Row streaming -------------------------------------------------------

/// A stream of rows from an executing query.
#[async_trait]
pub trait RowStream: Send {
    /// Column metadata (available immediately).
    fn columns(&self) -> &[ColumnInfo];

    /// Fetch the next row, or `None` if the stream is exhausted.
    async fn next(&mut self) -> AtlasResult<Option<ResultRow>>;

    /// Cancel the streaming query.
    async fn cancel(&mut self) -> AtlasResult<()> {
        Ok(())
    }
}

/// A simple [`RowStream`] that drains a pre-buffered result.
struct BufferedStream {
    columns: Vec<ColumnInfo>,
    rows: Vec<ResultRow>,
    pos: usize,
}

#[async_trait]
impl RowStream for BufferedStream {
    fn columns(&self) -> &[ColumnInfo] {
        &self.columns
    }

    async fn next(&mut self) -> AtlasResult<Option<ResultRow>> {
        if self.pos >= self.rows.len() {
            return Ok(None);
        }
        let row = self.rows[self.pos].clone();
        self.pos += 1;
        Ok(Some(row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_display() {
        assert_eq!(Value::Null.to_string(), "NULL");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::String("hello".into()).to_string(), "hello");
    }

    #[test]
    fn result_row_by_index() {
        let cols = vec![ColumnInfo {
            name: "id".into(),
            col_type: ColumnType::Integer,
            db_type: "INTEGER".into(),
            nullable: false,
            ordinal: 0,
            size_hint: None,
        }];
        let row = ResultRow::new(cols, vec![Value::Int(1)]);
        assert_eq!(row.get(0).unwrap(), &Value::Int(1));
        assert_eq!(row.get_by_name("id").unwrap(), &Value::Int(1));
        assert!(row.get_by_name("missing").is_none());
    }

    #[test]
    fn result_row_iter() {
        let cols = vec![
            ColumnInfo {
                name: "a".into(),
                col_type: ColumnType::Integer,
                db_type: "INT".into(),
                nullable: false,
                ordinal: 0,
                size_hint: None,
            },
            ColumnInfo {
                name: "b".into(),
                col_type: ColumnType::String,
                db_type: "TEXT".into(),
                nullable: true,
                ordinal: 1,
                size_hint: None,
            },
        ];
        let row = ResultRow::new(cols, vec![Value::Int(1), Value::String("x".into())]);
        let pairs: Vec<_> = row.iter().map(|(c, v)| (c.name.clone(), v.clone())).collect();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn column_type_display() {
        assert_eq!(ColumnType::Boolean.to_string(), "boolean");
        assert_eq!(ColumnType::Other("geometry".into()).to_string(), "geometry");
    }

    #[test]
    fn query_result_properties() {
        let r = QueryResult {
            columns: vec![],
            rows: vec![],
            rows_affected: 0,
            elapsed_us: 100,
            warnings: vec![],
        };
        assert!(!r.has_rows());
        assert!(!r.is_mutation());
    }
}
