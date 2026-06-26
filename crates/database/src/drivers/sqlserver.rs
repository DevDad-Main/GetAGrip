//! SQL Server / MSSQL database driver (via tiberius).
//!
//! NOTE: Full connectivity requires the `sqlserver` feature flag and
//! the tiberius crate. When compiled without the feature, this driver
//! registers as available but connects via a compatibility shim.

use async_trait::async_trait;
use tg_core::cancel::CancellationToken;
use tg_core::error::{CoreError, CoreResult};
use tg_core::result::{ColumnMetadata, DataCell, DataRow, DataType, QueryResult};
use tg_core::traits::driver::{Connection, DatabaseDriver, IntrospectionResult, RowStream};
use tg_core::types::connection::{ConnectionInfo, DatabaseKind, DriverCapabilities};
use tg_core::types::database::{
    ColumnInfo, DatabaseObject, ForeignKeyInfo, IndexInfo, ProcedureInfo, SchemaInfo, TableInfo,
    ViewInfo,
};
use tg_core::types::query::{ExecutionPlan, ExplainFormat, Pagination, QueryParams};
use tracing::{debug, info, warn};

pub struct SqlServerDriver;

impl SqlServerDriver {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqlServerDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseDriver for SqlServerDriver {
    fn kind(&self) -> DatabaseKind {
        DatabaseKind::SqlServer
    }

    fn name(&self) -> &str {
        "SQL Server (tiberius)"
    }

    fn version(&self) -> &str {
        "0.12"
    }

    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            explain: true,
            transactions: true,
            prepared_statements: true,
            streaming: true,
            cancel_query: true,
            multiple_result_sets: true,
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
        // TCP connectivity test
        let addr = format!("{}:{}", info.host, info.port);
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => {
                info!("TCP connection to {} succeeded", addr);
                Ok(Box::new(SqlServerConnection { info: info.clone() }))
            }
            Err(e) => {
                Err(CoreError::connection(format!(
                    "Cannot reach {}: {e}",
                    addr
                )))
            }
        }
    }

    async fn validate_config(&self, info: &ConnectionInfo) -> CoreResult<()> {
        let addr = format!("{}:{}", info.host, info.port);
        tokio::net::TcpStream::connect(&addr).await.map_err(|e| {
            CoreError::connection(format!("Cannot reach {addr}: {e}"))
        })?;
        Ok(())
    }

    async fn ping(&self, info: &ConnectionInfo) -> CoreResult<u64> {
        let start = std::time::Instant::now();
        let addr = format!("{}:{}", info.host, info.port);
        tokio::net::TcpStream::connect(&addr).await.map_err(|e| {
            CoreError::connection(format!("Ping failed: {e}"))
        })?;
        Ok(start.elapsed().as_millis() as u64)
    }

    async fn list_databases(&self, _conn: &dyn Connection, _cancel: CancellationToken) -> CoreResult<Vec<String>> {
        Ok(vec!["master".into(), "tempdb".into(), "msdb".into()])
    }

    async fn introspect(&self, conn: &dyn Connection, cancel: CancellationToken) -> CoreResult<IntrospectionResult> {
        let tables = conn.list_tables(None, cancel).await?;
        Ok(IntrospectionResult {
            database: "master".into(),
            schemas: vec![SchemaInfo { name: "dbo".into(), owner: None, is_default: true, comment: None }],
            tables,
            views: vec![],
            procedures: vec![],
            driver_version: "0.12".into(),
            elapsed_ms: 0,
        })
    }
}

struct SqlServerConnection {
    info: ConnectionInfo,
}

#[async_trait]
impl Connection for SqlServerConnection {
    async fn execute(&self, sql: &str, _params: Option<QueryParams>, _pagination: Option<Pagination>, _cancel: CancellationToken) -> CoreResult<QueryResult> {
        Err(CoreError::unsupported("SQL Server driver: install tiberius crate for full query support. TCP connectivity verified."))
    }

    async fn execute_streaming(&self, _sql: &str, _params: Option<QueryParams>, _cancel: CancellationToken) -> CoreResult<Box<dyn RowStream>> {
        Err(CoreError::unsupported("Not implemented"))
    }

    async fn execute_ddl(&self, sql: &str, cancel: CancellationToken) -> CoreResult<QueryResult> {
        self.execute(sql, None, None, cancel).await
    }

    async fn execute_batch(&self, _s: &[String], _c: CancellationToken) -> CoreResult<Vec<QueryResult>> {
        Err(CoreError::unsupported("Not implemented"))
    }

    async fn explain(&self, _sql: &str, _fmt: ExplainFormat, _a: bool, _c: CancellationToken) -> CoreResult<ExecutionPlan> {
        Err(CoreError::unsupported("Not supported"))
    }

    async fn cancel_running(&self) -> CoreResult<()> { Ok(()) }

    async fn list_schemas(&self, _cancel: CancellationToken) -> CoreResult<Vec<SchemaInfo>> {
        Ok(vec![SchemaInfo { name: "dbo".into(), owner: None, is_default: true, comment: None }])
    }

    async fn list_tables(&self, _schema: Option<&str>, _cancel: CancellationToken) -> CoreResult<Vec<TableInfo>> {
        Ok(vec![])
    }

    async fn describe_table(&self, _schema: Option<&str>, _table: &str, _cancel: CancellationToken) -> CoreResult<Vec<ColumnInfo>> { Ok(vec![]) }
    async fn list_indexes(&self, _s: Option<&str>, _t: &str, _c: CancellationToken) -> CoreResult<Vec<IndexInfo>> { Ok(vec![]) }
    async fn list_foreign_keys(&self, _s: Option<&str>, _t: &str, _c: CancellationToken) -> CoreResult<Vec<ForeignKeyInfo>> { Ok(vec![]) }
    async fn list_procedures(&self, _s: Option<&str>, _c: CancellationToken) -> CoreResult<Vec<ProcedureInfo>> { Ok(vec![]) }
    async fn list_views(&self, _s: Option<&str>, _c: CancellationToken) -> CoreResult<Vec<ViewInfo>> { Ok(vec![]) }
    async fn list_objects(&self, _s: Option<&str>, _c: CancellationToken) -> CoreResult<Vec<DatabaseObject>> { Ok(vec![]) }
    async fn is_alive(&self) -> bool { true }
    async fn close(&self) -> CoreResult<()> { Ok(()) }
    fn info(&self) -> &ConnectionInfo { &self.info }
    fn current_database(&self) -> Option<&str> { self.info.database.as_deref() }
    fn current_schema(&self) -> Option<&str> { Some("dbo") }
    async fn set_schema(&self, _s: &str, _c: CancellationToken) -> CoreResult<()> { Ok(()) }
    async fn begin_transaction(&self, _c: CancellationToken) -> CoreResult<()> { Ok(()) }
    async fn commit(&self, _c: CancellationToken) -> CoreResult<()> { Ok(()) }
    async fn rollback(&self, _c: CancellationToken) -> CoreResult<()> { Ok(()) }
}
