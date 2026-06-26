//! Database driver trait — the core abstraction for database connectivity.

use async_trait::async_trait;
use crate::cancel::CancellationToken;
use crate::error::CoreResult;
use crate::result::QueryResult;
use crate::types::connection::{ConnectionInfo, DatabaseKind, DriverCapabilities};
use crate::types::database::{
    ColumnInfo, DatabaseObject, ForeignKeyInfo, IndexInfo, ProcedureInfo, SchemaInfo,
    TableInfo, ViewInfo,
};
use crate::types::query::{ExecutionPlan, ExplainFormat, Pagination, QueryParams};

/// The primary trait that all database drivers must implement.
///
/// Each driver is responsible for connecting to a specific database kind,
/// executing queries, and introspecting schema objects.
#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    /// Return the database kind this driver handles.
    fn kind(&self) -> DatabaseKind;

    /// Return the driver name.
    fn name(&self) -> &str;

    /// Return the driver version.
    fn version(&self) -> &str;

    /// Return the capabilities this driver supports.
    fn capabilities(&self) -> DriverCapabilities;

    /// Open a new connection to the database.
    async fn connect(
        &self,
        info: &ConnectionInfo,
        cancel: CancellationToken,
    ) -> CoreResult<Box<dyn Connection>>;

    /// Validate that a connection configuration is correct without connecting.
    async fn validate_config(&self, info: &ConnectionInfo) -> CoreResult<()>;

    /// Test if a server is reachable.
    async fn ping(&self, info: &ConnectionInfo) -> CoreResult<u64>;

    /// Get a list of databases on the server.
    async fn list_databases(
        &self,
        conn: &dyn Connection,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<String>>;

    /// Introspect the database schema.
    async fn introspect(
        &self,
        conn: &dyn Connection,
        cancel: CancellationToken,
    ) -> CoreResult<IntrospectionResult>;
}

/// A live connection to a database.
#[async_trait]
pub trait Connection: Send + Sync {
    /// Execute a query and return results.
    async fn execute(
        &self,
        sql: &str,
        params: Option<QueryParams>,
        pagination: Option<Pagination>,
        cancel: CancellationToken,
    ) -> CoreResult<QueryResult>;

    /// Execute a query and stream results row by row.
    async fn execute_streaming(
        &self,
        sql: &str,
        params: Option<QueryParams>,
        cancel: CancellationToken,
    ) -> CoreResult<Box<dyn RowStream>>;

    /// Execute a statement that doesn't return rows (INSERT, UPDATE, DELETE, DDL).
    async fn execute_ddl(&self, sql: &str, cancel: CancellationToken) -> CoreResult<QueryResult>;

    /// Execute multiple statements in a single request.
    async fn execute_batch(
        &self,
        statements: &[String],
        cancel: CancellationToken,
    ) -> CoreResult<Vec<QueryResult>>;

    /// Get the EXPLAIN plan for a query.
    async fn explain(
        &self,
        sql: &str,
        format: ExplainFormat,
        analyze: bool,
        cancel: CancellationToken,
    ) -> CoreResult<ExecutionPlan>;

    /// Cancel the currently running query.
    async fn cancel_running(&self) -> CoreResult<()>;

    /// Get a list of schemas in the current database.
    async fn list_schemas(&self, cancel: CancellationToken) -> CoreResult<Vec<SchemaInfo>>;

    /// Get a list of tables in a schema.
    async fn list_tables(
        &self,
        schema: Option<&str>,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<TableInfo>>;

    /// Get detailed column information for a table.
    async fn describe_table(
        &self,
        schema: Option<&str>,
        table: &str,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<ColumnInfo>>;

    /// Get indexes for a table.
    async fn list_indexes(
        &self,
        schema: Option<&str>,
        table: &str,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<IndexInfo>>;

    /// Get foreign keys for a table.
    async fn list_foreign_keys(
        &self,
        schema: Option<&str>,
        table: &str,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<ForeignKeyInfo>>;

    /// Get stored procedures and functions.
    async fn list_procedures(
        &self,
        schema: Option<&str>,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<ProcedureInfo>>;

    /// Get views.
    async fn list_views(
        &self,
        schema: Option<&str>,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<ViewInfo>>;

    /// Get all objects of a given kind.
    async fn list_objects(
        &self,
        schema: Option<&str>,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<DatabaseObject>>;

    /// Check if the connection is still alive.
    async fn is_alive(&self) -> bool;

    /// Close the connection.
    async fn close(&self) -> CoreResult<()>;

    /// Get the connection info for this connection.
    fn info(&self) -> &ConnectionInfo;

    /// Get the current database name.
    fn current_database(&self) -> Option<&str>;

    /// Get the current schema name.
    fn current_schema(&self) -> Option<&str>;

    /// Set the current schema.
    async fn set_schema(&self, schema: &str, cancel: CancellationToken) -> CoreResult<()>;

    /// Begin a transaction.
    async fn begin_transaction(&self, cancel: CancellationToken) -> CoreResult<()>;

    /// Commit the current transaction.
    async fn commit(&self, cancel: CancellationToken) -> CoreResult<()>;

    /// Rollback the current transaction.
    async fn rollback(&self, cancel: CancellationToken) -> CoreResult<()>;
}

/// A streaming iterator over result rows.
#[async_trait]
pub trait RowStream: Send + Sync {
    /// Get the column metadata.
    fn columns(&self) -> &[crate::result::ColumnMetadata];

    /// Fetch the next batch of rows. Returns `None` when the stream is exhausted.
    async fn next_batch(&mut self) -> CoreResult<Option<Vec<crate::result::DataRow>>>;

    /// Cancel the underlying query.
    async fn cancel(&self) -> CoreResult<()>;
}

/// The result of a full schema introspection.
#[derive(Clone, Debug)]
pub struct IntrospectionResult {
    /// Database name.
    pub database: String,
    /// All schemas found.
    pub schemas: Vec<SchemaInfo>,
    /// All tables found.
    pub tables: Vec<TableInfo>,
    /// All views found.
    pub views: Vec<ViewInfo>,
    /// All procedures found.
    pub procedures: Vec<ProcedureInfo>,
    /// Driver version used.
    pub driver_version: String,
    /// Introspection duration in milliseconds.
    pub elapsed_ms: u64,
}
