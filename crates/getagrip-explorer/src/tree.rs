//! Explorer tree data model.
//!
//! Represents the hierarchical structure shown in the Connection Explorer
//! sidebar: Servers → Databases → Schemas → Tables/Views/etc.

use serde::{Deserialize, Serialize};

/// Kinds of nodes that can appear in the explorer tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExplorerNodeKind {
    /// A connection server.
    Server,
    /// A database within a server.
    Database,
    /// A schema/namespace.
    Schema,
    /// A folder grouping related items.
    Folder,
    /// A table.
    Table,
    /// A view.
    View,
    /// A materialized view.
    MaterializedView,
    /// An index.
    Index,
    /// A function.
    Function,
    /// A stored procedure.
    Procedure,
    /// A sequence.
    Sequence,
    /// A column within a table.
    Column,
    /// A constraint.
    Constraint,
    /// An extension.
    Extension,
    /// A role/user.
    Role,
    /// A partition.
    Partition,
}

/// A single node in the explorer tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerNode {
    /// Stable identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Node kind.
    pub kind: ExplorerNodeKind,
    /// Whether the node is expanded in the tree.
    pub expanded: bool,
    /// Whether children have been loaded (lazy loading).
    pub children_loaded: bool,
    /// Children of this node.
    pub children: Vec<ExplorerNode>,
    /// Icon override (e.g. for environment-coloured connections).
    pub icon: Option<String>,
    /// Whether this node is a favorite.
    pub favorite: bool,
    /// Tooltip text.
    pub tooltip: Option<String>,
    /// Whether the node is currently loading children.
    pub loading: bool,
    /// Whether the node has an error.
    pub has_error: bool,
}

impl ExplorerNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: ExplorerNodeKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            expanded: false,
            children_loaded: false,
            children: Vec::new(),
            icon: None,
            favorite: false,
            tooltip: None,
            loading: false,
            has_error: false,
        }
    }

    pub fn add_child(&mut self, child: ExplorerNode) {
        self.children.push(child);
    }

    pub fn find(&self, id: &str) -> Option<&ExplorerNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut ExplorerNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn flatten(&self) -> Vec<&ExplorerNode> {
        let mut v = vec![self];
        for child in &self.children {
            v.extend(child.flatten());
        }
        v
    }
}

/// The full explorer tree.
#[derive(Debug, Clone, Default)]
pub struct ExplorerTree {
    pub roots: Vec<ExplorerNode>,
    pub selected_id: Option<String>,
    pub search_filter: Option<String>,
}

impl ExplorerTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find(&self, id: &str) -> Option<&ExplorerNode> {
        for root in &self.roots {
            if let Some(found) = root.find(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut ExplorerNode> {
        for root in &mut self.roots {
            if let Some(found) = root.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn node_count(&self) -> usize {
        self.roots.iter().map(|r| r.flatten().len()).sum()
    }

    pub fn select(&mut self, id: &str) {
        self.selected_id = Some(id.to_owned());
    }

    pub fn clear_selection(&mut self) {
        self.selected_id = None;
    }

    pub fn add_root(&mut self, node: ExplorerNode) {
        self.roots.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_find_and_count() {
        let mut tree = ExplorerTree::new();
        let mut root = ExplorerNode::new("s1", "localhost", ExplorerNodeKind::Server);
        let db = ExplorerNode::new("db1", "mydb", ExplorerNodeKind::Database);
        root.add_child(db);
        tree.add_root(root);

        assert_eq!(tree.node_count(), 2);
        assert!(tree.find("s1").is_some());
        assert!(tree.find("db1").is_some());
        assert!(tree.find("missing").is_none());
    }

    #[test]
    fn node_children() {
        let mut parent = ExplorerNode::new("p", "parent", ExplorerNodeKind::Folder);
        parent.add_child(ExplorerNode::new("c1", "child1", ExplorerNodeKind::Table));
        parent.add_child(ExplorerNode::new("c2", "child2", ExplorerNodeKind::View));
        assert_eq!(parent.children.len(), 2);
    }
}
