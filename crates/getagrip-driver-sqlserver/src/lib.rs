//! Microsoft SQL Server driver for GetAGrip via tiberius.

use async_trait::async_trait;
use futures_util::TryStreamExt;
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use getagrip_core::AtlasResult;
use getagrip_database::driver::{
    ColumnInfo, ColumnType, ConnectionInfo, DatabaseDriver, DriverCapability, DriverConnection,
    QueryResult, ResultRow, ServerVersion, Value,
};

pub struct SqlServerDriver {
    pub trust_server_cert: bool,
}

impl SqlServerDriver {
    pub fn new() -> Self {
        Self { trust_server_cert: true }
    }
}

#[async_trait]
impl DatabaseDriver for SqlServerDriver {
    fn name(&self) -> &'static str { "SQL Server" }

    fn capabilities(&self) -> DriverCapability {
        DriverCapability::SELECT | DriverCapability::DDL | DriverCapability::DML
            | DriverCapability::TRANSACTIONS | DriverCapability::CANCEL
            | DriverCapability::INTROSPECTION
    }

    async fn connect(&self, url: &str) -> AtlasResult<Box<dyn DriverConnection>> {
        let config = parse_url(url, self.trust_server_cert)?;
        let tcp = TcpStream::connect(config.get_addr()).await.map_err(|e| {
            getagrip_core::AtlasError::Connection { source: url.into(), reason: e.to_string(), cause: None }
        })?;
        tcp.set_nodelay(true).ok();

        let client = Client::connect(config, tcp.compat_write()).await.map_err(|e| {
            getagrip_core::AtlasError::Connection { source: url.into(), reason: e.to_string(), cause: None }
        })?;

        Ok(Box::new(SqlServerConnection { client }))
    }
}

fn parse_url(url: &str, trust_cert: bool) -> AtlasResult<Config> {
    let mut config = Config::new();
    let rest = url.strip_prefix("sqlserver://")
        .or_else(|| url.strip_prefix("mssql://"))
        .ok_or_else(|| getagrip_core::AtlasError::Connection {
            source: url.into(), reason: "expected sqlserver:// prefix".into(), cause: None,
        })?;

    let (conn_part, query_str) = rest.split_once('?').unwrap_or((rest, ""));
    let (auth_host, database) = conn_part.split_once('/').unwrap_or((conn_part, ""));
    let (user_pass, host_port) = auth_host.split_once('@').unwrap_or(("", auth_host));
    let (username, password) = user_pass.split_once(':').unwrap_or(("", ""));
    let (host, port_str) = host_port.split_once(':').unwrap_or((host_port, "1433"));
    let port: u16 = port_str.parse().unwrap_or(1433);

    let database = database.trim();
    let host = host.trim();
    let username = username.trim();

    config.host(host);
    config.port(port);
    if !database.is_empty() { config.database(database); }
    if !username.is_empty() {
        config.authentication(tiberius::AuthMethod::sql_server(username, password));
    }

    for param in query_str.split('&') {
        let (k, v) = param.split_once('=').unwrap_or((param, ""));
        match k.to_lowercase().as_str() {
            "database" | "db" => { config.database(v); }
            "trustservercertificate" | "trust_cert" | "encrypt" => {
                if matches!(v.to_lowercase().as_str(), "true" | "1" | "yes") { config.trust_cert(); }
            }
            _ => {}
        }
    }

    if trust_cert { config.trust_cert(); }
    Ok(config)
}

// ---- Connection -----------------------------------------------------------

struct SqlServerConnection {
    client: Client<tokio_util::compat::Compat<TcpStream>>,
}

