//! Schema snapshot storage and versioning.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::introspection::DatabaseSchema;

/// A timestamped snapshot of a database schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSnapshot {
    /// Snapshot identifier.
    pub id: String,
    /// Connection profile id.
    pub connection_id: String,
    /// Database name.
    pub database: String,
    /// When the snapshot was taken.
    pub captured_at: DateTime<Utc>,
    /// The schema at that point in time.
    pub schema: DatabaseSchema,
    /// User-supplied label (e.g. "before migration v2").
    pub label: Option<String>,
}

/// In-memory store of schema snapshots.
///
/// In a future phase this will be persisted to disk.
#[derive(Debug, Default)]
pub struct SnapshotStore {
    snapshots: HashMap<String, Vec<SchemaSnapshot>>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
        }
    }

    pub fn save(&mut self, snapshot: SchemaSnapshot) {
        self.snapshots
            .entry(snapshot.connection_id.clone())
            .or_default()
            .push(snapshot);
    }

    pub fn list(&self, connection_id: &str) -> Vec<&SchemaSnapshot> {
        self.snapshots
            .get(connection_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn latest(&self, connection_id: &str) -> Option<&SchemaSnapshot> {
        self.snapshots
            .get(connection_id)
            .and_then(|v| v.last())
    }

    pub fn count(&self) -> usize {
        self.snapshots.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_store_save_and_list() {
        let mut store = SnapshotStore::new();
        let snapshot = SchemaSnapshot {
            id: "s1".into(),
            connection_id: "c1".into(),
            database: "testdb".into(),
            captured_at: Utc::now(),
            schema: DatabaseSchema::new("testdb"),
            label: None,
        };
        store.save(snapshot);
        assert_eq!(store.list("c1").len(), 1);
        assert!(store.latest("c1").is_some());
        assert_eq!(store.count(), 1);
    }
}
