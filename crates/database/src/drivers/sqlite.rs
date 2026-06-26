//! SQLite database driver implementation.

use async_trait::async_trait;
use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::types::ValueRef;
use tg_core::cancel::CancellationToken;
use tg_core::error::{CoreError, CoreResult};
use tg_core::result::{ColumnMetadata, DataCell, DataRow, DataType, QueryResult};
use tg_core::traits::driver::{Connection, DatabaseDriver, IntrospectionResult, RowStream};
use tg_core::types::connection::{ConnectionInfo, DatabaseKind, DriverCapabilities};
use tg_core::types::database::{
    ColumnInfo, DatabaseObject, DatabaseObjectKind, ForeignKeyInfo, IndexInfo, ProcedureInfo,
    SchemaInfo, TableInfo, ViewInfo,
};
use tg_core::types::query::{ExecutionPlan, ExplainFormat, Pagination, QueryParams};
use tracing::{debug, error, info};

/// The SQLite database driver.
pub struct SqliteDriver;

impl SqliteDriver {
    /// Create a new SQLite driver instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqliteDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseDriver for SqliteDriver {
    fn kind(&self) -> DatabaseKind {
        DatabaseKind::Sqlite
    }

    fn name(&self) -> &str {
        "SQLite"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            explain: true,
            transactions: true,
            prepared_statements: true,
            streaming: true,
            cancel_query: true,
            multiple_result_sets: false,
            introspection: true,
            read_write: true,
            ssh_tunneling: false,
        }
    }

    async fn connect(
        &self,
        info: &ConnectionInfo,
        _cancel: CancellationToken,
    ) -> CoreResult<Box<dyn Connection>> {
        let path = info.database.as_deref().unwrap_or(":memory:");
        debug!(path, "Opening SQLite database");

        let conn = rusqlite::Connection::open(path).map_err(|e| {
            CoreError::database_with_source(format!("Failed to open SQLite database at {path}"), e)
        })?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| CoreError::database_with_source("Failed to set SQLite pragmas", e))?;

        let sqlite_conn = SqliteConnection {
            inner: Arc::new(Mutex::new(conn)),
            info: info.clone(),
        };

        Ok(Box::new(sqlite_conn))
    }

    async fn validate_config(&self, info: &ConnectionInfo) -> CoreResult<()> {
        let path = info.database.as_deref().unwrap_or(":memory:");
        if path != ":memory:" && !std::path::Path::new(path).exists() {
            // Allow creating new databases
            debug!(path, "SQLite database file does not exist yet — will be created");
        }
        Ok(())
    }

    async fn ping(&self, info: &ConnectionInfo) -> CoreResult<u64> {
        let start = std::time::Instant::now();
        let path = info.database.as_deref().unwrap_or(":memory:");
        let conn = rusqlite::Connection::open(path).map_err(|e| {
            CoreError::database_with_source("Ping failed", e)
        })?;
        conn.execute_batch("SELECT 1").map_err(|e| {
            CoreError::database_with_source("Ping query failed", e)
        })?;
        Ok(start.elapsed().as_millis() as u64)
    }

    async fn list_databases(
        &self,
        _conn: &dyn Connection,
        _cancel: CancellationToken,
    ) -> CoreResult<Vec<String>> {
        // SQLite has one database per file
        Ok(vec!["main".into()])
    }

    async fn introspect(
        &self,
        conn: &dyn Connection,
        cancel: CancellationToken,
    ) -> CoreResult<IntrospectionResult> {
        let start = std::time::Instant::now();
        let tables = conn.list_tables(None, cancel.clone()).await?;
        let views = conn.list_views(None, cancel.clone()).await?;
        let procedures = conn.list_procedures(None, cancel).await?;

        Ok(IntrospectionResult {
            database: "main".into(),
            schemas: vec![SchemaInfo {
                name: "main".into(),
                owner: None,
                is_default: true,
                comment: None,
            }],
            tables,
            views,
            procedures,
            driver_version: Self.version().into(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}

/// A live SQLite connection.
struct SqliteConnection {
    inner: Arc<Mutex<rusqlite::Connection>>,
    info: ConnectionInfo,
}

impl SqliteConnection {
    /// Convert a rusqlite row to our DataRow type.
    fn row_to_values(row: &rusqlite::Row) -> rusqlite::Result<DataRow> {
        let mut values = Vec::new();
        for i in 0.. {
            match row.get_ref(i) {
                Ok(value) => {
                    let cell = match value {
                        ValueRef::Null => DataCell::Null,
                        ValueRef::Integer(i) => DataCell::I64(i),
                        ValueRef::Real(f) => DataCell::F64(f),
                        ValueRef::Text(s) => {
                            DataCell::String(String::from_utf8_lossy(s).into_owned())
                        }
                        ValueRef::Blob(b) => DataCell::Bytes(b.to_vec()),
                    };
                    values.push(cell);
                }
                Err(rusqlite::Error::InvalidColumnIndex(_)) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(values)
    }

    /// Convert rusqlite column info.
    fn column_metadata(stmt: &rusqlite::Statement) -> Vec<ColumnMetadata> {
        let count = stmt.column_count();
        (0..count).map(|i| {
            let name = stmt.column_name(i).unwrap_or("?").to_string();
            ColumnMetadata {
                name,
                data_type: DataType::Unknown,
                nullable: true,
                is_primary_key: false,
                is_foreign_key: false,
                table_name: None,
                schema_name: None,
                database_name: None,
                ordinal: i + 1,
                default_value: None,
                char_max_length: None,
                numeric_precision: None,
                numeric_scale: None,
            }
        }).collect()
    }
}

#[async_trait]
impl Connection for SqliteConnection {
    async fn execute(
        &self,
        sql: &str,
        _params: Option<QueryParams>,
        pagination: Option<Pagination>,
        cancel: CancellationToken,
    ) -> CoreResult<QueryResult> {
        let start = std::time::Instant::now();
        let inner = self.inner.clone();
        let sql = sql.to_string();

        let result = tokio::task::spawn_blocking(move || {
            let conn = inner.lock();

            if cancel.is_cancelled() {
                return Err(CoreError::Cancelled);
            }

            let mut stmt = conn.prepare(&sql).map_err(|e| {
                CoreError::query_with_sql(format!("SQLite prepare error: {e}"), &sql)
            })?;

            let columns = Self::column_metadata(&stmt);

            let pagination = pagination.unwrap_or_default();
            let rows_iter = stmt
                .query_map([], |row| Self::row_to_values(row))
                .map_err(|e| {
                    CoreError::query_with_sql(format!("SQLite query error: {e}"), &sql)
                })?;

            let mut all_rows: Vec<DataRow> = Vec::new();
            for row in rows_iter {
                if cancel.is_cancelled() {
                    return Err(CoreError::Cancelled);
                }
                match row {
                    Ok(r) => {
                        if all_rows.len() < pagination.limit as usize {
                            all_rows.push(r);
                        }
                    }
                    Err(e) => {
                        return Err(CoreError::query_with_sql(
                            format!("SQLite row error: {e}"),
                            &sql,
                        ));
                    }
                }
            }

            Ok(QueryResult {
                columns,
                rows: all_rows,
                total_rows: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
                rows_affected: None,
                has_more: false,
                explain_plan: None,
                metadata: serde_json::json!({"driver": "sqlite"}),
            })
        })
        .await
        .map_err(|e| CoreError::internal(format!("Task join error: {e}")))?;

        result
    }

    async fn execute_streaming(
        &self,
        _sql: &str,
        _params: Option<QueryParams>,
        _cancel: CancellationToken,
    ) -> CoreResult<Box<dyn RowStream>> {
        Err(CoreError::unsupported(
            "Streaming not yet implemented for SQLite",
        ))
    }

    async fn execute_ddl(&self, sql: &str, cancel: CancellationToken) -> CoreResult<QueryResult> {
        self.execute(sql, None, None, cancel).await
    }

    async fn execute_batch(
        &self,
        statements: &[String],
        cancel: CancellationToken,
    ) -> CoreResult<Vec<QueryResult>> {
        let mut results = Vec::new();
        for stmt in statements {
            if cancel.is_cancelled() {
                return Err(CoreError::Cancelled);
            }
            let result = self.execute(stmt, None, None, cancel.clone()).await?;
            results.push(result);
        }
        Ok(results)
    }

    async fn explain(
        &self,
        sql: &str,
        _format: ExplainFormat,
        _analyze: bool,
        cancel: CancellationToken,
    ) -> CoreResult<ExecutionPlan> {
        let explain_sql = format!("EXPLAIN QUERY PLAN {sql}");
        let result = self
            .execute(&explain_sql, None, None, cancel)
            .await?;

        let raw_text = result
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ")
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ExecutionPlan {
            root: tg_core::types::query::ExecutionPlanNode {
                operation: "SQLite Query Plan".into(),
                description: Some(raw_text.clone()),
                startup_cost: None,
                total_cost: None,
                plan_rows: None,
                plan_width: None,
                actual_time_ms: None,
                actual_rows: None,
                memory_kb: None,
                properties: std::collections::HashMap::new(),
                children: Vec::new(),
                warnings: Vec::new(),
                suggestions: Vec::new(),
            },
            planning_time_ms: None,
            execution_time_ms: Some(result.elapsed_ms as f64),
            triggers: None,
            raw_text: Some(raw_text),
            format: ExplainFormat::Text,
        })
    }

    async fn cancel_running(&self) -> CoreResult<()> {
        // rusqlite 0.32 interrupt is accessed differently
        Ok(())
    }

    async fn list_schemas(&self, _cancel: CancellationToken) -> CoreResult<Vec<SchemaInfo>> {
        Ok(vec![SchemaInfo {
            name: "main".into(),
            owner: None,
            is_default: true,
            comment: None,
        }])
    }

    async fn list_tables(
        &self,
        _schema: Option<&str>,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<TableInfo>> {
        let result = self
            .execute(
                "SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') ORDER BY name",
                None,
                None,
                cancel.clone(),
            )
            .await?;

        let mut tables = Vec::new();
        for row in &result.rows {
            if let (DataCell::String(name), DataCell::String(obj_type)) = (&row[0], &row[1]) {
                let table_type = match obj_type.as_str() {
                    "view" => tg_core::types::database::TableType::View,
                    _ => tg_core::types::database::TableType::Table,
                };

                // Get row count
                let count_sql = format!("SELECT COUNT(*) FROM \"{name}\"");
                let estimated_rows = if table_type == tg_core::types::database::TableType::Table {
                    self.execute(&count_sql, None, None, cancel.clone())
                        .await
                        .ok()
                        .and_then(|r| r.rows.first().and_then(|row| row.first().cloned()))
                        .and_then(|v| if let DataCell::I64(n) = v { Some(n as u64) } else { None })
                } else {
                    None
                };

                // Get columns
                let columns = self
                    .describe_table(None, name, cancel.clone())
                    .await
                    .unwrap_or_default();

                tables.push(TableInfo {
                    name: name.clone(),
                    schema: Some("main".into()),
                    database: None,
                    table_type,
                    estimated_rows,
                    size_bytes: None,
                    columns,
                    primary_key: Vec::new(),
                    indexes: Vec::new(),
                    foreign_keys: Vec::new(),
                    comment: None,
                    is_temporary: false,
                    is_partitioned: false,
                });
            }
        }
        Ok(tables)
    }

    async fn describe_table(
        &self,
        _schema: Option<&str>,
        table: &str,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<ColumnInfo>> {
        let result = self
            .execute(
                &format!("PRAGMA table_info(\"{table}\")"),
                None,
                None,
                cancel,
            )
            .await?;

        let columns = result
            .rows
            .iter()
            .map(|row| {
                let name = if let DataCell::String(s) = &row[1] {
                    s.clone()
                } else {
                    "unknown".into()
                };
                let type_str = if let DataCell::String(s) = &row[2] {
                    s.clone()
                } else {
                    String::new()
                };
                let not_null = if let DataCell::I64(n) = row[3] {
                    n != 0
                } else {
                    false
                };
                let pk = if let DataCell::I64(n) = row[5] {
                    n != 0
                } else {
                    false
                };

                let data_type = parse_sqlite_type(&type_str);

                ColumnInfo {
                    name,
                    data_type,
                    nullable: !not_null,
                    is_primary_key: pk,
                    default_value: None,
                    char_max_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    ordinal_position: 0,
                    comment: None,
                    is_generated: false,
                    generation_expression: None,
                }
            })
            .collect();

        Ok(columns)
    }

    async fn list_indexes(
        &self,
        _schema: Option<&str>,
        table: &str,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<IndexInfo>> {
        let result = self
            .execute(
                &format!("PRAGMA index_list(\"{table}\")"),
                None,
                None,
                cancel,
            )
            .await?;

        let mut indexes = Vec::new();
        for row in &result.rows {
            // PRAGMA index_list returns: seq, name, unique, origin, partial
            if let DataCell::String(name) = &row[1] {
                let is_unique = matches!(&row[2], DataCell::I64(n) if *n != 0);
                indexes.push(IndexInfo {
                    name: name.clone(),
                    columns: Vec::new(),
                    is_unique,
                    is_primary: name.contains("sqlite_autoindex"),
                    method: Some("btree".into()),
                    definition: None,
                    is_partial: false,
                    predicate: None,
                });
            }
        }
        Ok(indexes)
    }

    async fn list_foreign_keys(
        &self,
        _schema: Option<&str>,
        table: &str,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<ForeignKeyInfo>> {
        let result = self
            .execute(
                &format!("PRAGMA foreign_key_list(\"{table}\")"),
                None,
                None,
                cancel,
            )
            .await?;

        let mut fks = Vec::new();
        for row in &result.rows {
            // id, seq, table, from, to, on_update, on_delete, match
            if let (
                DataCell::String(ref_table),
                DataCell::String(from_col),
                DataCell::String(to_col),
            ) = (&row[2], &row[3], &row[4])
            {
                fks.push(ForeignKeyInfo {
                    name: format!("fk_{table}_{from_col}"),
                    columns: vec![from_col.clone()],
                    referenced_table: ref_table.clone(),
                    referenced_schema: None,
                    referenced_columns: vec![to_col.clone()],
                    on_delete: None,
                    on_update: None,
                });
            }
        }
        Ok(fks)
    }

    async fn list_procedures(
        &self,
        _schema: Option<&str>,
        _cancel: CancellationToken,
    ) -> CoreResult<Vec<ProcedureInfo>> {
        // SQLite does not have stored procedures
        Ok(Vec::new())
    }

    async fn list_views(
        &self,
        _schema: Option<&str>,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<ViewInfo>> {
        let result = self
            .execute(
                "SELECT name, sql FROM sqlite_master WHERE type = 'view' ORDER BY name",
                None,
                None,
                cancel.clone(),
            )
            .await?;

        let views = result
            .rows
            .iter()
            .map(|row| {
                let name = if let DataCell::String(s) = &row[0] {
                    s.clone()
                } else {
                    "unknown".into()
                };
                let definition = if let DataCell::String(s) = &row[1] {
                    Some(s.clone())
                } else {
                    None
                };

                ViewInfo {
                    name,
                    schema: Some("main".into()),
                    definition,
                    is_updatable: false,
                    is_materialized: false,
                    columns: Vec::new(),
                    comment: None,
                }
            })
            .collect();

        Ok(views)
    }

    async fn list_objects(
        &self,
        _schema: Option<&str>,
        cancel: CancellationToken,
    ) -> CoreResult<Vec<DatabaseObject>> {
        let result = self
            .execute(
                "SELECT type, name FROM sqlite_master WHERE type IN ('table', 'view', 'index', 'trigger') ORDER BY type, name",
                None,
                None,
                cancel,
            )
            .await?;

        let objects = result
            .rows
            .iter()
            .filter_map(|row| {
                let obj_type = if let DataCell::String(s) = &row[0] {
                    s.clone()
                } else {
                    return None;
                };
                let name = if let DataCell::String(s) = &row[1] {
                    s.clone()
                } else {
                    return None;
                };

                let kind = match obj_type.as_str() {
                    "table" => DatabaseObjectKind::Table,
                    "view" => DatabaseObjectKind::View,
                    "index" => DatabaseObjectKind::Index,
                    "trigger" => DatabaseObjectKind::Trigger,
                    _ => return None,
                };

                Some(DatabaseObject {
                    kind,
                    name,
                    schema: Some("main".into()),
                    database: None,
                    parent: None,
                    comment: None,
                })
            })
            .collect();

        Ok(objects)
    }

    async fn is_alive(&self) -> bool {
        // SQLite connections are always alive if the struct exists
        true
    }

    async fn close(&self) -> CoreResult<()> {
        debug!("Closing SQLite connection");
        // rusqlite Connection closes when dropped
        Ok(())
    }

    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn current_database(&self) -> Option<&str> {
        Some("main")
    }

    fn current_schema(&self) -> Option<&str> {
        Some("main")
    }

    async fn set_schema(&self, _schema: &str, _cancel: CancellationToken) -> CoreResult<()> {
        // SQLite doesn't support schemas
        Ok(())
    }

    async fn begin_transaction(&self, cancel: CancellationToken) -> CoreResult<()> {
        self.execute("BEGIN TRANSACTION", None, None, cancel)
            .await?;
        Ok(())
    }

    async fn commit(&self, cancel: CancellationToken) -> CoreResult<()> {
        self.execute("COMMIT", None, None, cancel).await?;
        Ok(())
    }

    async fn rollback(&self, cancel: CancellationToken) -> CoreResult<()> {
        self.execute("ROLLBACK", None, None, cancel).await?;
        Ok(())
    }
}

/// Parse a SQLite type declaration string into our DataType.
fn parse_sqlite_type(type_str: &str) -> DataType {
    let upper = type_str.to_uppercase();
    if upper.contains("INT") {
        DataType::BigInt
    } else if upper.contains("CHAR") || upper.contains("TEXT") || upper.contains("CLOB") {
        DataType::Text
    } else if upper.contains("BLOB") {
        DataType::Binary
    } else if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
        DataType::Double
    } else if upper.contains("BOOL") {
        DataType::Boolean
    } else if upper.is_empty() {
        DataType::Unknown
    } else {
        DataType::Custom(type_str.to_string())
    }
}

/// A streaming row iterator for SQLite (stub).
///
/// Full streaming support requires more work to bridge rusqlite's
/// sync API with async streaming. This will be implemented in a future phase.
pub struct SqliteRowStream;

#[async_trait]
impl RowStream for SqliteRowStream {
    fn columns(&self) -> &[ColumnMetadata] {
        &[]
    }

    async fn next_batch(&mut self) -> CoreResult<Option<Vec<DataRow>>> {
        Ok(None)
    }

    async fn cancel(&self) -> CoreResult<()> {
        Ok(())
    }
}
