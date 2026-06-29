use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::{Config, NoTls, Row};
use tokio_postgres::types::Type;
use tokio_postgres_rustls::MakeRustlsConnect;

use getagrip_core::AtlasResult;
use getagrip_database::driver::{
    ColumnInfo, ColumnType, ConnectionInfo, DatabaseDriver, DriverCapability, DriverConnection,
    QueryResult, ResultRow, ServerVersion, Value,
};

pub struct PostgresDriver;

impl PostgresDriver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DatabaseDriver for PostgresDriver {
    fn name(&self) -> &'static str { "PostgreSQL" }

    fn capabilities(&self) -> DriverCapability {
        DriverCapability::SELECT | DriverCapability::DDL | DriverCapability::DML
            | DriverCapability::TRANSACTIONS | DriverCapability::CANCEL
            | DriverCapability::INTROSPECTION
    }

    async fn connect(&self, url: &str) -> AtlasResult<Box<dyn DriverConnection>> {
        let config: Config = url.parse().map_err(|e: tokio_postgres::Error| {
            getagrip_core::AtlasError::Connection {
                source: url.into(), reason: format!("invalid postgres url: {e}"), cause: None,
            }
        })?;

        let use_tls = url.contains("sslmode=require")
            || url.contains("sslmode=prefer")
            || url.contains("sslmode=verify-ca")
            || url.contains("sslmode=verify-full");

        if use_tls {
            let roots = rustls::RootCertStore::from_iter(
                webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
            );
            let tls_config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            let tls = MakeRustlsConnect::new(tls_config);
            let (client, connection) = config.connect(tls).await.map_err(|e| {
                getagrip_core::AtlasError::Connection { source: url.into(), reason: e.to_string(), cause: None }
            })?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    tracing::error!("postgres tls connection error: {e}");
                }
            });
            let cancel_token = client.cancel_token();
            Ok(Box::new(PostgresConnection {
                client: Arc::new(Mutex::new(client)),
                cancel_token: Arc::new(Mutex::new(cancel_token)),
            }))
        } else {
            let (client, connection) = config.connect(NoTls).await.map_err(|e| {
                getagrip_core::AtlasError::Connection { source: url.into(), reason: e.to_string(), cause: None }
            })?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    tracing::error!("postgres connection error: {e}");
                }
            });
            let cancel_token = client.cancel_token();
            Ok(Box::new(PostgresConnection {
                client: Arc::new(Mutex::new(client)),
                cancel_token: Arc::new(Mutex::new(cancel_token)),
            }))
        }
    }
}

fn map_col_type(typ: &Type) -> ColumnType {
    match *typ {
        Type::BOOL => ColumnType::Boolean,
        Type::INT2 | Type::INT4 | Type::INT8
        | Type::INT2_ARRAY | Type::INT4_ARRAY | Type::INT8_ARRAY => ColumnType::Integer,
        Type::FLOAT4 | Type::FLOAT8 | Type::NUMERIC
        | Type::FLOAT4_ARRAY | Type::FLOAT8_ARRAY => ColumnType::Float,
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME
        | Type::TEXT_ARRAY | Type::VARCHAR_ARRAY
        | Type::JSON | Type::JSONB | Type::XML => ColumnType::String,
        Type::BYTEA => ColumnType::Binary,
        Type::DATE | Type::TIMESTAMP | Type::TIMESTAMPTZ | Type::TIME | Type::TIMETZ | Type::INTERVAL
        | Type::DATE_ARRAY | Type::TIMESTAMP_ARRAY | Type::TIMESTAMPTZ_ARRAY => ColumnType::DateTime,
        Type::UUID | Type::UUID_ARRAY => ColumnType::Uuid,
        _ if typ.name() == "json" || typ.name() == "jsonb" => ColumnType::Json,
        _ if typ.name().ends_with("[]") => ColumnType::String,
        _ => ColumnType::Other(typ.name().to_string()),
    }
}

struct PostgresConnection {
    client: Arc<Mutex<tokio_postgres::Client>>,
    cancel_token: Arc<Mutex<tokio_postgres::CancelToken>>,
}

