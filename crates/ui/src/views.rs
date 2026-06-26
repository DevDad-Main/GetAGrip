//! View rendering — draws all UI elements.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::*,
};
use tg_core::traits::theme::Theme;
use crate::state::{AppState, FocusedPanel};

/// Convert a theme `Color` enum to ratatui `Color`.
fn theme_color(c: &tg_core::traits::theme::Color) -> Color {
    match c {
        tg_core::traits::theme::Color::Hex(s) => parse_hex_color(s),
        tg_core::traits::theme::Color::Index(i) => Color::Indexed(*i),
        tg_core::traits::theme::Color::Rgb { r, g, b } => Color::Rgb(*r, *g, *b),
    }
}

/// Convert a hex string from semantic colors to ratatui `Color`.
fn hex_color(s: &str) -> Color {
    parse_hex_color(s)
}

fn parse_hex_color(s: &str) -> Color {
    let hex = s.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
        Color::Rgb(r, g, b)
    } else {
        Color::Gray
    }
}

/// The main application layout sections.
pub struct MainLayout {
    pub menu: Rect,
    pub sidebar: Rect,
    pub content: Rect,
    pub status: Rect,
}

/// Editor + Results split layout.
pub struct EditorResultsLayout {
    pub editor: Rect,
    pub results: Rect,
}

/// Compute the main application layout.
#[must_use]
pub fn main_layout(area: Rect, sidebar_width: u16) -> MainLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let content_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(sidebar_width),
            Constraint::Min(0),
        ])
        .split(vertical[1]);

    MainLayout {
        menu: vertical[0],
        sidebar: content_area[0],
        content: content_area[1],
        status: vertical[2],
    }
}

/// Split the content area into editor (top) and results (bottom).
#[must_use]
pub fn editor_results_split(area: Rect, split_pct: u16) -> EditorResultsLayout {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(split_pct),
            Constraint::Percentage(100 - split_pct),
        ])
        .split(area);

    EditorResultsLayout {
        editor: split[0],
        results: split[1],
    }
}

