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
    Schema,
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
        None => {
            // Initial load — list all databases
            list_databases(&pool, &managed.profile).await
        }
        Some(IntrospectKind::Database) => {
            // Expanding a database — show schemas (dbo, sys, etc.)
            let db = parent_db.or_else(|| {
                node_id.as_ref().and_then(|id| id.split('/').last().map(|s| s.to_string()))
            }).ok_or_else(|| "database name required".to_string())?;
            list_schemas(&pool, &managed.profile, &db).await
        }
        Some(IntrospectKind::Schema) => {
            // Expanding a schema — show Tables and Views folders
            let db = parent_db.ok_or_else(|| "database name required for Schema".to_string())?;
            let schema = node_id
                .as_ref()
                .and_then(|id| id.split('/').last())
                .map(|s| s.to_string())
                .ok_or_else(|| "schema name required".to_string())?;
            list_schema_contents(&pool, &managed.profile, &db, &schema).await
        }
        Some(IntrospectKind::TablesFolder) => {
            let db = parent_db.clone().ok_or_else(|| "parent_db required for TablesFolder".to_string())?;
            let schema = extract_schema_from_node_id(&node_id, &parent_db, db.len());
            list_tables(&pool, &managed.profile, &db, &schema, "BASE TABLE").await
        }
        Some(IntrospectKind::ViewsFolder) => {
            let db = parent_db.clone().ok_or_else(|| "parent_db required for ViewsFolder".to_string())?;
            let schema = extract_schema_from_node_id(&node_id, &parent_db, db.len());
            list_tables(&pool, &managed.profile, &db, &schema, "VIEW").await
        }
        Some(IntrospectKind::Table) => {
            let db = parent_db.clone().ok_or_else(|| "parent_db required for Table".to_string())?;
            let schema = extract_schema_from_node_id(&node_id, &parent_db, db.len());
            let table = node_id
                .as_ref()
                .and_then(|id| id.split('/').last())
                .map(|s| s.to_string())
                .ok_or_else(|| "table name required".to_string())?;
            list_columns(&pool, &managed.profile, &db, &schema, &table).await
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

async fn list_schemas(
    pool: &Arc<getagrip_database::ConnectionPool>,
    profile: &ConnectionProfile,
    database: &str,
) -> Result<Vec<ExplorerNode>, String> {
    let profile_id = &profile.id.to_string();
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;

    let sql = match profile.driver {
        getagrip_core::ConnectionDriver::Mssql => {
            format!("SELECT SCHEMA_NAME FROM {database}.sys.schemas WHERE SCHEMA_NAME NOT IN ('sys','INFORMATION_SCHEMA','guest','db_owner','db_accessadmin','db_securityadmin','db_ddladmin','db_backupoperator','db_datareader','db_datawriter','db_denydatareader','db_denydatawriter') ORDER BY SCHEMA_NAME")
        }
        _ => {
            "SELECT schema_name FROM information_schema.schemata WHERE schema_name NOT IN ('information_schema','pg_catalog') ORDER BY schema_name".to_string()
        }
    };

    let result = conn.connection_mut().execute(&sql).await.map_err(|e| format!("{e}"))?;

    let nodes: Vec<ExplorerNode> = result.rows.iter().map(|row| {
        let name = row.get(0).map(|v| v.to_string()).unwrap_or_default();
        ExplorerNode {
            id: format!("{profile_id}/{database}/{name}"),
            name: name.clone(),
            kind: ExplorerNodeKind::Schema,
            expanded: false,
            children_loaded: false,
            children: vec![],
            icon: None,
            favorite: false,
            tooltip: Some(format!("Schema: {name}")),
            loading: false,
            has_error: false,
        }
    }).collect();

    Ok(nodes)
}

async fn list_schema_contents(
    pool: &Arc<getagrip_database::ConnectionPool>,
    profile: &ConnectionProfile,
    database: &str,
    schema: &str,
) -> Result<Vec<ExplorerNode>, String> {
    let profile_id = &profile.id.to_string();
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;

    // Query table and view counts for this schema
    let count_sql = match profile.driver {
        getagrip_core::ConnectionDriver::Mssql => {
            format!(
                "SELECT TABLE_TYPE, COUNT(*) FROM {database}.INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{schema}' GROUP BY TABLE_TYPE"
            )
        }
        _ => format!(
            "SELECT table_type, COUNT(*) FROM information_schema.tables WHERE table_schema = '{schema}' GROUP BY table_type"
        ),
    };

    let mut table_count = 0u64;
    let mut view_count = 0u64;

    if let Ok(result) = conn.connection_mut().execute(&count_sql).await {
        for row in &result.rows {
            let kind = row.get(0).map(|v| v.to_string().to_lowercase()).unwrap_or_default();
            let count = row.get(1).and_then(|v| {
                if let getagrip_database::Value::Int(i) = v { Some(*i as u64) } else { None }
            }).unwrap_or(0);
            if kind.contains("base") { table_count = count; }
            else if kind.contains("view") { view_count = count; }
        }
    }

    Ok(vec![
        ExplorerNode {
            id: format!("{profile_id}/{database}/{schema}/tables"),
            name: format!("Tables ({table_count})"),
            kind: ExplorerNodeKind::Folder,
            expanded: false,
            children_loaded: false,
            children: vec![],
            icon: None,
            favorite: false,
            tooltip: Some(format!("{table_count} tables in {schema}")),
            loading: false,
            has_error: false,
        },
        ExplorerNode {
            id: format!("{profile_id}/{database}/{schema}/views"),
            name: format!("Views ({view_count})"),
            kind: ExplorerNodeKind::Folder,
            expanded: false,
            children_loaded: false,
            children: vec![],
            icon: None,
            favorite: false,
            tooltip: Some(format!("{view_count} views in {schema}")),
            loading: false,
            has_error: false,
        },
    ])
}

fn extract_schema_from_node_id(node_id: &Option<String>, _parent_db: &Option<String>, _db_len: usize) -> String {
    // Extract schema from the parent node_id path
    // node_id is like "profile_id/AdventureWorksDW2025/dbo"
    node_id
        .as_ref()
        .and_then(|id| {
            let parts: Vec<&str> = id.split('/').collect();
            if parts.len() >= 3 { Some(parts[2].to_string()) } else { None }
        })
        .unwrap_or_else(|| "dbo".to_string())
}

async fn list_tables(
    pool: &Arc<getagrip_database::ConnectionPool>,
    profile: &ConnectionProfile,
    database: &str,
    schema: &str,
    table_type: &str,
) -> Result<Vec<ExplorerNode>, String> {
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;

    let sql = match profile.driver {
        getagrip_core::ConnectionDriver::Mssql => format!(
            "SELECT TABLE_NAME FROM {database}.INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{schema}' AND TABLE_TYPE = '{table_type}' ORDER BY TABLE_NAME"
        ),
        _ => format!(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = '{schema}' AND table_type = '{table_type}' ORDER BY table_name"
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
    schema: &str,
    table: &str,
) -> Result<Vec<ExplorerNode>, String> {
    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;

    let sql = match profile.driver {
        getagrip_core::ConnectionDriver::Mssql => format!(
            "SELECT COLUMN_NAME, DATA_TYPE FROM {database}.INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{schema}' AND TABLE_NAME = '{table}' ORDER BY ORDINAL_POSITION"
        ),
        _ => format!(
            "SELECT column_name, data_type FROM information_schema.columns WHERE table_schema = '{schema}' AND table_name = '{table}' ORDER BY ordinal_position"
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
