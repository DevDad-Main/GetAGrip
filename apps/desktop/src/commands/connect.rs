//! Connection commands: test a URL, connect, list databases.
//!
//! The SQL Server driver is the only supported driver in Phase 1. The
//! introspection SQL is the same queries the old Slint `main.rs` used —
//! `sys.databases`, `INFORMATION_SCHEMA.TABLES`, `INFORMATION_SCHEMA.VIEWS`.

use std::time::Instant;

use serde::Serialize;

use getagrip_database::DatabaseDriver;
use getagrip_explorer::{ExplorerNode, ExplorerNodeKind};

use crate::commands::util::driver;

/// Result of a successful `connect` call: the connection's display name
/// plus the top-level explorer nodes (one server + N databases).
#[derive(Debug, Clone, Serialize)]
pub struct ConnectResult {
    /// Human-readable connection name (as the user typed it).
    pub name: String,
    /// Database product name, if the server reported one.
    pub product_name: String,
    /// Server version string.
    pub version: String,
    /// Top-level explorer nodes to render in the sidebar.
    pub nodes: Vec<ExplorerNode>,
}

/// Test whether a URL is reachable without fully connecting.
#[tauri::command]
pub async fn test_connection(url: String) -> Result<(), String> {
    let d = driver();
    d.test_connection(&url)
        .await
        .map_err(|e| format!("{e}"))
}

/// Connect to a database URL and return the initial explorer tree.
#[tauri::command]
pub async fn connect(url: String, name: String) -> Result<ConnectResult, String> {
    let start = Instant::now();
    let d = driver();
    let mut conn = d.connect(&url).await.map_err(|e| format!("{e}"))?;

    let info = conn.info().await.ok();
    let product_name = info
        .as_ref()
        .map(|i| i.product_name.clone())
        .unwrap_or_else(|| "Microsoft SQL Server".into());
    let version = info
        .as_ref()
        .map(|i| i.version.raw.clone())
        .unwrap_or_default();

    // List databases with table/view counts.
    let db_rows = conn
        .execute("SELECT name FROM sys.databases ORDER BY name")
        .await
        .map_err(|e| format!("{e}"))?;

    let mut db_names: Vec<(String, i64, i64)> = Vec::new();
    for row in &db_rows.rows {
        let name = row.get(0).map(|v| v.to_string()).unwrap_or_default();
        let mut tc = 0i64;
        let mut vc = 0i64;
        let tbl_sql = format!(
            "SELECT COUNT(*) FROM [{name}].INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'"
        );
        if let Ok(r) = conn.execute(&tbl_sql).await {
            if let Some(row) = r.rows.first() {
                tc = row
                    .get(0)
                    .and_then(|v| match v {
                        getagrip_database::driver::Value::Int(i) => Some(*i),
                        _ => None,
                    })
                    .unwrap_or(0);
            }
        }
        let view_sql = format!("SELECT COUNT(*) FROM [{name}].INFORMATION_SCHEMA.VIEWS");
        if let Ok(r) = conn.execute(&view_sql).await {
            if let Some(row) = r.rows.first() {
                vc = row
                    .get(0)
                    .and_then(|v| match v {
                        getagrip_database::driver::Value::Int(i) => Some(*i),
                        _ => None,
                    })
                    .unwrap_or(0);
            }
        }
        db_names.push((name, tc, vc));
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    tracing::info!(
        "connect: {} databases in {}ms",
        db_names.len(),
        elapsed_ms
    );

    // Build explorer tree: one server node + one database node each.
    let mut nodes: Vec<ExplorerNode> = Vec::new();
    nodes.push(ExplorerNode::new(
        format!("conn:{url}"),
        name.clone(),
        ExplorerNodeKind::Server,
    ));
    for (db, tc, vc) in &db_names {
        let label = if *tc > 0 || *vc > 0 {
            format!("{db}  ({tc} tables, {vc} views)")
        } else {
            db.clone()
        };
        let mut node = ExplorerNode::new(
            format!("db:{url}:{db}"),
            label,
            ExplorerNodeKind::Database,
        );
        node.children_loaded = false;
        nodes.push(node);
    }

    Ok(ConnectResult {
        name,
        product_name,
        version,
        nodes,
    })
}

/// Disconnect from a URL. Phase 1 holds no long-lived connection pool, so
/// this is a no-op beyond logging — the frontend just clears its state.
#[tauri::command]
pub async fn disconnect(_url: String) -> Result<(), String> {
    tracing::info!("disconnect: {_url}");
    Ok(())
}