#[async_trait]
impl DriverConnection for PostgresConnection {
    async fn execute(&mut self, sql: &str) -> AtlasResult<QueryResult> {
        let start = std::time::Instant::now();

        let stmt = {
            let client = self.client.lock().await;
            client.prepare(sql).await
        };

        let (_stmt, cols) = match stmt {
            Ok(s) => {
                let cols: Vec<ColumnInfo> = s.columns().iter().enumerate().map(|(i, c)| ColumnInfo {
                    name: c.name().to_string(),
                    col_type: map_col_type(c.type_()),
                    db_type: c.type_().name().to_string(),
                    nullable: true,
                    ordinal: i as u16,
                    size_hint: None,
                }).collect();
                (Some(s), cols)
            }
            Err(_) => {
                let client = self.client.lock().await;
                let rows_affected = client.execute(sql, &[]).await.map_err(|e| {
                    getagrip_core::AtlasError::Query { code: None, detail: e.to_string() }
                })?;
                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected,
                    elapsed_us: start.elapsed().as_micros() as u64,
                    warnings: vec![],
                });
            }
        };

        if cols.is_empty() {
            let client = self.client.lock().await;
            let rows_affected = client.execute(sql, &[]).await.map_err(|e| {
                getagrip_core::AtlasError::Query { code: None, detail: e.to_string() }
            })?;
            return Ok(QueryResult {
                columns: cols,
                rows: vec![],
                rows_affected,
                elapsed_us: start.elapsed().as_micros() as u64,
                warnings: vec![],
            });
        }

        let rows = {
            let client = self.client.lock().await;
            client.query(sql, &[]).await.map_err(|e| {
                getagrip_core::AtlasError::Query { code: None, detail: e.to_string() }
            })?
        };

        let col_info = cols.clone();
        let row_count = rows.len() as u64;
        let rows: Vec<ResultRow> = rows.iter().map(|row| {
            let values: Vec<Value> = (0..cols.len()).map(|i| extract_value(row, i)).collect();
            ResultRow::new(col_info.clone(), values)
        }).collect();

        Ok(QueryResult {
            columns: cols,
            rows,
            rows_affected: row_count,
            elapsed_us: start.elapsed().as_micros() as u64,
            warnings: vec![],
        })
    }

    async fn ping(&mut self) -> AtlasResult<()> {
        let client = self.client.lock().await;
        client.simple_query("SELECT 1").await.map_err(|e| {
            getagrip_core::AtlasError::Connection { source: "ping".into(), reason: e.to_string(), cause: None }
        })?;
        Ok(())
    }

    async fn cancel(&mut self) -> AtlasResult<()> {
        let token = self.cancel_token.lock().await;
        token.cancel_query(NoTls).await.map_err(|e| {
            getagrip_core::AtlasError::Query { code: None, detail: format!("cancel failed: {e}") }
        })?;
        Ok(())
    }

    async fn info(&self) -> AtlasResult<ConnectionInfo> {
        let version = "PostgreSQL".into();
        Ok(ConnectionInfo {
            product_name: "PostgreSQL".into(),
            version: ServerVersion { major: 0, minor: 0, patch: 0, raw: version },
            database: String::new(),
            schema: "public".into(),
            user: String::new(),
            server_pid: None,
            read_only: false,
        })
    }

    async fn begin(&mut self) -> AtlasResult<()> {
        let client = self.client.lock().await;
        client.simple_query("BEGIN").await.map_err(|e| {
            getagrip_core::AtlasError::Query { code: None, detail: e.to_string() }
        })?;
        Ok(())
    }

    async fn commit(&mut self) -> AtlasResult<()> {
        let client = self.client.lock().await;
        client.simple_query("COMMIT").await.map_err(|e| {
            getagrip_core::AtlasError::Query { code: None, detail: e.to_string() }
        })?;
        Ok(())
    }

    async fn rollback(&mut self) -> AtlasResult<()> {
        let client = self.client.lock().await;
        client.simple_query("ROLLBACK").await.map_err(|e| {
            getagrip_core::AtlasError::Query { code: None, detail: e.to_string() }
        })?;
        Ok(())
    }

    async fn close(&mut self) -> AtlasResult<()> { Ok(()) }
}

