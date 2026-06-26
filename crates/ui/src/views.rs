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
pub fn main_layout(area: Rect) -> MainLayout {
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Content area
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    let content_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // Sidebar
            Constraint::Min(0),     // Editor + Results
        ])
        .split(body[0]);

    MainLayout {
        sidebar: content_area[0],
        content: content_area[1],
        status: body[1],
    }
}

/// Split the content area into editor (top) and results (bottom).
#[must_use]
pub fn editor_results_split(area: Rect) -> EditorResultsLayout {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    EditorResultsLayout {
        editor: split[0],
        results: split[1],
    }
}

/// Render the sidebar (connections + object explorer).
pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let is_focused = *state.focused_panel.read() == FocusedPanel::Explorer;
    let bg = theme_color(&theme.palette.background);
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);
    let accent = theme_color(&theme.palette.blue);

    let block = Block::default()
        .title(" Explorer ")
        .borders(Borders::ALL)
        .style(Style::default().bg(bg).fg(fg))
        .border_style(if is_focused {
            Style::default().fg(accent)
        } else {
            Style::default().fg(dim)
        });

    let sidebar_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(block.inner(area));

    // Connections panel
    let conn_block = Block::default()
        .title(" Connections ")
        .borders(Borders::BOTTOM);

    // Build connection list from the connection manager
    let conns = state.connection_manager.list_connections().unwrap_or_default();
    let mut conn_items: Vec<ListItem> = if conns.is_empty() {
        vec![
            "  No connections".into(),
            "".into(),
            "  Press Ctrl+K then type:".into(),
            "  /connect <url>".into(),
            "".into(),
            "  e.g.:".into(),
            "  /connect postgres://user@host/db".into(),
            "  /connect sqlserver://user@host/db".into(),
        ]
    } else {
        conns.iter().map(|c| {
            let status = match c.status {
                tg_core::types::connection::ConnectionStatus::Connected => "●",
                _ => "○",
            };
            format!("  {status} {} ({})", c.name, c.kind).into()
        }).collect()
    };
    let conn_list = List::new(conn_items);

    frame.render_widget(conn_block, sidebar_split[0]);
    frame.render_widget(conn_list, sidebar_split[0].inner(Margin::new(1, 0)));

    // Object explorer tree
    let explorer_block = Block::default()
        .title(" Database Explorer ")
        .borders(Borders::NONE);

    let tree_items: Vec<ListItem> = vec![
        "  Connect to a database".into(),
        "  to browse objects".into(),
    ];
    let tree = List::new(tree_items);

    frame.render_widget(explorer_block, sidebar_split[1]);
    frame.render_widget(tree, sidebar_split[1].inner(Margin::new(1, 0)));

    frame.render_widget(block, area);
}

/// Render the query editor with tabs.
pub fn render_editor(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
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
    let tab_bar = render_tab_bar(state, theme);
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
fn render_tab_bar(state: &AppState, theme: &Theme) -> Paragraph<'static> {
    let active = *state.active_tab.read();
    let mut spans = Vec::new();
    let tab_bg = hex_color(&theme.semantic.tab_bar_bg);
    let tab_active = hex_color(&theme.semantic.tab_active);
    let tab_inactive = hex_color(&theme.semantic.tab_inactive);
    let fg = theme_color(&theme.palette.foreground);
    let dim = theme_color(&theme.palette.bright_black);

    for entry in state.tabs.iter() {
        let id = *entry.key();
        let tab = entry.value();
        let is_active = Some(id) == active;

        let prefix = if tab.dirty { " ● " } else { "   " };
        let title = format!("{prefix}{} ", tab.title);

        spans.push(Span::styled(
            title,
            if is_active {
                Style::default().fg(fg).bg(tab_active)
            } else {
                Style::default().fg(dim).bg(tab_inactive)
            },
        ));
        spans.push(Span::raw(" "));
    }

    Paragraph::new(Line::from(spans))
        .style(Style::default().bg(tab_bg))
}

/// Render the results grid.
pub fn render_results(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
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

/// Render the command palette overlay.
pub fn render_command_palette(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let query = state.command_palette_query.read();
    let bg = hex_color(&theme.semantic.command_palette_bg);
    let fg = theme_color(&theme.palette.foreground);
    let sel = hex_color(&theme.semantic.command_palette_selected);
    let accent = theme_color(&theme.palette.yellow);
    let dim = theme_color(&theme.palette.bright_black);

    // Center the palette
    let palette_width = 60u16.min(area.width - 4);
    let palette_height = 12u16.min(area.height - 4);
    let palette_x = (area.width - palette_width) / 2;
    let palette_y = (area.height - palette_height) / 3;
    let palette_area = Rect::new(palette_x, palette_y, palette_width, palette_height);

    frame.render_widget(Clear, palette_area);

    let block = Block::default()
        .title(" Command Palette (Esc to close) ")
        .borders(Borders::ALL)
        .style(Style::default().bg(bg))
        .border_style(Style::default().fg(accent));

    let prompt = format!("> {query}");
    let cursor_pos = 2 + query.len() as u16;

    let items = vec![
        Line::from(Span::styled(&prompt, Style::default().fg(fg))),
        Line::from(""),
        Line::from(Span::styled("  Type /connect <url> to add a connection", Style::default().fg(accent))),
        Line::from(Span::styled("  e.g. /connect sqlserver://user:pass@host/db", Style::default().fg(dim))),
        Line::from(""),
        Line::from(Span::styled("  Alt+1/2/3    Switch panes (editor/results/explorer)", Style::default().fg(dim))),
        Line::from(Span::styled("  Ctrl+K/F1    Toggle this palette", Style::default().fg(dim))),
        Line::from(Span::styled("  Ctrl+T/W     New/close tab", Style::default().fg(dim))),
        Line::from(Span::styled("  Ctrl+B       Toggle sidebar", Style::default().fg(dim))),
        Line::from(Span::styled("  Ctrl+C       Quit", Style::default().fg(dim))),
    ];

    let paragraph = Paragraph::new(items).block(block);

    frame.render_widget(paragraph, palette_area);

    // Render cursor
    if palette_x + cursor_pos < area.width {
        frame.set_cursor_position((palette_x + cursor_pos + 1, palette_y + 1));
    }
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
