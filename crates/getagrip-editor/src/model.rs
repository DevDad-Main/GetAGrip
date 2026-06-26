//! Editor model: tabs, bookmarks, session management.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The kind of editor tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabKind {
    /// A query editor connected to a database.
    Query,
    /// A scratch file with no connection.
    Scratch,
    /// A file opened from disk.
    File,
    /// A table data viewer.
    DataViewer,
    /// An explain plan viewer.
    ExplainPlan,
    /// A schema designer (ER diagram).
    SchemaDesigner,
}

/// The state of an editor tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabState {
    /// Cursor position (byte offset).
    pub cursor_offset: usize,
    /// First visible line (scroll position).
    pub scroll_line: usize,
    /// Whether the tab is pinned.
    pub pinned: bool,
    /// Whether the tab is sticky (survives "close all").
    pub sticky: bool,
    /// Current selection range (byte offset, length).
    pub selection: Option<(usize, usize)>,
    /// Bookmarks: line number → label.
    pub bookmarks: HashMap<usize, String>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            cursor_offset: 0,
            scroll_line: 0,
            pinned: false,
            sticky: false,
            selection: None,
            bookmarks: HashMap::new(),
        }
    }
}

/// An editor tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTab {
    /// Stable tab id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Tab kind.
    pub kind: TabKind,
    /// Current SQL/text content.
    pub content: String,
    /// Tab state (cursor, scroll, etc.).
    pub state: TabState,
    /// Connection profile id (for query tabs).
    pub connection_id: Option<String>,
    /// Database name within the connection.
    pub database: Option<String>,
    /// File path on disk (for file tabs).
    pub file_path: Option<String>,
    /// Whether there are unsaved changes.
    pub dirty: bool,
    /// When the tab was created.
    pub created_at: DateTime<Utc>,
    /// When the tab was last activated.
    pub last_accessed_at: DateTime<Utc>,
}

impl EditorTab {
    pub fn new(id: impl Into<String>, title: impl Into<String>, kind: TabKind) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            kind,
            content: String::new(),
            state: TabState::default(),
            connection_id: None,
            database: None,
            file_path: None,
            dirty: false,
            created_at: now,
            last_accessed_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_accessed_at = Utc::now();
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

/// The editor model — manages all open tabs.
#[derive(Debug, Clone, Default)]
pub struct EditorModel {
    /// All open tabs, keyed by id.
    pub tabs: HashMap<String, EditorTab>,
    /// Ordered list of tab ids (leftmost first).
    pub tab_order: Vec<String>,
    /// Currently active tab id.
    pub active_tab_id: Option<String>,
}

impl EditorModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_tab(&mut self, tab: EditorTab) -> &EditorTab {
        let id = tab.id.clone();
        self.tab_order.push(id.clone());
        self.tabs.insert(id.clone(), tab);
        self.active_tab_id = Some(id);
        self.active_tab().unwrap()
    }

    pub fn close_tab(&mut self, id: &str) -> Option<EditorTab> {
        self.tab_order.retain(|i| i != id);
        if self.active_tab_id.as_deref() == Some(id) {
            self.active_tab_id = self.tab_order.last().cloned();
        }
        self.tabs.remove(id)
    }

    pub fn activate(&mut self, id: &str) {
        if self.tabs.contains_key(id) {
            self.active_tab_id = Some(id.to_owned());
            if let Some(tab) = self.tabs.get_mut(id) {
                tab.touch();
            }
            // Move to end of order (most recent).
            self.tab_order.retain(|i| i != id);
            self.tab_order.push(id.to_owned());
        }
    }

    pub fn active_tab(&self) -> Option<&EditorTab> {
        self.active_tab_id
            .as_ref()
            .and_then(|id| self.tabs.get(id))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut EditorTab> {
        self.active_tab_id
            .clone()
            .and_then(|id| self.tabs.get_mut(&id))
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn ordered_tabs(&self) -> Vec<&EditorTab> {
        self.tab_order
            .iter()
            .filter_map(|id| self.tabs.get(id))
            .collect()
    }

    pub fn close_all(&mut self) {
        let to_close: Vec<String> = self
            .tabs
            .values()
            .filter(|t| !t.state.sticky)
            .map(|t| t.id.clone())
            .collect();
        for id in to_close {
            self.close_tab(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_activate_tab() {
        let mut model = EditorModel::new();
        let tab = EditorTab::new("t1", "Query 1", TabKind::Query);
        model.add_tab(tab);
        assert_eq!(model.tab_count(), 1);
        assert!(model.active_tab().is_some());
        assert_eq!(model.active_tab().unwrap().title, "Query 1");
    }

    #[test]
    fn close_tab_updates_active() {
        let mut model = EditorModel::new();
        model.add_tab(EditorTab::new("t1", "Tab 1", TabKind::Query));
        model.add_tab(EditorTab::new("t2", "Tab 2", TabKind::Query));
        model.close_tab("t2");
        assert_eq!(model.tab_count(), 1);
        assert_eq!(model.active_tab().unwrap().id, "t1");
    }

    #[test]
    fn set_content_marks_dirty() {
        let mut tab = EditorTab::new("t1", "test", TabKind::Query);
        assert!(!tab.dirty);
        tab.set_content("SELECT 1");
        assert!(tab.dirty);
        tab.mark_clean();
        assert!(!tab.dirty);
    }

    #[test]
    fn sticky_tabs_survive_close_all() {
        let mut model = EditorModel::new();
        let mut sticky = EditorTab::new("s1", "sticky", TabKind::Query);
        sticky.state.sticky = true;
        model.add_tab(sticky);
        model.add_tab(EditorTab::new("t1", "normal", TabKind::Query));
        model.close_all();
        assert_eq!(model.tab_count(), 1);
        assert_eq!(model.ordered_tabs()[0].id, "s1");
    }
}
