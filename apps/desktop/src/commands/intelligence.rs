use tauri::State;

use getagrip_core::id::Id;
use getagrip_core::session::ConnectionProfileId;
use getagrip_intelligence::{
    CompletionRequest, CompletionResponse, DiagnosticsRequest, DiagnosticsResponse,
    MetadataRefreshRequest,
};
use getagrip_intelligence::context::analyse_context;
use getagrip_intelligence::completion::request_completion;
use getagrip_intelligence::diagnostics::request_diagnostics;

use crate::state::AppState;

#[tauri::command]
pub async fn request_completion_cmd(
    state: State<'_, AppState>,
    request: CompletionRequest,
) -> Result<CompletionResponse, String> {
    let table_count = state.metadata_cache.get_tables(&request.connection_id).len();

    tracing::debug!(
        "completion: sql={:?} line={} col={} cache_tables={}",
        &request.sql[..request.sql.len().min(50)],
        request.cursor_line,
        request.cursor_column,
        table_count,
    );

    // Engine completions (always available, schema-aware).
    let engine_suggestions = request_completion(
        &request.sql,
        request.cursor_line,
        request.cursor_column,
        &request.connection_id,
        &state.metadata_cache,
    );

    // LSP completions (only when an LSP server is registered for this driver).
    let driver = state
        .manager
        .get_by_id_str(&request.connection_id)
        .map(|c| c.profile.driver_name().to_string());
    let lsp_suggestions = match driver {
        Some(d) => state
            .lsp_manager
            .lock()
            .complete(&request.connection_id, &d, &request.sql, request.cursor_line, request.cursor_column),
        None => Vec::new(),
    };

    let suggestions = if lsp_suggestions.is_empty() {
        engine_suggestions
    } else {
        getagrip_intelligence::lsp_client::merge_completions(&engine_suggestions, &lsp_suggestions)
    };

    // Surface the cursor word + its start column so the frontend can compute the
    // replacement range and highlight precisely, without relying on Monaco's
    // word segmentation (which can disagree with the engine on word boundaries).
    let ctx = analyse_context(&request.sql, request.cursor_line, request.cursor_column);
    let cursor_word = if ctx.cursor_word.is_empty() { None } else { Some(ctx.cursor_word) };
    let cursor_word_start_col = if cursor_word.is_none() { None } else { Some(ctx.cursor_word_start_col) };

    Ok(CompletionResponse {
        suggestions,
        cursor_word,
        cursor_word_start_col,
    })
}

#[tauri::command]
pub async fn request_diagnostics_cmd(
    state: State<'_, AppState>,
    request: DiagnosticsRequest,
) -> Result<DiagnosticsResponse, String> {
    let diagnostics = request_diagnostics(
        &request.sql,
        &request.connection_id,
        &state.metadata_cache,
    );

    Ok(DiagnosticsResponse { diagnostics })
}

#[tauri::command]
pub async fn refresh_metadata_cmd(
    state: State<'_, AppState>,
    request: MetadataRefreshRequest,
) -> Result<(), String> {
    let profile_id = Id::<ConnectionProfileId>::parse(&request.connection_id)
        .ok_or_else(|| format!("invalid profile id: {}", request.connection_id))?;

    let managed = state
        .manager
        .get(profile_id)
        .ok_or_else(|| format!("not connected: {}", request.connection_id))?;

    let pool = managed
        .pool
        .ok_or_else(|| format!("no pool for profile: {}", request.connection_id))?;

    let mut conn = pool.acquire().await.map_err(|e| format!("acquire: {e}"))?;

    // Use the actual connected database, not the profile field (may be empty)
    let db = conn
        .connection()
        .info()
        .await
        .map(|info| info.database)
        .unwrap_or_else(|_| managed.profile.database.clone().unwrap_or_else(|| "master".to_string()));

    let mut schema = getagrip_schema::DatabaseSchema::new(&db);

    let tables_sql = format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME FROM {db}.INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_SCHEMA, TABLE_NAME"
    );
    if let Ok(result) = conn.connection_mut().execute(&tables_sql).await {
        for row in &result.rows {
            let table_schema = row
                .get_by_name("TABLE_SCHEMA")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "dbo".into());
            let table_name = row
                .get_by_name("TABLE_NAME")
                .map(|v| v.to_string())
                .unwrap_or_default();

            let cols_sql = format!(
                "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, ORDINAL_POSITION FROM {db}.INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{table_schema}' AND TABLE_NAME = '{table_name}' ORDER BY ORDINAL_POSITION"
            );
            let mut columns = Vec::new();
            if let Ok(col_result) = conn.connection_mut().execute(&cols_sql).await {
                for col_row in &col_result.rows {
                    let col_name = col_row
                        .get_by_name("COLUMN_NAME")
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let data_type = col_row
                        .get_by_name("DATA_TYPE")
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let nullable = col_row
                        .get_by_name("IS_NULLABLE")
                        .map(|v| v.to_string() == "YES")
                        .unwrap_or(true);
                    let ordinal = col_row
                        .get_by_name("ORDINAL_POSITION")
                        .map(|v| v.to_string().parse::<u16>().unwrap_or(0))
                        .unwrap_or(0);

                    columns.push(getagrip_schema::ColumnSchema {
                        name: col_name,
                        col_type: getagrip_database::driver::ColumnType::String,
                        db_type: data_type,
                        nullable,
                        default_value: None,
                        is_primary_key: false,
                        ordinal,
                        comment: None,
                    });
                }
            }

            schema.tables.push(getagrip_schema::TableSchema {
                name: table_name,
                schema: table_schema,
                columns,
                constraints: vec![],
                indexes: vec![],
                comment: None,
                row_count_estimate: None,
            });
        }
    }

    let table_count = schema.tables.len();
    state.metadata_cache.store(&request.connection_id, schema);
    tracing::info!(
        "Metadata refreshed for {}: {} tables (db={})",
        request.connection_id,
        table_count,
        db
    );

    Ok(())
}