/// Render the sidebar (connections + object explorer).
pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    state.layout_cache.write().explorer_rect = Some((area.x, area.y, area.width, area.height));
    let is_focused = *state.focused_panel.read() == FocusedPanel::Explorer;
    let bg = theme_color(&theme.palette.background);
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.blue);
    let sel_bg = theme_color(&theme.palette.selection);
    let green = theme_color(&theme.palette.green);

    let block = Block::default()
        .title(if is_focused { " Explorer [Alt+3] " } else { " Explorer " })
        .borders(Borders::ALL)
        .style(Style::default().bg(bg).fg(fg))
        .border_style(if is_focused {
            Style::default().fg(accent)
        } else {
            Style::default().fg(dim)
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build tree items from explorer state
    let explorer = state.explorer.read();
    let items = &explorer.items;
    let selected = explorer.selected;

    // If no items, show help
    if items.is_empty() {
        let help_lines: Vec<Line> = vec![
            Line::from(""),
            Line::from(Span::styled("  Ctrl+K → /connect <url>", Style::default().fg(green))),
            Line::from(""),
            Line::from(Span::styled("  e.g.:", Style::default().fg(dim))),
            Line::from(Span::styled("  /connect postgres://user@host/db", Style::default().fg(dim))),
            Line::from(Span::styled("  /connect sqlserver://user@host/db", Style::default().fg(dim))),
            Line::from(""),
            Line::from(Span::styled("  ↓/↑ navigate  Enter expand/connect", Style::default().fg(dim))),
        ];
        frame.render_widget(Paragraph::new(help_lines).style(Style::default().fg(fg)), inner);
        return;
    }

    // Render visible items with scroll
    let visible_height = inner.height as usize;
    let scroll = selected.saturating_sub(visible_height.saturating_sub(3));

    let rendered_items: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, item)| {
            let indent = "  ".repeat(item.depth as usize);
            let expand_icon = match item.kind {
                crate::state::ExplorerItemKind::Connection
                | crate::state::ExplorerItemKind::Database
                | crate::state::ExplorerItemKind::Schema => {
                    if item.expanded { " " } else { " " }
                }
                _ => "  ",
            };
            let icon = match item.kind {
                crate::state::ExplorerItemKind::Connection => " ",
                crate::state::ExplorerItemKind::Database => " ",
                crate::state::ExplorerItemKind::Schema => " ",
                crate::state::ExplorerItemKind::Table => " ",
                crate::state::ExplorerItemKind::View => " ",
                crate::state::ExplorerItemKind::Column => "  ",
                crate::state::ExplorerItemKind::Header => "  ",
            };
            let text = format!("{indent}{expand_icon}{icon} {}", item.label);

            if i == selected {
                Line::from(Span::styled(text, Style::default().fg(fg).bg(sel_bg)))
            } else {
                Line::from(Span::styled(text, Style::default().fg(fg)))
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(rendered_items),
        inner,
    );
}

/// Render the query editor with tabs.
pub fn render_editor(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    state.layout_cache.write().editor_rect = Some((area.x, area.y, area.width, area.height));
    let is_focused = *state.focused_panel.read() == FocusedPanel::Editor;
    let bg = hex_color(&theme.editor.bg);
    let fg = hex_color(&theme.editor.fg);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.blue);

    // Tab bar + Editor
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    // Render tab bar
    let tab_bar = render_tab_bar(state, theme, split[0]);
    frame.render_widget(tab_bar, split[0]);

    // Render editor content
    let title = if is_focused {
        " SQL Editor [FOCUSED] "
    } else {
        " SQL Editor "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(bg).fg(fg))
        .border_style(if is_focused {
            Style::default().fg(accent)
        } else {
            Style::default().fg(dim)
        });

    let text = state.active_tab_state().map(|tab| tab.content.clone()).unwrap_or_default();
    let content = if text.is_empty() {
        vec![
            Line::from(Span::styled(
                "  Write your SQL here...",
                Style::default().fg(dim),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Ctrl+Enter to execute   |   Ctrl+K for commands   |   Ctrl+1/2/3 to switch panes",
                Style::default().fg(dim),
            )),
        ]
    } else {
        text
            .lines()
            .map(|line| {
                Line::from(Span::styled(
                    if line.is_empty() { " " } else { line },
                    Style::default().fg(fg),
                ))
            })
            .collect()
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .scroll((0, 0));

    frame.render_widget(paragraph, split[1]);
}

/// Render the tab bar.
fn render_tab_bar(state: &AppState, theme: &Theme, area: Rect) -> Paragraph<'static> {
    let active = *state.active_tab.read();
    let mut spans = Vec::new();
    let mut tab_positions: Vec<(u16, u16, tg_core::id::Id<tg_core::id::TabTag>)> = Vec::new();
    let tab_bg = hex_color(&theme.semantic.tab_bar_bg);
    let tab_active = hex_color(&theme.semantic.tab_active);
    let tab_inactive = hex_color(&theme.semantic.tab_inactive);
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);

    let mut x_offset = area.x;

    for entry in state.tabs.iter() {
        let id = *entry.key();
        let tab = entry.value();
        let is_active = Some(id) == active;

        let prefix = if tab.dirty { " ● " } else { "   " };
        let title = format!("{prefix}{}  ", tab.title);
        let title_width = title.len() as u16;

        tab_positions.push((x_offset, title_width, id));

        spans.push(Span::styled(
            title,
            if is_active {
                Style::default().fg(fg).bg(tab_active)
            } else {
                Style::default().fg(dim).bg(tab_inactive)
            },
        ));
        spans.push(Span::raw(" "));

        x_offset += title_width + 1;
    }

    state.layout_cache.write().tabs = tab_positions;

    Paragraph::new(Line::from(spans))
        .style(Style::default().bg(tab_bg))
}