fn extract_value(row: &Row, idx: usize) -> Value {
    let col_type = row.columns()[idx].type_();

    match *col_type {
        Type::BOOL => row.try_get::<_, bool>(idx).ok().map(Value::Bool).unwrap_or(Value::Null),
        Type::INT2 => row.try_get::<_, i16>(idx).ok().map(|v| Value::Int(v as i64)).unwrap_or(Value::Null),
        Type::INT4 => row.try_get::<_, i32>(idx).ok().map(|v| Value::Int(v as i64)).unwrap_or(Value::Null),
        Type::INT8 => row.try_get::<_, i64>(idx).ok().map(Value::Int).unwrap_or(Value::Null),
        Type::FLOAT4 => row.try_get::<_, f32>(idx).ok().map(|v| Value::Float(v as f64)).unwrap_or(Value::Null),
        Type::FLOAT8 => row.try_get::<_, f64>(idx).ok().map(Value::Float).unwrap_or(Value::Null),
        Type::NUMERIC => {
            row.try_get::<_, &str>(idx).ok()
                .and_then(|s| s.parse::<f64>().ok())
                .map(Value::Float)
                .unwrap_or(Value::Null)
        }
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::JSON | Type::JSONB | Type::XML => {
            row.try_get::<_, &str>(idx).ok().map(|s| Value::String(s.to_string())).unwrap_or(Value::Null)
        }
        Type::BYTEA => {
            row.try_get::<_, &[u8]>(idx).ok().map(|b| Value::Bytes(b.to_vec())).unwrap_or(Value::Null)
        }
        Type::DATE | Type::TIMESTAMP | Type::TIMESTAMPTZ | Type::TIME | Type::TIMETZ => {
            row.try_get::<_, chrono::NaiveDateTime>(idx).ok()
                .map(|dt| Value::DateTime(dt.and_utc()))
                .or_else(|| row.try_get::<_, chrono::DateTime<chrono::Utc>>(idx).ok().map(Value::DateTime))
                .or_else(|| {
                    row.try_get::<_, chrono::NaiveDate>(idx).ok()
                        .map(|d| Value::DateTime(d.and_hms_opt(0, 0, 0).unwrap().and_utc()))
                })
                .unwrap_or(Value::Null)
        }
        Type::UUID => {
            row.try_get::<_, uuid::Uuid>(idx).ok().map(Value::Uuid).unwrap_or(Value::Null)
        }
        Type::INT2_ARRAY => {
            row.try_get::<_, Vec<i16>>(idx).ok().map(|v| Value::String(format!("{:?}", v))).unwrap_or(Value::Null)
        }
        Type::INT4_ARRAY => {
            row.try_get::<_, Vec<i32>>(idx).ok().map(|v| Value::String(format!("{:?}", v))).unwrap_or(Value::Null)
        }
        Type::INT8_ARRAY => {
            row.try_get::<_, Vec<i64>>(idx).ok().map(|v| Value::String(format!("{:?}", v))).unwrap_or(Value::Null)
        }
        Type::FLOAT4_ARRAY => {
            row.try_get::<_, Vec<f32>>(idx).ok().map(|v| Value::String(format!("{:?}", v))).unwrap_or(Value::Null)
        }
        Type::FLOAT8_ARRAY => {
            row.try_get::<_, Vec<f64>>(idx).ok().map(|v| Value::String(format!("{:?}", v))).unwrap_or(Value::Null)
        }
        Type::TEXT_ARRAY | Type::VARCHAR_ARRAY => {
            row.try_get::<_, Vec<&str>>(idx).ok()
                .map(|v| Value::String(format!("{:?}", v)))
                .unwrap_or(Value::Null)
        }
        Type::DATE_ARRAY | Type::TIMESTAMP_ARRAY | Type::TIMESTAMPTZ_ARRAY => {
            row.try_get::<_, Vec<chrono::NaiveDateTime>>(idx).ok()
                .map(|v| Value::String(format!("{:?}", v)))
                .unwrap_or(Value::Null)
        }
        Type::UUID_ARRAY => {
            row.try_get::<_, Vec<uuid::Uuid>>(idx).ok()
                .map(|v| Value::String(format!("{:?}", v)))
                .unwrap_or(Value::Null)
        }
        _ => {
            row.try_get::<_, &str>(idx).ok().map(|s| Value::String(s.to_string())).unwrap_or(Value::Null)
        }
    }
}
