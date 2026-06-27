//! Shared helpers for Tauri command handlers.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use getagrip_core::session::{ConnectionDriver, ConnectionProfile, ConnectionProfiles};
use getagrip_database::DatabaseDriver;
use getagrip_database::driver::{ColumnInfo, QueryResult, Value};
use getagrip_driver_sqlserver::SqlServerDriver;

use crate::commands::query::{QueryColumnDto, QueryResultDto};

/// Build a [`SqlServerDriver`] with default options (trust cert = true).
pub fn driver() -> SqlServerDriver {
    SqlServerDriver::new()
}

/// Create the appropriate driver for a connection profile.
pub fn driver_for(profile: &ConnectionProfile) -> Result<Arc<dyn DatabaseDriver>, String> {
    match profile.driver {
        ConnectionDriver::Mssql => Ok(Arc::new(SqlServerDriver::new())),
        ConnectionDriver::Postgres => Err("PostgreSQL driver not yet implemented".into()),
        ConnectionDriver::Mysql => Err("MySQL driver not yet implemented".into()),
        ConnectionDriver::Sqlite => Err("SQLite driver not yet implemented".into()),
        ConnectionDriver::Oracle => Err("Oracle driver not yet implemented".into()),
        ConnectionDriver::MongoDB => Err("MongoDB driver not yet implemented".into()),
        ConnectionDriver::Redis => Err("Redis driver not yet implemented".into()),
        ConnectionDriver::Generic => Err("Generic driver not yet implemented".into()),
    }
}

/// Persist connection profiles to the datasources.json file.
pub fn persist_profiles(profiles: &ConnectionProfiles, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(profiles).map_err(|e| format!("serialize: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

/// Convert a [`QueryResult`] into a JSON-friendly DTO for the frontend.
///
/// We flatten `ResultRow` into `HashMap<String, Value>` so the JS side
/// doesn't need to know about our row-index caching.
pub fn query_result_to_dto(result: QueryResult, elapsed_ms: u64) -> QueryResultDto {
    let columns: Vec<ColumnInfo> = result.columns.clone();
    let rows: Vec<serde_json::Map<String, serde_json::Value>> = result
        .rows
        .into_iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                let val = row.get(i).cloned().unwrap_or(Value::Null);
                map.insert(col.name.clone(), value_to_json(&val));
            }
            map
        })
        .collect();

    QueryResultDto {
        columns: columns
            .into_iter()
            .map(|c| QueryColumnDto {
                name: c.name,
                col_type: c.col_type.to_string(),
                db_type: c.db_type,
                nullable: c.nullable,
                ordinal: c.ordinal,
            })
            .collect(),
        rows,
        elapsed_ms,
        rows_affected: result.rows_affected,
    }
}

/// Turn a database `Value` into a JSON value.
///
/// Null becomes `null`, strings/text become JSON strings, numbers become
/// JSON numbers, blobs become base64 strings.
pub fn value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => {
            use base64::Engine;
            serde_json::Value::String(
                base64::engine::general_purpose::STANDARD.encode(b),
            )
        }
        Value::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
        Value::Uuid(u) => serde_json::Value::String(u.to_string()),
        Value::Json(s) => serde_json::Value::String(s.clone()),
    }
}

