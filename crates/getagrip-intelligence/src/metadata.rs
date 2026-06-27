//! Metadata cache — stores database schema snapshots indexed for fast lookups.
//!
//! The cache is a [`dashmap::DashMap`] keyed by connection ID. Each entry
//! holds a [`DatabaseSchema`] plus secondary indexes for O(1) table and
//! column lookups by name.

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
        let db_name = schema.database_name.clone();

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

            let schema = if table.schema.is_empty() { "dbo" } else { &table.schema };
            let table_key = table_key(connection_id, &db_name, schema, &table.name);
            self.columns.insert(table_key, columns.clone());

            cached_tables.push(CachedTable {
                name: table.name.clone(),
                schema_name: if table.schema.is_empty() { "dbo".into() } else { table.schema.clone() },
                columns,
            });
        }

        for view in &schema.views {
            let columns: Vec<ColumnInfo> = vec![ColumnInfo {
                name: "rows".into(),
                db_type: "view".into(),
                nullable: true,
                is_primary_key: false,
                ordinal: 0,
                kind: CompletionKind::View,
            }];

            let table_key = table_key(connection_id, &db_name, &view.schema, &view.name);
            self.columns.insert(table_key, columns.clone());

            cached_tables.push(CachedTable {
                name: view.name.clone(),
                schema_name: view.schema.clone(),
                columns,
            });
        }

        let key = format!("{connection_id}/{db_name}");
        self.tables.insert(key, cached_tables);
        self.schemas.insert(connection_id.to_owned(), schema);
    }

    pub fn get_schema(&self, connection_id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, DatabaseSchema>> {
        self.schemas.get(connection_id)
    }

    pub fn get_tables(&self, connection_id: &str, database: &str) -> Vec<CachedTable> {
        let key = format!("{connection_id}/{database}");
        self.tables
            .get(&key)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    pub fn get_columns(&self, connection_id: &str, database: &str, table: &str) -> Vec<ColumnInfo> {
        let key = table_key(connection_id, database, "dbo", table);
        if let Some(cols) = self.columns.get(&key) {
            if !cols.is_empty() {
                return cols.clone();
            }
        }
        let key = table_key(connection_id, database, "public", table);
        self.columns
            .get(&key)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    pub fn get_columns_exact(
        &self,
        connection_id: &str,
        database: &str,
        schema: &str,
        table: &str,
    ) -> Vec<ColumnInfo> {
        let key = table_key(connection_id, database, schema, table);
        self.columns
            .get(&key)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    pub fn invalidate(&self, connection_id: &str) {
        self.schemas.remove(connection_id);
        self.tables.retain(|k, _| !k.starts_with(connection_id));
        self.columns.retain(|k, _| !k.starts_with(connection_id));
    }

    pub fn has(&self, connection_id: &str) -> bool {
        self.schemas.contains_key(connection_id)
    }
}

fn table_key(connection_id: &str, database: &str, schema: &str, table: &str) -> String {
    format!("{connection_id}/{database}/{schema}/{table}")
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

        let tables = cache.get_tables("conn1", "testdb");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "users");

        let cols = cache.get_columns("conn1", "testdb", "users");
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
        assert!(cache.get_tables("conn1", "testdb").is_empty());
    }
}