/// Render the results grid.
pub fn render_results(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    state.layout_cache.write().results_rect = Some((area.x, area.y, area.width, area.height));
    let is_focused = *state.focused_panel.read() == FocusedPanel::Results;
    let bg = hex_color(&theme.grid.bg);
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.blue);

    let block = Block::default()
        .title(if is_focused {
            " Results [FOCUSED] "
        } else {
            " Results "
        })
        .borders(Borders::ALL)
        .style(Style::default().bg(bg).fg(fg))
        .border_style(if is_focused {
            Style::default().fg(accent)
        } else {
            Style::default().fg(dim)
        });

    let message = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  No results yet. Execute a query to see results here.",
            Style::default().fg(dim),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Features: virtual scrolling | sorting | filtering | CSV/JSON/Parquet export",
            Style::default().fg(dim),
        )),
    ])
    .block(block)
    .alignment(Alignment::Left);

    frame.render_widget(message, area);
}

/// Render the menu bar at the top.
pub fn render_menu_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let bg = hex_color(&theme.semantic.panel_bg);
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.blue);
    let menu = state.menu_open.read();
    let open_idx = *menu;

    let items = [" File ", " Edit ", " View ", " Help "];
    let mut spans: Vec<Span> = Vec::new();
    let mut positions: Vec<(u16, u16)> = Vec::new();
    let mut x = area.x;

    for (i, item) in items.iter().enumerate() {
        let w = item.len() as u16;
        positions.push((x, w));
        let style = if Some(i) == open_idx {
            Style::default().fg(fg).bg(accent)
        } else {
            Style::default().fg(dim)
        };
        spans.push(Span::styled(*item, style));
        x += w;
    }

    state.layout_cache.write().menu_positions = positions;
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(bg)),
        area,
    );
}

/// Render menu dropdown as an overlay (rendered AFTER main content to stay on top).
pub fn render_menu_dropdown(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let open_idx = match *state.menu_open.read() {
        Some(idx) => idx,
        None => {
            state.layout_cache.write().menu_dropdown_rect = None;
            return;
        }
    };

    let bg = hex_color(&theme.semantic.panel_bg);
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.blue);

    let dropdown_items: Vec<&str> = match open_idx {
        0 => vec![" New Connection  Ctrl+K", " Open Settings", "────", " Quit  Ctrl+C"],
        1 => vec![" Undo  Ctrl+Z", " Redo  Ctrl+Y", " Cut", " Copy", " Paste"],
        2 => vec![" Toggle Sidebar  Ctrl+B", " Command Palette  Ctrl+K", " Cycle Theme", " Toggle Vim Mode"],
        3 => vec![" About GetAGrip", " Keybindings Reference"],
        _ => vec![],
    };

    let cache = state.layout_cache.read();
    let dx = cache.menu_positions.get(open_idx).map(|(x, _)| *x).unwrap_or(0);
    drop(cache);

    let dropdown_w = 32u16;
    let dropdown_h = (dropdown_items.len() as u16) + 2;
    let dropdown_rect = Rect::new(dx.min(area.width.saturating_sub(dropdown_w)), 1, dropdown_w, dropdown_h);

    // Cache for mouse hit-testing
    {
        let mut c = state.layout_cache.write();
        c.menu_dropdown_rect = Some((dropdown_rect.x, dropdown_rect.y, dropdown_rect.width, dropdown_rect.height));
        let mut items = Vec::new();
        for (i, s) in dropdown_items.iter().enumerate() {
            items.push((dropdown_rect.y + i as u16, s.to_string()));
        }
        c.menu_dropdown_items = items;
    }

    // Clear area behind dropdown
    frame.render_widget(Clear, dropdown_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(bg))
        .border_style(Style::default().fg(accent));
    let inner = block.inner(dropdown_rect);
    frame.render_widget(block, dropdown_rect);

    let items: Vec<Line> = dropdown_items.iter().map(|s| {
        let is_sep = s.contains('─');
        if is_sep {
            Line::from(Span::styled(format!("  {s}"), Style::default().fg(dim)))
        } else {
            Line::from(Span::styled(format!("  {s}"), Style::default().fg(fg)))
        }
    }).collect();
    frame.render_widget(Paragraph::new(items), inner);
}

