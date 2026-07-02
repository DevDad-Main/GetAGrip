//! Query execution commands — standard + streaming.

use std::time::Instant;

use chrono::Utc;
use serde::Serialize;
use tauri::{Emitter, State};

use getagrip_core::id::Id;
use getagrip_core::session::ConnectionProfileId;
use getagrip_query_engine::scheduler::{QueryId, QueryStatus, TabId};
use getagrip_query_engine::HistoryEntry;

use getagrip_database::driver::{ColumnInfo, Value};

use crate::commands::util::{persist_history, query_result_to_dto, value_to_json};
use crate::state::AppState;

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
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    pub elapsed_ms: u64,
    pub rows_affected: u64,
}

/// Streaming query summary (returned after all events emitted).
#[derive(Debug, Clone, Serialize)]
pub struct StreamSummary {
    pub total_rows: usize,
    pub elapsed_ms: u64,
}

/// Execute a SQL statement using a connected profile (standard — returns all at once).
#[tauri::command]
pub async fn execute_query(
    profile_id: String,
    sql: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResultDto>, String> {
    if sql.trim().is_empty() {
        return Err("empty query".into());
    }

    let profile_id_typed = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let managed = state
        .manager
        .get(profile_id_typed)
        .ok_or_else(|| format!("not connected: {profile_id}"))?;

    let pool = managed
        .pool
        .ok_or_else(|| format!("no pool for profile: {profile_id}"))?;

    let query_id = Id::<QueryId>::new_v7();
    let tab_id_typed = Id::<TabId>::parse(&tab_id).unwrap_or_else(Id::new);

    let start = Instant::now();

    state.history.add(HistoryEntry {
        query_id,
        tab_id: tab_id_typed,
        sql: sql.clone(),
        status: QueryStatus::Running,
        started_at: Utc::now(),
        completed_at: None,
        rows_affected: None,
        elapsed_us: None,
        error: None,
    });

    let mut conn = pool.acquire().await.map_err(|e| {
        let elapsed = start.elapsed().as_micros() as u64;
        state.history.update(query_id, QueryStatus::Failed, elapsed, Some(e.to_string()));
        let _ = persist_history(&state.history, &state.history_path);
        format!("acquire: {e}")
    })?;

    let result = conn.connection_mut().execute(&sql).await.map_err(|e| {
        let elapsed = start.elapsed().as_micros() as u64;
        state.history.update(query_id, QueryStatus::Failed, elapsed, Some(format!("{e}")));
        let _ = persist_history(&state.history, &state.history_path);
        format!("{e}")
    })?;

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let elapsed_us = start.elapsed().as_micros() as u64;

    state.history.update(query_id, QueryStatus::Completed, elapsed_us, None);
    let _ = persist_history(&state.history, &state.history_path);

    Ok(vec![query_result_to_dto(result, elapsed_ms)])
}

/// Execute a SQL statement and stream results in batches via Tauri events.
///
/// The frontend listens for `query-batch` events with payloads:
/// - `{ type: "meta", queryId, columns, totalRows }`
/// - `{ type: "batch", queryId, rows: [[val,...],...] }`  (value-arrays, no keys)
/// - `{ type: "complete", queryId, elapsedMs, totalRows }`
#[tauri::command]
pub async fn execute_query_stream(
    app: tauri::AppHandle,
    profile_id: String,
    sql: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<StreamSummary, String> {
    if sql.trim().is_empty() {
        return Err("empty query".into());
    }

    let profile_id_typed = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let managed = state
        .manager
        .get(profile_id_typed)
        .ok_or_else(|| format!("not connected: {profile_id}"))?;

    let pool = managed
        .pool
        .ok_or_else(|| format!("no pool for profile: {profile_id}"))?;

    let query_id = Id::<QueryId>::new_v7();
    let query_id_str = query_id.to_string();
    let tab_id_typed = Id::<TabId>::parse(&tab_id).unwrap_or_else(Id::new);

    let start = Instant::now();

    state.history.add(HistoryEntry {
        query_id,
        tab_id: tab_id_typed,
        sql: sql.clone(),
        status: QueryStatus::Running,
        started_at: Utc::now(),
        completed_at: None,
        rows_affected: None,
        elapsed_us: None,
        error: None,
    });

    let mut conn = pool.acquire().await.map_err(|e| {
        let elapsed = start.elapsed().as_micros() as u64;
        state.history.update(query_id, QueryStatus::Failed, elapsed, Some(e.to_string()));
        let _ = persist_history(&state.history, &state.history_path);
        format!("acquire: {e}")
    })?;

    let result = conn.connection_mut().execute(&sql).await.map_err(|e| {
        let elapsed = start.elapsed().as_micros() as u64;
        state.history.update(query_id, QueryStatus::Failed, elapsed, Some(format!("{e}")));
        let _ = persist_history(&state.history, &state.history_path);
        format!("{e}")
    })?;

    let columns: Vec<ColumnInfo> = result.columns.clone();
    let total_rows = result.rows.len();

    // Emit column metadata
    let col_dtos: Vec<QueryColumnDto> = columns.iter().map(|c| QueryColumnDto {
        name: c.name.clone(),
        col_type: c.col_type.to_string(),
        db_type: c.db_type.clone(),
        nullable: c.nullable,
        ordinal: c.ordinal,
    }).collect();

    let meta = serde_json::json!({
        "type": "meta",
        "queryId": query_id_str,
        "tabId": tab_id,
        "columns": col_dtos,
        "totalRows": total_rows,
    });
    app.emit("query-batch", meta).map_err(|e| format!("emit meta: {e}"))?;

    // Emit rows in batches (value-arrays, not maps — much smaller on the wire)
    const BATCH_SIZE: usize = 50000;
    for chunk in result.rows.chunks(BATCH_SIZE) {
        let batch_rows: Vec<Vec<serde_json::Value>> = chunk
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .enumerate()
                    .map(|(i, col)| value_to_json(&row.get(i).cloned().unwrap_or(Value::Null)))
                    .collect()
            })
            .collect();

        let batch = serde_json::json!({
            "type": "batch",
            "queryId": query_id_str,
            "rows": batch_rows,
        });
        app.emit("query-batch", batch).map_err(|e| format!("emit batch: {e}"))?;
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let elapsed_us = start.elapsed().as_micros() as u64;

    let complete = serde_json::json!({
        "type": "complete",
        "queryId": query_id_str,
        "elapsedMs": elapsed_ms,
        "totalRows": total_rows,
    });
    app.emit("query-batch", complete).map_err(|e| format!("emit complete: {e}"))?;

    state.history.update(query_id, QueryStatus::Completed, elapsed_us, None);
    let _ = persist_history(&state.history, &state.history_path);

    Ok(StreamSummary { total_rows, elapsed_ms })
}


