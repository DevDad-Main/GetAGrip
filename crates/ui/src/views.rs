//! View rendering — draws all UI elements.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::*,
};
use tg_core::traits::theme::Theme;
use crate::state::{AppState, FocusedPanel};

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
pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &AppState, _theme: &Theme) {
    let is_focused = *state.focused_panel.read() == FocusedPanel::Explorer;

    let block = Block::default()
        .title(" Explorer ")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
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

    let conn_items: Vec<ListItem> = vec![
        "  + New Connection...".into(),
        "".into(),
        "  No connections yet.".into(),
        "  Press Ctrl+N to add one.".into(),
    ];
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
pub fn render_editor(frame: &mut Frame, area: Rect, state: &AppState, _theme: &Theme) {
    let is_focused = *state.focused_panel.read() == FocusedPanel::Editor;

    // Tab bar + Editor
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    // Render tab bar
    let tab_bar = render_tab_bar(state);
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
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let text = state.active_tab_state().map(|tab| tab.content.clone()).unwrap_or_default();
    let content = if text.is_empty() {
        vec![
            Line::from(Span::styled(
                "  Write your SQL here...",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Ctrl+Enter to execute   |   Ctrl+Shift+F to format   |   Ctrl+Shift+P for commands",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        text
            .lines()
            .map(|line| {
                Line::from(Span::styled(
                    if line.is_empty() { " " } else { line },
                    Style::default().fg(Color::White),
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
fn render_tab_bar(state: &AppState) -> Paragraph<'static> {
    let active = *state.active_tab.read();
    let mut spans = Vec::new();

    for entry in state.tabs.iter() {
        let id = *entry.key();
        let tab = entry.value();
        let is_active = Some(id) == active;

        let prefix = if tab.dirty { " ● " } else { "   " };
        let title = format!("{prefix}{} ", tab.title);
        let title = if tab.pinned {
            format!("📌{title}")
        } else {
            title
        };

        spans.push(Span::styled(
            title,
            if is_active {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Gray)
            },
        ));
        spans.push(Span::raw(" "));
    }

    Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(24, 24, 37)))
}

/// Render the results grid.
pub fn render_results(frame: &mut Frame, area: Rect, state: &AppState, _theme: &Theme) {
    let is_focused = *state.focused_panel.read() == FocusedPanel::Results;

    let block = Block::default()
        .title(if is_focused {
            " Results [FOCUSED] "
        } else {
            " Results "
        })
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let message = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  No results yet. Execute a query to see results here.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Features: virtual scrolling | sorting | filtering | CSV/JSON/Parquet export",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(block)
    .alignment(Alignment::Left);

    frame.render_widget(message, area);
}

/// Render the status bar at the bottom.
pub fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState, _theme: &Theme) {
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
        Span::styled(left, Style::default().fg(Color::Gray)),
        Span::raw(" ".repeat(
            area.width.saturating_sub(left_len + right_len) as usize
        )),
        Span::styled(right, Style::default().fg(Color::Gray)),
    ]);

    let bar = Paragraph::new(spans)
        .style(Style::default().bg(Color::Rgb(24, 24, 37)));

    frame.render_widget(bar, area);
}

/// Render the command palette overlay.
pub fn render_command_palette(frame: &mut Frame, area: Rect, state: &AppState, _theme: &Theme) {
    let query = state.command_palette_query.read();

    // Center the palette
    let palette_width = 60u16.min(area.width - 4);
    let palette_height = 12u16.min(area.height - 4);
    let palette_x = (area.width - palette_width) / 2;
    let palette_y = (area.height - palette_height) / 3;
    let palette_area = Rect::new(palette_x, palette_y, palette_width, palette_height);

    // Dim background
    frame.render_widget(Clear, palette_area);

    let block = Block::default()
        .title(" Command Palette ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let prompt = format!("> {query}");
    let cursor_pos = 2 + query.len() as u16;

    let items = vec![
        Line::from(Span::styled(
            &prompt,
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  New Connection...          Ctrl+N",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Execute Query              Ctrl+Enter",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Format SQL                 Ctrl+Shift+F",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Toggle Vim Mode            ",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Switch Theme...            ",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Preferences...             ",
            Style::default().fg(Color::DarkGray),
        )),
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