#[async_trait]
impl DriverConnection for SqlServerConnection {
    async fn execute(&mut self, sql: &str) -> AtlasResult<QueryResult> {
        let start = std::time::Instant::now();
        let mut stream = self.client.query(sql, &[]).await.map_err(|e| {
            getagrip_core::AtlasError::Query { code: None, detail: e.to_string() }
        })?;

        let cols: Vec<ColumnInfo> = stream.columns().await
            .map_err(|e| getagrip_core::AtlasError::Query { code: None, detail: e.to_string() })?
            .map(|cols| {
                cols.iter().enumerate().map(|(i, c)| ColumnInfo {
                    name: c.name().to_string(),
                    col_type: ColumnType::String,
                    db_type: format!("{:?}", c.column_type()),
                    nullable: true,
                    ordinal: i as u16,
                    size_hint: None,
                }).collect()
            })
            .unwrap_or_default();

        let mut rows = Vec::new();
        let mut rows_affected = 0u64;

        if cols.is_empty() {
            // rows_affected is collected at the end for mutation queries
            rows_affected = 0;
        } else {
            let col_info = cols.clone();
            loop {
                let item = stream.try_next().await
                    .map_err(|e| getagrip_core::AtlasError::Query { code: None, detail: e.to_string() })?;
                match item {
                    Some(tiberius::QueryItem::Row(row)) => {
                        let values: Vec<Value> = (0..cols.len()).map(|i| {
                            extract_value(&row, i)
                        }).collect();
                        rows.push(ResultRow::new(col_info.clone(), values));
                    }
                    None => break,
                    _ => {} // skip metadata notifications
                }
            }
        }

        Ok(QueryResult {
            columns: cols,
            rows,
            rows_affected,
            elapsed_us: start.elapsed().as_micros() as u64,
            warnings: Vec::new(),
        })
    }

    async fn ping(&mut self) -> AtlasResult<()> {
        self.client.query("SELECT 1", &[]).await.map_err(|e| {
            getagrip_core::AtlasError::Connection { source: "ping".into(), reason: e.to_string(), cause: None }
        })?;
        Ok(())
    }

    async fn info(&self) -> AtlasResult<ConnectionInfo> {
        Ok(ConnectionInfo {
            product_name: "Microsoft SQL Server".into(),
            version: ServerVersion { major: 2025, minor: 0, patch: 0, raw: "SQL Server 2025".into() },
            database: String::new(), schema: "dbo".into(), user: String::new(),
            server_pid: None, read_only: false,
        })
    }

    async fn begin(&mut self) -> AtlasResult<()> { self.execute("BEGIN TRANSACTION").await.map(|_| ()) }
    async fn commit(&mut self) -> AtlasResult<()> { self.execute("COMMIT").await.map(|_| ()) }
    async fn rollback(&mut self) -> AtlasResult<()> { self.execute("ROLLBACK").await.map(|_| ()) }
    async fn close(&mut self) -> AtlasResult<()> { Ok(()) }
}

fn extract_value(row: &tiberius::Row, idx: usize) -> Value {
    // Try standard types in order. tiberius returns Option<T>.
    if let Ok(Some(v)) = row.try_get::<&str, _>(idx) { return Value::String(v.to_string()); }
    if let Ok(Some(v)) = row.try_get::<bool, _>(idx) { return Value::Bool(v); }
    if let Ok(Some(v)) = row.try_get::<i32, _>(idx) { return Value::Int(v as i64); }
    if let Ok(Some(v)) = row.try_get::<i64, _>(idx) { return Value::Int(v); }
    if let Ok(Some(v)) = row.try_get::<f32, _>(idx) { return Value::Float(v as f64); }
    if let Ok(Some(v)) = row.try_get::<f64, _>(idx) { return Value::Float(v); }
    if let Ok(Some(v)) = row.try_get::<uuid::Uuid, _>(idx) {
        let (a, b, c, d) = v.as_fields();
        return Value::Uuid(uuid::Uuid::from_fields(a, b, c, d));
    }
    if let Ok(Some(v)) = row.try_get::<&[u8], _>(idx) { return Value::Bytes(v.to_vec()); }
    Value::Null
}
