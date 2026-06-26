//! Schema introspection: types representing database schema objects
//! and the trait for introspecting them from a live connection.

use serde::{Deserialize, Serialize};

use getagrip_database::driver::ColumnType;

/// A complete snapshot of a database's schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub database_name: String,
    pub tables: Vec<TableSchema>,
    pub views: Vec<ViewSchema>,
    pub indexes: Vec<IndexSchema>,
    pub functions: Vec<ProcedureSchema>,
    pub procedures: Vec<ProcedureSchema>,
    pub sequences: Vec<SequenceSchema>,
}

impl DatabaseSchema {
    pub fn new(database_name: impl Into<String>) -> Self {
        Self {
            database_name: database_name.into(),
            tables: Vec::new(),
            views: Vec::new(),
            indexes: Vec::new(),
            functions: Vec::new(),
            procedures: Vec::new(),
            sequences: Vec::new(),
        }
    }

    pub fn find_table(&self, name: &str) -> Option<&TableSchema> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }
}

/// A table definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub schema: String,
    pub columns: Vec<ColumnSchema>,
    pub constraints: Vec<ConstraintSchema>,
    pub indexes: Vec<IndexSchema>,
    pub comment: Option<String>,
    pub row_count_estimate: Option<u64>,
}

/// A column definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub col_type: ColumnType,
    pub db_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub ordinal: u16,
    pub comment: Option<String>,
}

/// A constraint (primary key, foreign key, unique, check).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintSchema {
    PrimaryKey {
        name: Option<String>,
        columns: Vec<String>,
    },
    ForeignKey(ForeignKeySchema),
    Unique {
        name: Option<String>,
        columns: Vec<String>,
    },
    Check {
        name: Option<String>,
        expression: String,
    },
}

/// A foreign key constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeySchema {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// An index definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSchema {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub is_primary: bool,
    pub method: Option<String>,
}

/// A view definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSchema {
    pub name: String,
    pub schema: String,
    pub definition: Option<String>,
    pub comment: Option<String>,
}

/// A function or stored procedure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureSchema {
    pub name: String,
    pub schema: String,
    pub kind: ProcedureKind,
    pub definition: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcedureKind {
    Function,
    Procedure,
}

/// A sequence definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceSchema {
    pub name: String,
    pub schema: String,
    pub current_value: Option<i64>,
    pub increment: i64,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
}

/// Trait for drivers that support schema introspection.
///
/// Not all drivers will implement this — it's an optional capability
/// advertised via `DriverCapability::INTROSPECTION`.
#[async_trait::async_trait]
pub trait SchemaIntrospector: Send + Sync {
    /// Introspect the full database schema.
    async fn introspect(&self) -> getagrip_core::AtlasResult<DatabaseSchema>;

    /// Introspect a single table by name.
    async fn introspect_table(&self, table: &str) -> getagrip_core::AtlasResult<TableSchema>;

    /// List all table names.
    async fn list_tables(&self) -> getagrip_core::AtlasResult<Vec<String>>;

    /// List all schema names.
    async fn list_schemas(&self) -> getagrip_core::AtlasResult<Vec<String>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_schema_find_table() {
        let mut schema = DatabaseSchema::new("testdb");
        schema.tables.push(TableSchema {
            name: "users".into(),
            schema: "public".into(),
            columns: vec![],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        assert!(schema.find_table("users").is_some());
        assert!(schema.find_table("missing").is_none());
        assert_eq!(schema.table_count(), 1);
    }
}
