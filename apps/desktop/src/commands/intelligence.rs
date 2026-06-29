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

    let driver = managed.profile.driver_name();
    let db = conn
        .connection()
        .info()
        .await
        .map(|info| info.database)
        .unwrap_or_else(|_| managed.profile.database.clone().unwrap_or_else(|| "master".to_string()));

    let mut schema = getagrip_schema::DatabaseSchema::new(&db);

    // Driver-aware information_schema queries
    let inner = conn.connection_mut();
    match driver {
        "postgres" => {
            let sql = "SELECT TABLE_SCHEMA, TABLE_NAME FROM information_schema.tables WHERE TABLE_TYPE = 'BASE TABLE' AND TABLE_SCHEMA NOT IN ('pg_catalog', 'information_schema') ORDER BY TABLE_SCHEMA, TABLE_NAME";
            if let Ok(result) = inner.execute(sql).await {
                for row in &result.rows {
                    let table_schema = row.get_by_name("TABLE_SCHEMA").map(|v| v.to_string()).unwrap_or_default();
                    let table_name = row.get_by_name("TABLE_NAME").map(|v| v.to_string()).unwrap_or_default();
                    let cols = fetch_info_schema_cols(inner, &table_schema, &table_name).await;
                    schema.tables.push(make_table(table_name, table_schema, cols));
                }
            }
        }
        "mysql" => {
            let sql = "SELECT TABLE_SCHEMA, TABLE_NAME FROM information_schema.TABLES WHERE TABLE_TYPE = 'BASE TABLE' AND TABLE_SCHEMA = DATABASE() ORDER BY TABLE_SCHEMA, TABLE_NAME";
            if let Ok(result) = inner.execute(sql).await {
                for row in &result.rows {
                    let table_schema = row.get_by_name("TABLE_SCHEMA").map(|v| v.to_string()).unwrap_or_default();
                    let table_name = row.get_by_name("TABLE_NAME").map(|v| v.to_string()).unwrap_or_default();
                    let cols = fetch_info_schema_cols(inner, &table_schema, &table_name).await;
                    schema.tables.push(make_table(table_name, table_schema, cols));
                }
            }
        }
        "mssql" => {
            let tables_sql = format!("SELECT TABLE_SCHEMA, TABLE_NAME FROM [{db}].INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_SCHEMA, TABLE_NAME");
            if let Ok(result) = inner.execute(&tables_sql).await {
                for row in &result.rows {
                    let table_schema = row.get_by_name("TABLE_SCHEMA").map(|v| v.to_string()).unwrap_or_else(|| "dbo".into());
                    let table_name = row.get_by_name("TABLE_NAME").map(|v| v.to_string()).unwrap_or_default();
                    let cols = fetch_mssql_cols(inner, &db, &table_schema, &table_name).await;
                    schema.tables.push(make_table(table_name, table_schema, cols));
                }
            }
        }
        "sqlite" => {
            if let Ok(result) = inner.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").await {
                for row in &result.rows {
                    let table_name = row.get_by_name("name").map(|v| v.to_string()).unwrap_or_default();
                    let cols = fetch_sqlite_cols(inner, &table_name).await;
                    schema.tables.push(make_table(table_name, "main".into(), cols));
                }
            }
        }
        other => {
            tracing::warn!("no metadata refresh impl for driver: {}", other);
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

fn make_table(name: String, schema: String, columns: Vec<getagrip_schema::ColumnSchema>) -> getagrip_schema::TableSchema {
    getagrip_schema::TableSchema {
        name,
        schema,
        columns,
        constraints: vec![],
        indexes: vec![],
        comment: None,
        row_count_estimate: None,
    }
}

async fn fetch_info_schema_cols(
    conn: &mut dyn getagrip_database::driver::DriverConnection,
    table_schema: &str,
    table_name: &str,
) -> Vec<getagrip_schema::ColumnSchema> {
    let sql = format!(
        "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, ORDINAL_POSITION FROM information_schema.columns WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' ORDER BY ORDINAL_POSITION",
        table_schema.replace('\'', "''"),
        table_name.replace('\'', "''"),
    );
    let mut columns = Vec::new();
    if let Ok(col_result) = conn.execute(&sql).await {
        for col_row in &col_result.rows {
            let col_name = col_row.get_by_name("COLUMN_NAME").map(|v| v.to_string()).unwrap_or_default();
            let data_type = col_row.get_by_name("DATA_TYPE").map(|v| v.to_string()).unwrap_or_default();
            let nullable = col_row.get_by_name("IS_NULLABLE").map(|v| v.to_string() == "YES").unwrap_or(true);
            let ordinal = col_row.get_by_name("ORDINAL_POSITION").map(|v| v.to_string().parse::<u16>().unwrap_or(0)).unwrap_or(0);
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
    columns
}

async fn fetch_mssql_cols(
    conn: &mut dyn getagrip_database::driver::DriverConnection,
    db: &str,
    table_schema: &str,
    table_name: &str,
) -> Vec<getagrip_schema::ColumnSchema> {
    let sql = format!(
        "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, ORDINAL_POSITION FROM [{db}].INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{schema}' AND TABLE_NAME = '{name}' ORDER BY ORDINAL_POSITION",
        schema = table_schema.replace('\'', "''"),
        name = table_name.replace('\'', "''"),
    );
    let mut columns = Vec::new();
    if let Ok(col_result) = conn.execute(&sql).await {
        for col_row in &col_result.rows {
            let col_name = col_row.get_by_name("COLUMN_NAME").map(|v| v.to_string()).unwrap_or_default();
            let data_type = col_row.get_by_name("DATA_TYPE").map(|v| v.to_string()).unwrap_or_default();
            let nullable = col_row.get_by_name("IS_NULLABLE").map(|v| v.to_string() == "YES").unwrap_or(true);
            let ordinal = col_row.get_by_name("ORDINAL_POSITION").map(|v| v.to_string().parse::<u16>().unwrap_or(0)).unwrap_or(0);
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
    columns
}

async fn fetch_sqlite_cols(
    conn: &mut dyn getagrip_database::driver::DriverConnection,
    table_name: &str,
) -> Vec<getagrip_schema::ColumnSchema> {
    let sql = format!("PRAGMA table_info('{}')", table_name.replace('\'', "''"));
    let mut columns = Vec::new();
    if let Ok(pragma) = conn.execute(&sql).await {
        for pr in &pragma.rows {
            let col_name = pr.get_by_name("name").map(|v| v.to_string()).unwrap_or_default();
            let data_type = pr.get_by_name("type").map(|v| v.to_string()).unwrap_or_default();
            let nullable = pr.get_by_name("notnull").map(|v| v.to_string() != "1").unwrap_or(true);
            let ordinal = pr.get_by_name("cid").map(|v| v.to_string().parse::<u16>().unwrap_or(0)).unwrap_or(0);
            columns.push(getagrip_schema::ColumnSchema {
                name: col_name,
                col_type: getagrip_database::driver::ColumnType::String,
                db_type: data_type,
                nullable,
                default_value: None,
                is_primary_key: pr.get_by_name("pk").map(|v| v.to_string() == "1").unwrap_or(false),
                ordinal,
                comment: None,
            });
        }
    }
    columns
}


