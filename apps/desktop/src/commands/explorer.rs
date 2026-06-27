use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use getagrip_core::id::Id;
use getagrip_core::session::{ConnectionProfile, ConnectionProfileId};
use getagrip_explorer::{ExplorerNode, ExplorerNodeKind};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub enum IntrospectKind {
    Database,
    TablesFolder,
    ViewsFolder,
    Table,
}

#[derive(Debug, Serialize)]
pub struct IntrospectNodeResult {
    pub nodes: Vec<ExplorerNode>,
}

#[tauri::command]
pub async fn introspect_node(
    profile_id: String,
    node_id: Option<String>,
    kind: Option<IntrospectKind>,
    parent_db: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ExplorerNode>, String> {
    let profile_id_typed = Id::<ConnectionProfileId>::parse(&profile_id)
        .ok_or_else(|| format!("invalid profile id: {profile_id}"))?;

    let managed = state
        .manager
        .get(profile_id_typed)
        .ok_or_else(|| format!("not connected: {profile_id}"))?;

    let pool = managed
        .pool
        .ok_or_else(|| format!("no pool for profile: {profile_id}"))?;

    match kind {
        Some(IntrospectKind::Database) | None => {
            list_databases(&pool, &managed.profile).await
        }
        Some(IntrospectKind::TablesFolder) => {
            let db = parent_db.ok_or_else(|| "parent_db required for TablesFolder".to_string())?;
            list_tables(&pool, &managed.profile, &db, "BASE TABLE").await
        }
        Some(IntrospectKind::ViewsFolder) => {
            let db = parent_db.ok_or_else(|| "parent_db required for ViewsFolder".to_string())?;
            list_tables(&pool, &managed.profile, &db, "VIEW").await
        }
        Some(IntrospectKind::Table) => {
            let db = parent_db.ok_or_else(|| "parent_db required for Table".to_string())?;
            let table = node_id.ok_or_else(|| "node_id required for Table".to_string())?;
            list_columns(&pool, &managed.profile, &db, &table).await
        }
    }
}

async fn list_databases(
    pool: &Arc<getagrip_database::ConnectionPool>,
    profile: &ConnectionProfile,
) -> Result<Vec<ExplorerNode>, String> {
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;
    let sql = match profile.driver {
        getagrip_core::ConnectionDriver::Mssql => {
            "SELECT name FROM sys.databases WHERE name NOT IN ('master','tempdb','model','msdb') ORDER BY name"
        }
        _ => "SELECT schema_name FROM information_schema.schemata ORDER BY schema_name",
    };
    let result = conn.connection_mut().execute(sql).await.map_err(|e| format!("{e}"))?;

    let nodes: Vec<ExplorerNode> = result
        .rows
        .iter()
        .map(|row| {
            let name = row.get(0).map(|v| v.to_string()).unwrap_or_default();
            ExplorerNode {
                id: format!("{}/{}", profile.id, name),
                name,
                kind: ExplorerNodeKind::Database,
                expanded: false,
                children_loaded: false,
                children: vec![],
                icon: None,
                favorite: false,
                tooltip: None,
                loading: false,
                has_error: false,
            }
        })
        .collect();

    Ok(nodes)
}

async fn list_tables(
    pool: &Arc<getagrip_database::ConnectionPool>,
    profile: &ConnectionProfile,
    database: &str,
    table_type: &str,
) -> Result<Vec<ExplorerNode>, String> {
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;

    let sql = match profile.driver {
        getagrip_core::ConnectionDriver::Mssql => format!(
            "SELECT TABLE_NAME FROM {database}.INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = '{table_type}' ORDER BY TABLE_NAME"
        ),
        _ => format!(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = '{database}' AND table_type = '{table_type}' ORDER BY table_name"
        ),
    };

    let result = conn.connection_mut().execute(&sql).await.map_err(|e| format!("{e}"))?;

    let kind = if table_type == "VIEW" {
        ExplorerNodeKind::View
    } else {
        ExplorerNodeKind::Table
    };

    let nodes: Vec<ExplorerNode> = result
        .rows
        .iter()
        .map(|row| {
            let name = row.get(0).map(|v| v.to_string()).unwrap_or_default();
            ExplorerNode {
                id: format!("{}/{}/{}", profile.id, database, name),
                name,
                kind,
                expanded: false,
                children_loaded: false,
                children: vec![],
                icon: None,
                favorite: false,
                tooltip: None,
                loading: false,
                has_error: false,
            }
        })
        .collect();

    Ok(nodes)
}

async fn list_columns(
    pool: &Arc<getagrip_database::ConnectionPool>,
    profile: &ConnectionProfile,
    database: &str,
    table: &str,
) -> Result<Vec<ExplorerNode>, String> {
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;

    let sql = match profile.driver {
        getagrip_core::ConnectionDriver::Mssql => format!(
            "SELECT COLUMN_NAME, DATA_TYPE FROM {database}.INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = '{table}' ORDER BY ORDINAL_POSITION"
        ),
        _ => format!(
            "SELECT column_name, data_type FROM information_schema.columns WHERE table_schema = '{database}' AND table_name = '{table}' ORDER BY ordinal_position"
        ),
    };

    let result = conn.connection_mut().execute(&sql).await.map_err(|e| format!("{e}"))?;

    let nodes: Vec<ExplorerNode> = result
        .rows
        .iter()
        .map(|row| {
            let name = row.get(0).map(|v| v.to_string()).unwrap_or_default();
            let data_type = row.get(1).map(|v| v.to_string()).unwrap_or_default();
            ExplorerNode {
                id: format!("{}/{}/{}/{}", profile.id, database, table, name),
                name: format!("{name} ({data_type})"),
                kind: ExplorerNodeKind::Column,
                expanded: false,
                children_loaded: true,
                children: vec![],
                icon: None,
                favorite: false,
                tooltip: Some(data_type),
                loading: false,
                has_error: false,
            }
        })
        .collect();

    Ok(nodes)
}
