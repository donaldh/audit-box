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
    let visible_files = app.get_visible_files();

    let items: Vec<ListItem> = visible_files
        .iter()
        .map(|(_, entry)| {
            let indent = "  ".repeat(entry.depth);

            // Directory expand/collapse indicator
            let dir_indicator = if entry.is_dir {
                if entry.collapsed {
                    "▶ "
                } else {
                    "▼ "
                }
            } else {
                "  "
            };

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
                Span::raw(format!("{}{}{} ", indent, dir_indicator, icon)),
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
                .title("Files [Space: select, ←→: collapse/expand, ↑↓: navigate, Tab: switch, q: quit]"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Convert the selected index from the full file list to the visible list position
    let mut render_state = ratatui::widgets::ListState::default();
    if let Some(selected_idx) = app.list_state.selected() {
        // Find the position of the selected index in the visible files
        let visible_position = visible_files
            .iter()
            .position(|(idx, _)| *idx == selected_idx);
        render_state.select(visible_position);
    }

    f.render_stateful_widget(items, area, &mut render_state);
}
