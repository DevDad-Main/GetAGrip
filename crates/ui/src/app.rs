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
        let layout = views::main_layout(area);

        // Sidebar (connections + object explorer)
        views::render_sidebar(frame, layout.sidebar, &self.state, &theme);

        // Editor + results vertical split
        let editor_layout = views::editor_results_split(layout.content);

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
        let _alt = key_event.modifiers.contains(KeyModifiers::ALT);

        match key_event.code {
            // ── Global shortcuts ──────────────────────────────────
            KeyCode::Char('c') if ctrl => {
                self.should_quit = true;
            }
            KeyCode::Char('p') if ctrl && shift => {
                // Toggle command palette
                let mut open = self.state.command_palette_open.write();
                *open = !*open;
                if *open {
                    self.state.command_palette_query.write().clear();
                }
                self.needs_redraw = true;
            }
            KeyCode::Esc => {
                // Close command palette if open
                if *self.state.command_palette_open.read() {
                    *self.state.command_palette_open.write() = false;
                    self.needs_redraw = true;
                }
            }
            KeyCode::Char('l') if ctrl && shift => {
                // Toggle sidebar focus
                let mut focus = self.state.focused_panel.write();
                *focus = match *focus {
                    FocusedPanel::Editor | FocusedPanel::Results => FocusedPanel::Explorer,
                    _ => FocusedPanel::Editor,
                };
                self.needs_redraw = true;
            }
            KeyCode::Char('t') if ctrl => {
                // New tab
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
            KeyCode::Char('w') if ctrl => {
                // Close tab
                let active = *self.state.active_tab.read();
                if let Some(id) = active {
                    self.state.tabs.remove(&id);
                    // Switch to another tab
                    let remaining: Vec<_> = self.state.tabs.iter().map(|e| *e.key()).collect();
                    *self.state.active_tab.write() = remaining.first().copied();
                }
                self.needs_redraw = true;
            }
            // ── Editor shortcuts ──────────────────────────────────
            KeyCode::Enter if ctrl && shift => {
                // Execute query
                self.state.notify("Query execution not yet wired up", crate::state::NotificationLevel::Info);
                self.needs_redraw = true;
            }
            KeyCode::Char('s') if ctrl => {
                // Save
                self.state.notify("Save not yet implemented", crate::state::NotificationLevel::Info);
                self.needs_redraw = true;
            }
            // ── Tab navigation ────────────────────────────────────
            KeyCode::Tab if !shift => {
                // Next tab
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
            KeyCode::BackTab => {
                // Previous tab
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
            _ => {
                // Handle editor input when editor is focused
                if *self.state.focused_panel.read() == FocusedPanel::Editor
                    && !*self.state.command_palette_open.read()
                {
                    if let KeyCode::Char(ch) = key_event.code {
                        if let Some(tab) = self.state.active_tab_state() {
                            let mut content = tab.content.clone();
                            let cursor = tab.cursor;
                            if cursor.1 <= content.len() {
                                content.insert(cursor.1, ch);
                            }
                            drop(tab);
                            // Update the tab
                            if let Some(mut tab) = self.state.tabs.get_mut(&self.state.active_tab.read().unwrap()) {
                                tab.content = content;
                                tab.cursor.1 = tab.cursor.1.saturating_add(1);
                                tab.dirty = true;
                            }
                            self.needs_redraw = true;
                        }
                    } else {
                        self.handle_editor_key(key_event);
                    }
                }
            }
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