/// Render the status bar at the bottom.
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let sb_bg = hex_color(&theme.semantic.status_bar_bg);
    let sb_fg = hex_color(&theme.semantic.status_bar_fg);
    let settings = state.settings.read();

    let left = format!(
        " {} | Keybindings: {:?} | {} | Tab {}",
        env!("CARGO_PKG_VERSION"),
        settings.keybindings.mode,
        "NORMAL",
        state.tabs.len()
    );

    let right = format!(
        "[{}] {} {}  ",
        settings.theme.active,
        state.connection_manager.supported_kinds().len(),
        if state.connection_manager.supported_kinds().len() == 1 {
            "driver"
        } else {
            "drivers"
        }
    );

    let left_len = left.len() as u16;
    let right_len = right.len() as u16;

    let spans = Line::from(vec![
        Span::styled(left, Style::default().fg(sb_fg)),
        Span::raw(" ".repeat(
            area.width.saturating_sub(left_len + right_len) as usize
        )),
        Span::styled(right, Style::default().fg(sb_fg)),
    ]);

    let bar = Paragraph::new(spans)
        .style(Style::default().bg(sb_bg));

    frame.render_widget(bar, area);
}

/// Render the command palette overlay with fuzzy suggestions.
pub fn render_command_palette(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let query = state.command_palette_query.read().to_lowercase();
    let selected = *state.palette_selected.read();
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.yellow);
    let bg = hex_color(&theme.semantic.panel_bg);
    let border_color = hex_color(&theme.semantic.panel_border_active);
    let sel_bg = theme_color(&theme.palette.selection);

    // Dim the entire screen
    for y in 0..area.height {
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(Color::Black)),
            Rect::new(0, y, area.width, 1),
        );
    }

    let commands: Vec<(&str, &str)> = vec![
        ("/connect",        "Add a new database connection"),
        ("Switch Theme",    "Cycle through color themes"),
        ("New Tab",         "Open a new query tab"),
        ("Close Tab",       "Close the current tab"),
        ("Format SQL",      "Format the current query"),
        ("Toggle Vim Mode", "Enable/disable Vim keys"),
        ("Execute Query",   "Run the current SQL"),
        ("Explain Query",   "Show execution plan"),
        ("help",            "Show available commands"),
    ];

    let filtered: Vec<&(&str, &str)> = if query.is_empty() {
        commands.iter().collect()
    } else {
        commands.iter().filter(|(name, desc)| {
            name.to_lowercase().contains(&query) || desc.to_lowercase().contains(&query)
        }).collect()
    };

    let max_items = (area.height / 2).min(12) as usize;
    let visible = filtered.len().min(max_items);

    let palette_h = (visible + 4).min(16) as u16;
    let palette_w = 62u16.min(area.width - 4);
    let palette_x = (area.width - palette_w) / 2;
    let palette_y = (area.height - palette_h) / 3;
    let palette_rect = Rect::new(palette_x, palette_y, palette_w, palette_h);

    // Background fill
    frame.render_widget(Clear, palette_rect);

    let block = Block::default()
        .title("  Search  ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(bg));

    let inner = block.inner(palette_rect);
    frame.render_widget(block, palette_rect);

    // Input line with search icon
    let q = state.command_palette_query.read();
    let input_text = if q.is_empty() {
        Span::styled("Type to search commands...", Style::default().fg(dim))
    } else {
        Span::styled(format!("{q}"), Style::default().fg(fg))
    };
    frame.render_widget(
        Paragraph::new(Line::from(input_text)),
        Rect::new(inner.x + 2, inner.y, inner.width - 2, 1),
    );

    // Separator
    let sep_y = inner.y + 1;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("─".repeat(inner.width as usize - 2), Style::default().fg(dim)))),
        Rect::new(inner.x + 1, sep_y, inner.width - 2, 1),
    );

    // Suggestions list
    let list_y = sep_y + 1;
    let items: Vec<Line> = filtered
        .iter()
        .enumerate()
        .take(max_items)
        .map(|(i, (name, desc))| {
            let prefix = if i == selected { "▶" } else { " " };
            let line = format!(" {prefix} {name:<20} {desc}");
            if i == selected {
                Line::from(Span::styled(line, Style::default().fg(fg).bg(sel_bg)))
            } else {
                Line::from(Span::styled(line, Style::default().fg(fg)))
            }
        })
        .collect();

    let item_count = items.len();
    frame.render_widget(
        Paragraph::new(items),
        Rect::new(inner.x + 1, list_y, inner.width - 2, (item_count as u16).min(inner.height - 3)),
    );

    // Footer hint
    let footer_y = palette_rect.y + palette_h - 1;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " ↑↓ select  Enter confirm  Esc dismiss  Type to filter",
            Style::default().fg(dim),
        ))),
        Rect::new(inner.x + 1, footer_y, inner.width - 2, 1),
    );
}

