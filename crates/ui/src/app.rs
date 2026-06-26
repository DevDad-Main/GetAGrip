//! Application — main TUI application controller.

use ratatui::prelude::*;
use crate::input::{Event, InputHandler};
use crate::state::{AppState, FocusedPanel};
use crate::views;

/// The main application struct.
pub struct App {
    /// Shared application state.
    state: AppState,
    /// Input handler.
    input: InputHandler,
    /// Whether the app should exit.
    should_quit: bool,
    /// Whether the app should redraw.
    needs_redraw: bool,
}

impl App {
    /// Create a new application instance.
    #[must_use]
    pub fn new(state: AppState) -> Self {
        state.refresh_explorer();
        Self {
            state,
            input: InputHandler::new(),
            should_quit: false,
            needs_redraw: true,
        }
    }

    /// Run the main event loop.
    ///
    /// # Errors
    /// Returns an error if terminal operations fail.
    pub async fn run(
        mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> anyhow::Result<()> {
        let tick_rate = std::time::Duration::from_millis(16); // ~60 FPS

        loop {
            if self.needs_redraw {
                terminal.draw(|frame| self.render(frame))?;
                self.needs_redraw = false;
            }

            let event = self.input.next_event(tick_rate).await?;
            self.handle_event(event);

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Render a single frame.
    fn render(&self, frame: &mut Frame) {
        let theme = self.state.theme_manager.active();
        let area = frame.area();

        // Main layout: sidebar | editor | results
        let layout = views::main_layout(area, *self.state.sidebar_width.read());
        views::render_sidebar(frame, layout.sidebar, &self.state, &theme);
        let editor_layout = views::editor_results_split(layout.content, *self.state.editor_split_pct.read());

        // Query editor
        views::render_editor(frame, editor_layout.editor, &self.state, &theme);

        // Results grid
        views::render_results(frame, editor_layout.results, &self.state, &theme);

        // Status bar
        views::render_status_bar(frame, layout.status, &self.state, &theme);

        // Command palette (overlay)
        if *self.state.command_palette_open.read() {
            views::render_command_palette(frame, area, &self.state, &theme);
        }

        // Notification toast
        views::render_notification(frame, area, &self.state, &theme);
    }

    /// Handle an input event.
    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Quit => {
                self.should_quit = true;
            }
            Event::Key(key_event) => {
                self.handle_key(key_event);
            }
            Event::Mouse(_mouse_event) => {
                // Mouse handling will be implemented in a future phase
            }
            Event::Resize(_, _) => {
                self.needs_redraw = true;
            }
            Event::Tick => {
                self.state.cleanup_notifications();
                self.needs_redraw = true;
            }
            Event::Error => {
                tracing::error!("Input error occurred");
            }
        }
    }

    /// Handle a keyboard event.
    fn handle_key(&mut self, key_event: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key_event.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key_event.modifiers.contains(KeyModifiers::ALT);

        // ── Command palette is open: route all input there ──────────
        if *self.state.command_palette_open.read() {
            self.handle_palette_key(key_event);
            return;
        }

        match key_event.code {
            // ── Global shortcuts ──────────────────────────────────
            KeyCode::Char('c') if ctrl => {
                self.should_quit = true;
            }
            KeyCode::Char('k') if ctrl => { self.toggle_command_palette(); }
            KeyCode::F(1) => { self.toggle_command_palette(); }
            KeyCode::Char('p') if ctrl => { self.toggle_command_palette(); }
            KeyCode::Esc => { /* no-op when palette closed */ }

            KeyCode::Char('b') if ctrl => {
                let mut focus = self.state.focused_panel.write();
                *focus = match *focus {
                    FocusedPanel::Editor | FocusedPanel::Results => FocusedPanel::Explorer,
                    _ => FocusedPanel::Editor,
                };
                self.needs_redraw = true;
            }

            // Pane switching: Alt+1/2/3 (terminals reliably send Alt+digit)
            KeyCode::Char('1') if alt => { *self.state.focused_panel.write() = FocusedPanel::Editor; self.needs_redraw = true; }
            KeyCode::Char('2') if alt => { *self.state.focused_panel.write() = FocusedPanel::Results; self.needs_redraw = true; }
            KeyCode::Char('3') if alt => { *self.state.focused_panel.write() = FocusedPanel::Explorer; self.needs_redraw = true; }

            // Panel resizing: Alt+H/L for sidebar width, Alt+J/K for editor/results split
            KeyCode::Char('h') if alt => {
                let mut w = self.state.sidebar_width.write();
                *w = w.saturating_sub(2).max(10);
                self.needs_redraw = true;
            }
            KeyCode::Char('l') if alt => {
                let mut w = self.state.sidebar_width.write();
                *w = (*w + 2).min(80);
                self.needs_redraw = true;
            }
            KeyCode::Char('j') if alt => {
                let mut p = self.state.editor_split_pct.write();
                *p = p.saturating_sub(5).max(10);
                self.needs_redraw = true;
            }
            KeyCode::Char('k') if alt => {
                let mut p = self.state.editor_split_pct.write();
                *p = (*p + 5).min(90);
                self.needs_redraw = true;
            }

            // Tabs
            KeyCode::Char('t') if ctrl => { self.new_tab(); }
            KeyCode::Char('w') if ctrl => { self.close_tab(); }
            KeyCode::Tab if !shift => { self.next_tab(); }
            KeyCode::BackTab => { self.prev_tab(); }

            // ── Editor shortcuts ──────────────────────────────────
            KeyCode::Enter if ctrl && shift => {
                self.state.notify("Query execution not yet wired up", crate::state::NotificationLevel::Info);
                self.needs_redraw = true;
            }
            KeyCode::Char('s') if ctrl => {
                self.state.notify("Save not yet implemented", crate::state::NotificationLevel::Info);
                self.needs_redraw = true;
            }

            // ── Editor/Explorer input ────────────────────────────
            _ => {
                let focus = *self.state.focused_panel.read();
                if focus == FocusedPanel::Editor {
                    if let KeyCode::Char(ch) = key_event.code {
                        self.insert_char(ch);
                    } else {
                        self.handle_editor_key(key_event);
                    }
                } else if focus == FocusedPanel::Explorer {
                    self.handle_explorer_key(key_event);
                }
            }
        }
    }

    /// Handle keys when the explorer sidebar is focused.
    fn handle_explorer_key(&mut self, key_event: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let mut explorer = self.state.explorer.write();
        let max = explorer.items.len().saturating_sub(1);

        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                explorer.selected = explorer.selected.saturating_sub(1);
                drop(explorer);
                self.needs_redraw = true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if explorer.selected < max {
                    explorer.selected += 1;
                }
                drop(explorer);
                self.needs_redraw = true;
            }
            KeyCode::Enter => {
                let idx = explorer.selected;
                if idx < explorer.items.len() {
                    let item = explorer.items[idx].clone();
                    drop(explorer);
                    self.handle_explorer_enter_item(&item, idx);
                }
            }
            KeyCode::Char(' ') => {
                let idx = explorer.selected;
                if idx < explorer.items.len() {
                    let item = &mut explorer.items[idx];
                    if matches!(item.kind,
                        crate::state::ExplorerItemKind::Connection
                        | crate::state::ExplorerItemKind::Database
                        | crate::state::ExplorerItemKind::Schema
                    ) {
                        item.expanded = !item.expanded;
                    }
                }
                drop(explorer);
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    /// Handle Enter on an explorer item: connect or expand.
    fn handle_explorer_enter_item(&mut self, item: &crate::state::ExplorerItem, idx: usize) {
        let mut explorer = self.state.explorer.write();
        match item.kind {
            crate::state::ExplorerItemKind::Connection => {
                if item.expanded {
                    explorer.items[idx].expanded = false;
                    self.collapse_explorer_node(&mut explorer, idx);
                } else {
                    if let Some(conn_id) = item.connection_id {
                        self.state.notify(
                            &format!("Connecting to {}...", item.label),
                            crate::state::NotificationLevel::Info,
                        );
                        explorer.items[idx].expanded = true;
                        self.expand_connection_node(&mut explorer, idx, conn_id);
                    }
                }
            }
            crate::state::ExplorerItemKind::Schema | crate::state::ExplorerItemKind::Table => {
                if !item.expanded {
                    explorer.items[idx].expanded = true;
                } else {
                    explorer.items[idx].expanded = false;
                    self.collapse_explorer_node(&mut explorer, idx);
                }
            }
            _ => {}
        }
        self.needs_redraw = true;
    }

    fn collapse_explorer_node(&self, explorer: &mut crate::state::ExplorerState, idx: usize) {
        let depth = explorer.items[idx].depth;
        let mut remove_count = 0;
        for i in (idx + 1)..explorer.items.len() {
            if explorer.items[i].depth <= depth {
                break;
            }
            remove_count += 1;
        }
        if remove_count > 0 {
            explorer.items.drain((idx + 1)..=(idx + remove_count));
        }
    }

    fn expand_connection_node(&self, explorer: &mut crate::state::ExplorerState, idx: usize, conn_id: tg_core::types::connection::ConnectionId) {
        let depth = explorer.items[idx].depth + 1;

        let rows: Vec<(String, String)> = if let Ok(conn_info) = self.state.connection_manager.get_connection(conn_id) {
            if conn_info.kind == tg_core::types::connection::DatabaseKind::Sqlite {
                let path = conn_info.database.as_deref().unwrap_or(":memory:");
                read_sqlite_schema(path)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if !rows.is_empty() {
            let insert_at = idx + 1;
            for (i, (name, obj_type)) in rows.iter().enumerate() {
                let kind = if obj_type == "view" {
                    crate::state::ExplorerItemKind::View
                } else {
                    crate::state::ExplorerItemKind::Table
                };
                explorer.items.insert(insert_at + i, crate::state::ExplorerItem {
                    label: name.clone(), depth: depth + 1, expanded: false, kind,
                    connection_id: Some(conn_id),
                    database: self.state.connection_manager.get_connection(conn_id).ok().and_then(|c| c.database.clone()),
                    schema: Some("main".into()),
                    table: Some(name.clone()),
                });
            }
            self.state.notify(&format!("Connected: {} objects loaded", rows.len()), crate::state::NotificationLevel::Success);
            return;
        }

        // Check if this is SQLite and failed
        if let Ok(conn_info) = self.state.connection_manager.get_connection(conn_id) {
            if conn_info.kind == tg_core::types::connection::DatabaseKind::Sqlite {
                self.state.notify("Connection failed", crate::state::NotificationLevel::Error);
                return;
            }
        }

        let insert_at = idx + 1;
        explorer.items.insert(insert_at, crate::state::ExplorerItem {
            label: "driver pending...".into(), depth, expanded: false,
            kind: crate::state::ExplorerItemKind::Column,
            connection_id: None, database: None, schema: None, table: None,
        });
        self.state.notify("Driver not yet implemented", crate::state::NotificationLevel::Warning);
    }

    /// Handle keys when the command palette is open.
    fn handle_palette_key(&mut self, key_event: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key_event.code {
            KeyCode::Esc => {
                *self.state.command_palette_open.write() = false;
                self.needs_redraw = true;
            }
            KeyCode::Up => {
                let mut sel = self.state.palette_selected.write();
                *sel = sel.saturating_sub(1);
                self.needs_redraw = true;
            }
            KeyCode::Down => {
                let mut sel = self.state.palette_selected.write();
                *sel += 1;
                self.needs_redraw = true;
            }
            KeyCode::Enter => {
                let query = self.state.command_palette_query.read().clone();
                *self.state.command_palette_open.write() = false;
                *self.state.palette_selected.write() = 0;
                self.execute_palette_command(&query);
                self.needs_redraw = true;
            }
            KeyCode::Backspace => {
                let mut q = self.state.command_palette_query.write();
                q.pop();
                *self.state.palette_selected.write() = 0;
                self.needs_redraw = true;
            }
            KeyCode::Char(ch) => {
                self.state.command_palette_query.write().push(ch);
                *self.state.palette_selected.write() = 0;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    /// Execute a command entered in the command palette.
    fn execute_palette_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();

        // First check for /connect
        if cmd.starts_with("/connect ") {
            let url = &cmd[9..];
            self.add_connection_from_url(url);
            return;
        }
        if cmd.starts_with("/connect") && cmd.len() <= 9 {
            self.state.notify("Usage: /connect <url>", crate::state::NotificationLevel::Info);
            return;
        }

        // Match by command name
        let lower = cmd.to_lowercase();
        if lower.contains("switch theme") || lower.contains("theme") {
            self.cycle_theme();
        } else if lower.contains("help") || lower == "?" {
            self.state.notify(
                "Commands: /connect <url>, Switch Theme, help",
                crate::state::NotificationLevel::Info,
            );
        } else if lower.contains("new tab") {
            self.new_tab();
        } else if lower.contains("close tab") {
            self.close_tab();
        } else if lower.contains("format") {
            self.state.notify("Format SQL not yet wired", crate::state::NotificationLevel::Info);
        } else if !cmd.is_empty() {
            self.state.notify(
                &format!("Unknown: {cmd}. Try help"),
                crate::state::NotificationLevel::Warning,
            );
        }
    }

    fn cycle_theme(&mut self) {
        let themes = self.state.theme_manager.list_themes();
        let current = self.state.settings.read().theme.active.clone();
        let pos = themes.iter().position(|t| *t == current).unwrap_or(0);
        let next = (pos + 1) % themes.len();
        let name = &themes[next];
        if self.state.theme_manager.set_theme(name).is_ok() {
            self.state.settings.write().theme.active = name.clone();
            self.state.notify(
                &format!("Theme: {name} ({}/{})", next + 1, themes.len()),
                crate::state::NotificationLevel::Success,
            );
        }
        self.needs_redraw = true;
    }

    /// Parse a JDBC-style URL into a ConnectionInfo and add it.
    fn add_connection_from_url(&mut self, url: &str) {
        // Parse: kind://user:pass@host:port/db?params
        let (kind_str, rest) = url.split_once("://").unwrap_or(("postgres", url));
        let kind = match kind_str.to_lowercase().as_str() {
            "postgres" | "postgresql" | "pg" => tg_core::types::connection::DatabaseKind::Postgres,
            "mysql" | "mariadb" => tg_core::types::connection::DatabaseKind::Mysql,
            "sqlite" => tg_core::types::connection::DatabaseKind::Sqlite,
            "duckdb" => tg_core::types::connection::DatabaseKind::DuckDb,
            "sqlserver" | "mssql" | "sql server" => tg_core::types::connection::DatabaseKind::SqlServer,
            "redis" => tg_core::types::connection::DatabaseKind::Redis,
            "clickhouse" => tg_core::types::connection::DatabaseKind::ClickHouse,
            other => tg_core::types::connection::DatabaseKind::Custom(other.to_string()),
        };

        // user:pass@host:port/db?params
        let (auth_host, db_params) = rest.split_once('/').unwrap_or((rest, ""));
        let (user_pass, host_port) = auth_host.split_once('@').unwrap_or(("", auth_host));
        let (user, _password) = user_pass.split_once(':').unwrap_or((user_pass, ""));
        let (host, port_str) = host_port.split_once(':').unwrap_or((host_port, ""));
        let port: u16 = port_str.parse().unwrap_or_else(|_| kind.default_port());
        let (db, _params) = db_params.split_once('?').unwrap_or((db_params, ""));

        let name = format!("{user}@{host}/{db}");
        let mut info = tg_core::types::connection::ConnectionInfo::new(name.clone(), kind, host, port);
        info.database = if db.is_empty() { None } else { Some(db.to_string()) };
        info.user = if user.is_empty() { None } else { Some(user.to_string()) };

        match self.state.connection_manager.add_connection(info) {
            Ok(id) => {
                self.state.notify(
                    &format!("Connection added: {name}"),
                    crate::state::NotificationLevel::Success,
                );
                // Wire connection to active tab
                if let Some(active_id) = *self.state.active_tab.read() {
                    if let Some(mut tab) = self.state.tabs.get_mut(&active_id) {
                        tab.connection_id = Some(id);
                    }
                }
                self.state.refresh_explorer();
            }
            Err(e) => {
                self.state.notify(
                    &format!("Failed to add connection: {e}"),
                    crate::state::NotificationLevel::Error,
                );
            }
        }
        self.needs_redraw = true;
    }

    fn new_tab(&mut self) {
        let tab_id = tg_core::id::Id::new();
        let tab_count = self.state.tabs.len() + 1;
        self.state.tabs.insert(
            tab_id,
            crate::state::TabState {
                title: format!("Query {tab_count}"),
                content: String::new(),
                pinned: false,
                dirty: false,
                connection_id: None,
                cursor: (0, 0),
                scroll: 0,
            },
        );
        *self.state.active_tab.write() = Some(tab_id);
        self.needs_redraw = true;
    }

    fn close_tab(&mut self) {
        let active = *self.state.active_tab.read();
        if let Some(id) = active {
            self.state.tabs.remove(&id);
            let remaining: Vec<_> = self.state.tabs.iter().map(|e| *e.key()).collect();
            *self.state.active_tab.write() = remaining.first().copied();
        }
        self.needs_redraw = true;
    }

    fn next_tab(&mut self) {
        let active = *self.state.active_tab.read();
        if let Some(id) = active {
            let ids: Vec<_> = self.state.tabs.iter().map(|e| *e.key()).collect();
            if let Some(pos) = ids.iter().position(|k| *k == id) {
                let next = (pos + 1) % ids.len();
                *self.state.active_tab.write() = Some(ids[next]);
            }
        }
        self.needs_redraw = true;
    }

    fn prev_tab(&mut self) {
        let active = *self.state.active_tab.read();
        if let Some(id) = active {
            let ids: Vec<_> = self.state.tabs.iter().map(|e| *e.key()).collect();
            if let Some(pos) = ids.iter().position(|k| *k == id) {
                let prev = if pos == 0 { ids.len() - 1 } else { pos - 1 };
                *self.state.active_tab.write() = Some(ids[prev]);
            }
        }
        self.needs_redraw = true;
    }

    fn insert_char(&mut self, ch: char) {
        if let Some(tab) = self.state.active_tab_state() {
            let mut content = tab.content.clone();
            let cursor = tab.cursor;
            if cursor.1 <= content.len() {
                content.insert(cursor.1, ch);
            }
            drop(tab);
            if let Some(active_id) = *self.state.active_tab.read() {
                if let Some(mut tab) = self.state.tabs.get_mut(&active_id) {
                    tab.content = content;
                    tab.cursor.1 = tab.cursor.1.saturating_add(1);
                    tab.dirty = true;
                }
            }
            self.needs_redraw = true;
        }
    }

    fn toggle_command_palette(&mut self) {
        let mut open = self.state.command_palette_open.write();
        *open = !*open;
        if *open {
            self.state.command_palette_query.write().clear();
            *self.state.palette_selected.write() = 0;
        }
        self.needs_redraw = true;
    }

    fn refresh_explorer(&self) {
        let conns = self.state.connection_manager.list_connections().unwrap_or_default();
        let mut explorer = self.state.explorer.write();
        explorer.items.clear();

        if conns.is_empty() {
            return;
        }

        for conn in &conns {
            let depth = 0;
            let status_icon = match conn.status {
                tg_core::types::connection::ConnectionStatus::Connected => "●",
                _ => "○",
            };
            explorer.items.push(crate::state::ExplorerItem {
                label: format!("{status_icon} {}", conn.name),
                depth,
                expanded: false,
                kind: crate::state::ExplorerItemKind::Connection,
                connection_id: Some(conn.id),
                database: conn.database.clone(),
                schema: conn.schema.clone(),
                table: None,
            });
        }

        if explorer.selected >= explorer.items.len() {
            explorer.selected = explorer.items.len().saturating_sub(1);
        }
    }

    /// Handle editor-specific key events.
    fn handle_editor_key(&mut self, key_event: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        let active = *self.state.active_tab.read();
        let Some(id) = active else { return };

        let mut tab = match self.state.tabs.get_mut(&id) {
            Some(t) => t,
            None => return,
        };

        let col = tab.cursor.1;
        let content_len = tab.content.len();

        match key_event.code {
            KeyCode::Backspace => {
                if col > 0 {
                    tab.content.remove(col - 1);
                    tab.cursor.1 = col.saturating_sub(1);
                    tab.dirty = true;
                }
            }
            KeyCode::Delete => {
                if col < content_len {
                    tab.content.remove(col);
                    tab.dirty = true;
                }
            }
            KeyCode::Left => {
                tab.cursor.1 = col.saturating_sub(1);
            }
            KeyCode::Right => {
                tab.cursor.1 = (col + 1).min(content_len);
            }
            KeyCode::Up => {
                tab.cursor.0 = tab.cursor.0.saturating_sub(1);
            }
            KeyCode::Down => {
                tab.cursor.0 = tab.cursor.0.saturating_add(1);
            }
            KeyCode::Home => {
                tab.cursor.1 = 0;
            }
            KeyCode::End => {
                let slice = &tab.content[..col.min(content_len)];
                let line_start = slice.rfind('\n').map_or(0, |p| p + 1);
                let slice2 = &tab.content[line_start..];
                let line_end = slice2.find('\n').map_or(content_len, |p| line_start + p);
                tab.cursor.1 = line_end;
            }
            KeyCode::Enter => {
                tab.content.insert(col, '\n');
                tab.cursor.1 = col + 1;
                tab.cursor.0 += 1;
                tab.dirty = true;
            }
            _ => {}
        }
    }
}

fn read_sqlite_schema(path: &str) -> Vec<(String, String)> {
    let db = match rusqlite::Connection::open(path) {
        Ok(db) => db,
        Err(_) => return Vec::new(),
    };
    let _ = db.execute_batch("PRAGMA journal_mode=WAL;");
    let mut stmt = match db.prepare(
        "SELECT name, type FROM sqlite_master WHERE type IN ('table','view') ORDER BY type, name"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    });
    match rows {
        Ok(r) => r.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}
