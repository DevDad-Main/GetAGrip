//! Query execution commands.

use std::time::Instant;

use chrono::Utc;
use serde::Serialize;
use tauri::State;

use getagrip_core::id::Id;
use getagrip_core::session::ConnectionProfileId;
use getagrip_query_engine::scheduler::{QueryId, QueryStatus, TabId};
use getagrip_query_engine::HistoryEntry;

use crate::commands::util::{persist_history, query_result_to_dto};
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

/// Execute a SQL statement using a connected profile.
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
