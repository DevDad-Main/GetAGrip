//! Schema comparison and diffing.

use serde::{Deserialize, Serialize};

use crate::introspection::DatabaseSchema;

/// A single change detected between two schema snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    pub kind: ChangeKind,
    pub object_type: ObjectType,
    pub object_name: String,
    pub detail: String,
    pub sql: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    Table,
    Column,
    Index,
    Constraint,
    View,
    Function,
    Procedure,
    Sequence,
}

/// The result of comparing two schemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiff {
    pub changes: Vec<SchemaChange>,
    pub from_name: String,
    pub to_name: String,
}

impl SchemaDiff {
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    pub fn added_count(&self) -> usize {
        self.changes.iter().filter(|c| c.kind == ChangeKind::Added).count()
    }

    pub fn removed_count(&self) -> usize {
        self.changes.iter().filter(|c| c.kind == ChangeKind::Removed).count()
    }

    pub fn modified_count(&self) -> usize {
        self.changes.iter().filter(|c| c.kind == ChangeKind::Modified).count()
    }
}

/// Compare two database schemas and produce a diff.
pub fn compare_schemas(from: &DatabaseSchema, to: &DatabaseSchema) -> SchemaDiff {
    let mut changes = Vec::new();

    // Compare tables
    for table in &to.tables {
        match from.find_table(&table.name) {
            None => {
                changes.push(SchemaChange {
                    kind: ChangeKind::Added,
                    object_type: ObjectType::Table,
                    object_name: table.name.clone(),
                    detail: format!("Table '{}' added", table.name),
                    sql: None,
                });
            }
            Some(_old) => {
                // Column-level comparison (simplified for Phase 3)
                for col in &table.columns {
                    if !_old.columns.iter().any(|c| c.name == col.name) {
                        changes.push(SchemaChange {
                            kind: ChangeKind::Added,
                            object_type: ObjectType::Column,
                            object_name: format!("{}.{}", table.name, col.name),
                            detail: format!("Column '{}' added to '{}'", col.name, table.name),
                            sql: None,
                        });
                    }
                }
            }
        }
    }

    for table in &from.tables {
        if to.find_table(&table.name).is_none() {
            changes.push(SchemaChange {
                kind: ChangeKind::Removed,
                object_type: ObjectType::Table,
                object_name: table.name.clone(),
                detail: format!("Table '{}' removed", table.name),
                sql: None,
            });
        }
    }

    SchemaDiff {
        changes,
        from_name: from.database_name.clone(),
        to_name: to.database_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_schemas_produce_no_diff() {
        let a = DatabaseSchema::new("a");
        let b = DatabaseSchema::new("b");
        let diff = compare_schemas(&a, &b);
        assert!(!diff.has_changes());
    }

    #[test]
    fn added_table_is_detected() {
        let from = DatabaseSchema::new("from");
        let mut to = DatabaseSchema::new("to");
        to.tables.push(crate::introspection::TableSchema {
            name: "new_table".into(),
            schema: "public".into(),
            columns: vec![],
            constraints: vec![],
            indexes: vec![],
            comment: None,
            row_count_estimate: None,
        });
        let diff = compare_schemas(&from, &to);
        assert_eq!(diff.added_count(), 1);
        assert!(diff.has_changes());
    }
}
