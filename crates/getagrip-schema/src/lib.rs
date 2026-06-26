//! Schema introspection, comparison, and migration for GetAGrip.

pub mod comparison;
pub mod introspection;
pub mod migration;
pub mod snapshot;

pub use comparison::{SchemaChange, SchemaDiff, compare_schemas};
pub use introspection::{
    ColumnSchema, ConstraintSchema, DatabaseSchema, ForeignKeySchema, IndexSchema,
    ProcedureSchema, SchemaIntrospector, SequenceSchema, TableSchema, ViewSchema,
};
pub use migration::{Migration, MigrationStep, generate_migration};
pub use snapshot::{SchemaSnapshot, SnapshotStore};
