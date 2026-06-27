//! Lazy introspection for the explorer tree.
//!
//! The frontend calls `introspect` when a node is expanded. We run the
//! appropriate `INFORMATION_SCHEMA` query and return fresh `ExplorerNode`s
//! to splice into the tree.

use serde::{Deserialize, Serialize};

use getagrip_database::DatabaseDriver;
use getagrip_explorer::{ExplorerNode, ExplorerNodeKind};

use crate::commands::util::driver;

/// What kind of node the frontend wants to expand.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IntrospectKind {
    /// A database node — list its `Tables (N)` and `Views (M)` folders.
    Database,
    /// A "Tables (N)" folder — list the actual tables.
    TablesFolder,
    /// A "Views (M)" folder — list the actual views.
    ViewsFolder,
    /// A table node — list its columns.
    Table,
}

/// Lazy-load children for an explorer node and return them.
///
/// The frontend is responsible for splicing the returned nodes into its
/// tree; we just return the new children.
#[tauri::command]
pub async fn introspect(
    node_id: String,
    kind: IntrospectKind,
    parent_db: Option<String>,
    url: String,
) -> Result<Vec<ExplorerNode>, String> {
    let d = driver();
    let mut conn = d.connect(&url).await.map_err(|e| format!("{e}"))?;

    match kind {
        IntrospectKind::Database => {
            let db_name = parent_db.ok_or_else(|| "database name missing".to_string())?;
            let tc = count_int(
                &mut conn,
                &format!("SELECT COUNT(*) FROM [{db_name}].INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'"),
            )
            .await?;
            let vc = count_int(
                &mut conn,
                &format!("SELECT COUNT(*) FROM [{db_name}].INFORMATION_SCHEMA.VIEWS"),
            )
            .await?;

            let mut nodes: Vec<ExplorerNode> = Vec::new();
            if tc > 0 {
                let mut folder = ExplorerNode::new(
                    format!("tables:{url}:{db_name}"),
                    format!("Tables ({tc})"),
                    ExplorerNodeKind::Folder,
                );
                folder.children_loaded = false;
                nodes.push(folder);
            }
            if vc > 0 {
                let mut folder = ExplorerNode::new(
                    format!("views:{url}:{db_name}"),
                    format!("Views ({vc})"),
                    ExplorerNodeKind::Folder,
                );
                folder.children_loaded = false;
                nodes.push(folder);
            }
            Ok(nodes)
        }
        IntrospectKind::TablesFolder => {
            let db_name = parent_db.ok_or_else(|| "database name missing".to_string())?;
            let sql = format!(
                "SELECT TABLE_NAME FROM [{db_name}].INFORMATION_SCHEMA.TABLES \
                 WHERE TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME"
            );
            let rows = conn.execute(&sql).await.map_err(|e| format!("{e}"))?;
            Ok(rows
                .rows
                .iter()
                .filter_map(|row| {
                    row.get(0).map(|v| {
                        let name = v.to_string();
                        ExplorerNode::new(
                            format!("table:{url}:{db_name}:{name}"),
                            name,
                            ExplorerNodeKind::Table,
                        )
                    })
                })
                .collect())
        }
        IntrospectKind::ViewsFolder => {
            let db_name = parent_db.ok_or_else(|| "database name missing".to_string())?;
            let sql = format!(
                "SELECT TABLE_NAME FROM [{db_name}].INFORMATION_SCHEMA.VIEWS ORDER BY TABLE_NAME"
            );
            let rows = conn.execute(&sql).await.map_err(|e| format!("{e}"))?;
            Ok(rows
                .rows
                .iter()
                .filter_map(|row| {
                    row.get(0).map(|v| {
                        let name = v.to_string();
                        ExplorerNode::new(
                            format!("view:{url}:{db_name}:{name}"),
                            name,
                            ExplorerNodeKind::View,
                        )
                    })
                })
                .collect())
        }
        IntrospectKind::Table => {
            let db_name = parent_db.ok_or_else(|| "database name missing".to_string())?;
            // Decode the table name from the node_id (format: "table:{url}:{db}:{table}").
            let table_name = node_id
                .split(':')
                .nth(3)
                .ok_or_else(|| format!("bad node_id: {node_id}"))?
                .to_string();
            let sql = format!(
                "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE \
                 FROM [{db_name}].INFORMATION_SCHEMA.COLUMNS \
                 WHERE TABLE_NAME = '{table_name}' ORDER BY ORDINAL_POSITION"
            );
            let rows = conn.execute(&sql).await.map_err(|e| format!("{e}"))?;
            Ok(rows
                .rows
                .iter()
                .filter_map(|row| {
                    let name = row.get(0).map(|v| v.to_string()).unwrap_or_default();
                    let dtype = row.get(1).map(|v| v.to_string()).unwrap_or_default();
                    if name.is_empty() {
                        return None;
                    }
                    Some(ExplorerNode::new(
                        format!("col:{url}:{db_name}:{table_name}:{name}"),
                        format!("{name}  {dtype}"),
                        ExplorerNodeKind::Column,
                    ))
                })
                .collect())
        }
    }
}

/// Run a `SELECT COUNT(*)` and return the first column of the first row.
async fn count_int(
    conn: &mut Box<dyn getagrip_database::driver::DriverConnection>,
    sql: &str,
) -> Result<i64, String> {
    let r = conn.execute(sql).await.map_err(|e| format!("{e}"))?;
    r.rows
        .first()
        .and_then(|row| row.get(0))
        .and_then(|v| match v {
            getagrip_database::driver::Value::Int(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| "count query returned no rows".to_string())
        .or(Ok(0))
}

