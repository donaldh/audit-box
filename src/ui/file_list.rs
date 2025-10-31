use crate::app::App;
use crate::types::{ActivePane, FileStatus};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|entry| {
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_dir { "📁" } else { "📄" };
            let status_indicator = match entry.status {
                FileStatus::New => "[N]",
                FileStatus::Modified => "[M]",
            };
            let status_color = match entry.status {
                FileStatus::New => Color::Green,
                FileStatus::Modified => Color::Yellow,
            };
            let selection_indicator = if entry.selected { "[✓] " } else { "[ ] " };

            let content = vec![
                Span::raw(selection_indicator),
                Span::raw(format!("{}{} ", indent, icon)),
                Span::styled(status_indicator, Style::default().fg(status_color)),
                Span::raw(format!(" {}", entry.name)),
            ];

            ListItem::new(Line::from(content))
        })
        .collect();

    let file_list_border_style = if app.active_pane == ActivePane::FileList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let items = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(file_list_border_style)
                .title("Files [Space: select, Tab: switch, ↑↓: navigate, q: quit]"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(items, area, &mut app.list_state);
}
