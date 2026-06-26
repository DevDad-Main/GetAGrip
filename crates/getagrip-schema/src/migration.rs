//! Migration generation from schema diffs.

use serde::{Deserialize, Serialize};

use crate::comparison::{ChangeKind, SchemaDiff};

/// A set of migration steps that transform one schema into another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub steps: Vec<MigrationStep>,
    pub description: String,
}

/// A single DDL statement in a migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStep {
    pub sql: String,
    pub description: String,
    pub step_number: u32,
    pub reversible: bool,
}

/// Generate migration SQL from a schema diff.
pub fn generate_migration(diff: &SchemaDiff) -> Migration {
    let mut steps = Vec::new();
    let mut step_num = 1u32;

    for change in &diff.changes {
        let (sql, description, reversible) = match change.kind {
            ChangeKind::Added => {
                if let Some(ref sql) = change.sql {
                    (sql.clone(), change.detail.clone(), true)
                } else {
                    (String::new(), change.detail.clone(), false)
                }
            }
            ChangeKind::Removed => {
                let sql = format!("-- DROP {} {}", object_type_sql(change.object_type), change.object_name);
                (sql, change.detail.clone(), false)
            }
            ChangeKind::Modified => {
                let sql = format!("-- ALTER {} {}", object_type_sql(change.object_type), change.object_name);
                (sql, change.detail.clone(), false)
            }
        };

        if !sql.is_empty() {
            steps.push(MigrationStep {
                sql,
                description,
                step_number: step_num,
                reversible,
            });
            step_num += 1;
        }
    }

    Migration {
        steps,
        description: format!("Migration: {} → {}", diff.from_name, diff.to_name),
    }
}

fn object_type_sql(ot: crate::comparison::ObjectType) -> &'static str {
    match ot {
        crate::comparison::ObjectType::Table => "TABLE",
        crate::comparison::ObjectType::Column => "COLUMN",
        crate::comparison::ObjectType::Index => "INDEX",
        crate::comparison::ObjectType::Constraint => "CONSTRAINT",
        crate::comparison::ObjectType::View => "VIEW",
        crate::comparison::ObjectType::Function => "FUNCTION",
        crate::comparison::ObjectType::Procedure => "PROCEDURE",
        crate::comparison::ObjectType::Sequence => "SEQUENCE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparison::{ObjectType, SchemaChange, SchemaDiff};

    #[test]
    fn generates_migration_steps() {
        let diff = SchemaDiff {
            changes: vec![SchemaChange {
                kind: ChangeKind::Added,
                object_type: ObjectType::Table,
                object_name: "users".into(),
                detail: "Table users added".into(),
                sql: Some("CREATE TABLE users (id INT)".into()),
            }],
            from_name: "a".into(),
            to_name: "b".into(),
        };
        let migration = generate_migration(&diff);
        assert_eq!(migration.steps.len(), 1);
        assert_eq!(migration.steps[0].sql, "CREATE TABLE users (id INT)");
        assert!(migration.steps[0].reversible);
    }
}
