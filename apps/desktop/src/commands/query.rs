//! Query execution commands.

use std::time::Instant;

use serde::Serialize;

use getagrip_database::DatabaseDriver;

use crate::commands::util::{driver, query_result_to_dto};

/// JSON-friendly mirror of [`getagrip_database::driver::ColumnInfo`].
#[derive(Debug, Clone, Serialize)]
pub struct QueryColumnDto {
    pub name: String,
    pub col_type: String,
    pub db_type: String,
    pub nullable: bool,
    pub ordinal: u16,
}

/// JSON-friendly query result sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct QueryResultDto {
    pub columns: Vec<QueryColumnDto>,
    /// One row per HashMap. Keys are column names.
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    pub elapsed_ms: u64,
    pub rows_affected: u64,
}

/// Execute a SQL statement against a connection URL.
///
/// Phase 1: one-shot connect → execute → return. No streaming, no
/// cancellation. The frontend caps displayed rows at 5000.
#[tauri::command]
pub async fn execute_query(sql: String, url: String) -> Result<QueryResultDto, String> {
    if sql.trim().is_empty() {
        return Err("empty query".into());
    }

    let start = Instant::now();
    let d = driver();
    let mut conn = d.connect(&url).await.map_err(|e| format!("{e}"))?;

    let result = conn.execute(&sql).await.map_err(|e| format!("{e}"))?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(query_result_to_dto(result, elapsed_ms))
}
