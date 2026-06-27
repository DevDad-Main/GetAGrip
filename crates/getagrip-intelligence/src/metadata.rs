//! Metadata cache — stores database schema snapshots indexed for fast lookups.
//!
//! Key design: all lookups use `connection_id` as the primary key.
//! Database name is NOT part of the key — a connection has exactly one
//! active database, so storing by DB would create mismatches between
//! what `refresh_metadata_cmd` discovers and what `request_completion_cmd`
//! looks up.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use getagrip_schema::DatabaseSchema;

use crate::types::CompletionKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTable {
    pub name: String,
    pub schema_name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub db_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub ordinal: u16,
    pub kind: CompletionKind,
}

#[derive(Debug, Default)]
pub struct MetadataCache {
    schemas: DashMap<String, DatabaseSchema>,
    tables: DashMap<String, Vec<CachedTable>>,
    columns: DashMap<String, Vec<ColumnInfo>>,
}

impl MetadataCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store(&self, connection_id: &str, schema: DatabaseSchema) {
        let mut cached_tables: Vec<CachedTable> = Vec::new();

        for table in &schema.tables {
            let columns: Vec<ColumnInfo> = table
                .columns
                .iter()
                .map(|col| ColumnInfo {
                    name: col.name.clone(),
                    db_type: col.db_type.clone(),
                    nullable: col.nullable,
                    is_primary_key: col.is_primary_key,
                    ordinal: col.ordinal,
                    kind: CompletionKind::Column,
                })
                .collect();

            let col_key = col_key(connection_id, &table.name);
            self.columns.insert(col_key, columns.clone());

            cached_tables.push(CachedTable {
                name: table.name.clone(),
                schema_name: if table.schema.is_empty() {
                    "dbo".into()
                } else {
                    table.schema.clone()
                },
                columns,
            });
        }

        for view in &schema.views {
            let col_key = col_key(connection_id, &view.name);
            self.columns.insert(
                col_key,
                vec![ColumnInfo {
                    name: "rows".into(),
                    db_type: "view".into(),
                    nullable: true,
                    is_primary_key: false,
                    ordinal: 0,
                    kind: CompletionKind::View,
                }],
            );

            cached_tables.push(CachedTable {
                name: view.name.clone(),
                schema_name: view.schema.clone(),
                columns: vec![],
            });
        }

        self.tables.insert(connection_id.to_owned(), cached_tables);
        self.schemas.insert(connection_id.to_owned(), schema);
    }

    pub fn get_schema(&self, connection_id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, DatabaseSchema>> {
        self.schemas.get(connection_id)
    }

    pub fn get_tables(&self, connection_id: &str) -> Vec<CachedTable> {
        self.tables
            .get(connection_id)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    pub fn get_columns(&self, connection_id: &str, table: &str) -> Vec<ColumnInfo> {
        let key = col_key(connection_id, table);
        self.columns
            .get(&key)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    pub fn invalidate(&self, connection_id: &str) {
        self.schemas.remove(connection_id);
        self.tables.remove(connection_id);
        self.columns.retain(|k, _| !k.starts_with(connection_id));
    }

    pub fn has(&self, connection_id: &str) -> bool {
        self.schemas.contains_key(connection_id)
    }
}

fn col_key(connection_id: &str, table: &str) -> String {
    format!("{connection_id}/{table}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use getagrip_schema::{ColumnSchema, TableSchema};

    fn sample_schema() -> DatabaseSchema {
        let mut schema = DatabaseSchema::new("testdb");
        schema.tables.push(TableSchema {
            name: "users".into(),
            schema: "dbo".into(),
            columns: vec![ColumnSchema {
                name: "id".into(),
                col_type: getagrip_database::driver::ColumnType::Integer,
                db_type: "int".into(),
                nullable: false,
                default_value: None,
                is_primary_key: true,
                ordinal: 0,
                comment: None,
            }],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        schema
    }

    #[test]
    fn store_and_retrieve() {
        let cache = MetadataCache::new();
        cache.store("conn1", sample_schema());

        let tables = cache.get_tables("conn1");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "users");

        let cols = cache.get_columns("conn1", "users");
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].name, "id");
        assert_eq!(cols[0].db_type, "int");
    }

    #[test]
    fn invalidate_removes_data() {
        let cache = MetadataCache::new();
        cache.store("conn1", sample_schema());
        assert!(cache.has("conn1"));
        cache.invalidate("conn1");
        assert!(!cache.has("conn1"));
        assert!(cache.get_tables("conn1").is_empty());
    }
}
