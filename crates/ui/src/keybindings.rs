//! Keybinding definitions and management.
//!
//! Defines the default keybinding map and supports Vim, Emacs, and Standard modes.
//! All keybindings are customizable through the config file.

use std::collections::HashMap;

/// A keybinding definition.
#[derive(Clone, Debug)]
pub struct Keybinding {
    /// Human-readable description.
    pub description: &'static str,
    /// The default key sequence for Standard mode.
    pub standard: &'static str,
    /// The default key sequence for Vim Normal mode.
    pub vim_normal: Option<&'static str>,
    /// The default key sequence for Vim Insert mode.
    pub vim_insert: Option<&'static str>,
    /// The default key sequence for Emacs mode.
    pub emacs: Option<&'static str>,
}

/// All available command keybindings.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Command {
    // Global
    Quit,
    CommandPalette,
    SearchEverywhere,
    SaveAll,

    // Editor
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    SaveFile,
    ExecuteQuery,
    ExecuteCurrentStatement,
    ExplainQuery,
    FormatSql,
    ToggleComment,
    Find,
    FindReplace,
    GoToLine,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Indent,
    Outdent,
    DuplicateLine,
    DeleteLine,
    MoveLineUp,
    MoveLineDown,

    // Navigation
    FocusEditor,
    FocusResults,
    FocusSidebar,
    FocusConnections,
    ToggleSidebar,

    // Results
    CopyCell,
    CopyRow,
    CopyColumn,
    ExportResults,
    SortAscending,
    SortDescending,
    FilterColumn,
    NextPage,
    PreviousPage,

    // Connections
    NewConnection,
    Connect,
    Disconnect,
    Reconnect,
    RefreshSchema,
}

/// Get the default keybinding map.
#[must_use]
pub fn default_keybindings() -> HashMap<Command, Keybinding> {
    let mut map = HashMap::new();

    // ── Global ────────────────────────────────────────────────────
    map.insert(
        Command::Quit,
        Keybinding {
            description: "Quit",
            standard: "ctrl+q",
            vim_normal: Some(":q"),
            vim_insert: None,
            emacs: Some("ctrl+x ctrl+c"),
        },
    );
    map.insert(
        Command::CommandPalette,
        Keybinding {
            description: "Command Palette",
            standard: "ctrl+shift+p",
            vim_normal: Some(":"),
            vim_insert: None,
            emacs: Some("alt+x"),
        },
    );
    map.insert(
        Command::SearchEverywhere,
        Keybinding {
            description: "Search Everywhere",
            standard: "ctrl+p",
            vim_normal: Some("<leader>p"),
            vim_insert: None,
            emacs: Some("ctrl+x f"),
        },
    );
    map.insert(
        Command::SaveAll,
        Keybinding {
            description: "Save All",
            standard: "ctrl+shift+s",
            vim_normal: Some(":wa"),
            vim_insert: None,
            emacs: Some("ctrl+x s"),
        },
    );

    // ── Editor ────────────────────────────────────────────────────
    map.insert(
        Command::NewTab,
        Keybinding {
            description: "New Tab",
            standard: "ctrl+t",
            vim_normal: None,
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::CloseTab,
        Keybinding {
            description: "Close Tab",
            standard: "ctrl+w",
            vim_normal: Some(":q"),
            vim_insert: None,
            emacs: Some("ctrl+x k"),
        },
    );
    map.insert(
        Command::NextTab,
        Keybinding {
            description: "Next Tab",
            standard: "ctrl+tab",
            vim_normal: Some("gt"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::PreviousTab,
        Keybinding {
            description: "Previous Tab",
            standard: "ctrl+shift+tab",
            vim_normal: Some("gT"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::SaveFile,
        Keybinding {
            description: "Save",
            standard: "ctrl+s",
            vim_normal: Some(":w"),
            vim_insert: None,
            emacs: Some("ctrl+x ctrl+s"),
        },
    );
    map.insert(
        Command::ExecuteQuery,
        Keybinding {
            description: "Execute Query",
            standard: "ctrl+enter",
            vim_normal: Some("<leader>r"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::ExecuteCurrentStatement,
        Keybinding {
            description: "Execute Current Statement",
            standard: "ctrl+shift+enter",
            vim_normal: Some("<leader>e"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::ExplainQuery,
        Keybinding {
            description: "Explain Query",
            standard: "ctrl+shift+e",
            vim_normal: Some("<leader>ep"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::FormatSql,
        Keybinding {
            description: "Format SQL",
            standard: "ctrl+shift+f",
            vim_normal: Some("="),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::ToggleComment,
        Keybinding {
            description: "Toggle Comment",
            standard: "ctrl+/",
            vim_normal: Some("gc"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::Find,
        Keybinding {
            description: "Find",
            standard: "ctrl+f",
            vim_normal: Some("/"),
            vim_insert: None,
            emacs: Some("ctrl+s"),
        },
    );
    map.insert(
        Command::FindReplace,
        Keybinding {
            description: "Find and Replace",
            standard: "ctrl+h",
            vim_normal: Some(":%s/"),
            vim_insert: None,
            emacs: Some("alt+shift+5"),
        },
    );
    map.insert(
        Command::GoToLine,
        Keybinding {
            description: "Go to Line",
            standard: "ctrl+g",
            vim_normal: Some(":"),
            vim_insert: None,
            emacs: Some("alt+g g"),
        },
    );

    // ── Navigation ────────────────────────────────────────────────
    map.insert(
        Command::FocusEditor,
        Keybinding {
            description: "Focus Editor",
            standard: "ctrl+1",
            vim_normal: Some("<leader>1"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::FocusResults,
        Keybinding {
            description: "Focus Results",
            standard: "ctrl+2",
            vim_normal: Some("<leader>2"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::FocusSidebar,
        Keybinding {
            description: "Focus Sidebar",
            standard: "ctrl+3",
            vim_normal: Some("<leader>3"),
            vim_insert: None,
            emacs: None,
        },
    );
    map.insert(
        Command::ToggleSidebar,
        Keybinding {
            description: "Toggle Sidebar",
            standard: "ctrl+b",
            vim_normal: Some("<leader>b"),
            vim_insert: None,
            emacs: None,
        },
    );

    map
}

/// Get a human-readable key string for a given command in a given mode.
#[must_use]
pub fn key_for_command(
    bindings: &HashMap<Command, Keybinding>,
    command: Command,
    mode: &str,
) -> Option<String> {
    let binding = bindings.get(&command)?;
    match mode {
        "vim_normal" => binding.vim_normal.map(str::to_string),
        "vim_insert" => binding.vim_insert.map(str::to_string),
        "emacs" => binding.emacs.map(str::to_string),
        _ => Some(binding.standard.to_string()),
    }
}
