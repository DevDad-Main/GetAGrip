//! Shared helpers for Tauri command handlers.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use getagrip_core::session::{ConnectionDriver, ConnectionProfile, ConnectionProfiles};
use getagrip_database::DatabaseDriver;
use getagrip_database::driver::{ColumnInfo, QueryResult, Value};
use getagrip_driver_sqlserver::SqlServerDriver;
use getagrip_driver_postgres::PostgresDriver;

use crate::commands::query::{QueryColumnDto, QueryResultDto};

use serde::Serialize;

/// Build a [`SqlServerDriver`] with default options (trust cert = true).
pub fn driver() -> SqlServerDriver {
    SqlServerDriver::new()
}

/// Create the appropriate driver for a connection profile.
pub fn driver_for(profile: &ConnectionProfile) -> Result<Arc<dyn DatabaseDriver>, String> {
    match profile.driver {
        ConnectionDriver::Mssql => Ok(Arc::new(SqlServerDriver::new())),
        ConnectionDriver::Postgres => Ok(Arc::new(PostgresDriver::new())),
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

/// Persist query history to disk.
pub fn persist_history(history: &getagrip_query_engine::QueryHistory, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir: {e}"))?;
    }
    let entries = history.all();
    let json = serde_json::to_string_pretty(&entries).map_err(|e| format!("serialize: {e}"))?;
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

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Determine the shell name from a full path (e.g. `/usr/bin/fish` → `fish`).
fn shell_name(shell_path: &str) -> &str {
    Path::new(shell_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sh")
}

/// Wrap a command so the shell sources its rc file first (aliases, etc.).
fn wrap_for_shell(shell: &str, command: &str) -> String {
    let name = shell_name(shell);
    match name {
        "bash" => format!("source ~/.bashrc 2>/dev/null; {}", command),
        "zsh"  => format!("source ~/.zshrc  2>/dev/null; {}", command),
        "fish" => format!("source $HOME/.config/fish/config.fish 2>/dev/null; {}", command),
        _ => command.to_string(),
    }
}

/// Run a shell command and return its output. Used by the integrated terminal.
///
/// The command is run through the user's default shell (`$SHELL`), or through
/// `shell` if provided.  Bash/zsh rc files are explicitly sourced so aliases
/// and config apply.
#[tauri::command]
pub async fn run_command(
    command: String,
    shell: Option<String>,
) -> Result<CommandOutput, String> {
    let shell_path = shell
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/sh".to_string());

    let wrapped = wrap_for_shell(&shell_path, &command);

    let output = std::process::Command::new(&shell_path)
        .args(["-c", &wrapped])
        .output()
        .map_err(|e| format!("Failed to run '{command}': {e}"))?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CommandOutput { stdout, stderr, exit_code })
}

/// Detect which shells are available on the system.
///
/// Returns a map of shell name → absolute path for every shell found.
#[tauri::command]
pub fn detect_available_shells() -> HashMap<String, String> {
    let mut candidates = vec![
        ("bash".to_string(), "/bin/bash".to_string()),
        ("bash".to_string(), "/usr/bin/bash".to_string()),
        ("zsh".to_string(), "/bin/zsh".to_string()),
        ("zsh".to_string(), "/usr/bin/zsh".to_string()),
        ("fish".to_string(), "/usr/bin/fish".to_string()),
        ("fish".to_string(), "/usr/local/bin/fish".to_string()),
        ("sh".to_string(), "/bin/sh".to_string()),
        ("sh".to_string(), "/usr/bin/sh".to_string()),
    ];

    // Also check the SHELL environment variable
    if let Ok(shell_path) = std::env::var("SHELL") {
        let name = shell_name(&shell_path).to_string();
        // Only add if not already present (by name)
        if !candidates.iter().any(|(n, _)| n == &name) {
            candidates.push((name, shell_path));
        }
    }

    let mut result = HashMap::new();
    for (name, path) in &candidates {
        if !result.contains_key(name) && Path::new(path).exists() {
            result.insert(name.clone(), path.clone());
        }
    }
    result
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

