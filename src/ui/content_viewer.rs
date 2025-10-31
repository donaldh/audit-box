use crate::app::App;
use crate::types::ActivePane;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let content_border_style = if app.active_pane == ActivePane::FileContent {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let content_text: Vec<Line> = app
        .file_content
        .iter()
        .skip(app.content_scroll)
        .map(|line| {
            // Colorize diff lines only when viewing a diff
            if app.is_diff_view {
                if line.starts_with('+') && !line.starts_with("+++") {
                    Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Green)))
                } else if line.starts_with('-') && !line.starts_with("---") {
                    Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Red)))
                } else if line.starts_with("---") || line.starts_with("+++") {
                    Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Cyan)))
                } else {
                    Line::from(line.as_str())
                }
            } else {
                Line::from(line.as_str())
            }
        })
        .collect();

    let paragraph = Paragraph::new(content_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(content_border_style)
                .title("Content [Tab: switch, ↑↓: scroll]"),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