/// Render the connection dialog (centered URL input form).
pub fn render_connection_dialog(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let input = state.connection_dialog_input.read();
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.cyan);
    let green = theme_color(&theme.palette.green);
    let bg = hex_color(&theme.semantic.panel_bg);
    let border_color = theme_color(&theme.palette.blue);

    // Dim backdrop
    for y in 0..area.height {
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(Color::Black)),
            Rect::new(0, y, area.width, 1),
        );
    }

    let dialog_w = 65u16.min(area.width - 4);
    let dialog_h = 10;
    let dx = (area.width - dialog_w) / 2;
    let dy = (area.height - dialog_h) / 3;
    let dialog_rect = Rect::new(dx, dy, dialog_w, dialog_h);

    frame.render_widget(Clear, dialog_rect);

    let block = Block::default()
        .title("   New Connection ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(bg));

    let inner = block.inner(dialog_rect);
    frame.render_widget(block, dialog_rect);

    let lines: Vec<Line> = vec![
        Line::from(Span::styled("  Database URL:", Style::default().fg(dim))),
        Line::from(""),
        Line::from(Span::styled(format!("  ❯ {input}"), Style::default().fg(fg))),
        Line::from(""),
        Line::from(Span::styled("  Examples:", Style::default().fg(dim))),
        Line::from(Span::styled("    postgres://user:pass@host:5432/db", Style::default().fg(dim))),
        Line::from(Span::styled("    mysql://user:pass@host:3306/db", Style::default().fg(dim))),
        Line::from(Span::styled("    sqlserver://user:pass@host:1433/db", Style::default().fg(dim))),
        Line::from(Span::styled("    sqlite:/path/to/file.db", Style::default().fg(dim))),
        Line::from(""),
        Line::from(Span::styled("  Enter to connect, Esc to cancel", Style::default().fg(green))),
    ];

    frame.render_widget(Paragraph::new(lines), Rect::new(inner.x + 1, inner.y, inner.width - 2, dialog_h - 2));
}

/// Render a notification toast.
pub fn render_notification(frame: &mut Frame, area: Rect, state: &AppState, _theme: &Theme) {
    if let Some(ref notif) = *state.notification.read() {
        let msg_width = notif.message.len() as u16 + 4;
        let x = area.width.saturating_sub(msg_width + 2);
        let y = area.height.saturating_sub(2);

        let color = match notif.level {
            crate::state::NotificationLevel::Info => Color::Blue,
            crate::state::NotificationLevel::Success => Color::Green,
            crate::state::NotificationLevel::Warning => Color::Yellow,
            crate::state::NotificationLevel::Error => Color::Red,
        };

        let span = Span::styled(
            format!(" {} ", notif.message),
            Style::default().fg(Color::Black).bg(color),
        );

        frame.render_widget(
            Paragraph::new(span),
            Rect::new(x, y, msg_width, 1),
        );
    }
}
