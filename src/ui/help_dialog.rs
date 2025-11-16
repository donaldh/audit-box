use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if !app.show_help_dialog {
        return;
    }

    // Create centered dialog area
    let area = f.area();
    let dialog_width = area.width.min(70);
    let dialog_height = area.height.min(25);
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    // Clear the area and render dialog
    f.render_widget(Clear, dialog_area);

    let dialog_block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(dialog_block, dialog_area);

    // Split dialog into content area
    let dialog_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(1)])
        .split(dialog_area);

    // Create help content
    let help_lines = vec![
        Line::from(vec![
            Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Up/Down      ", Style::default().fg(Color::Green)),
            Span::raw("Navigate files or scroll content"),
        ]),
        Line::from(vec![
            Span::styled("  Tab          ", Style::default().fg(Color::Green)),
            Span::raw("Switch between file list and content panes"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Actions", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Space        ", Style::default().fg(Color::Green)),
            Span::raw("Toggle file/directory selection"),
        ]),
        Line::from(vec![
            Span::styled("  a            ", Style::default().fg(Color::Green)),
            Span::raw("Apply selected changes to base filesystem"),
        ]),
        Line::from(vec![
            Span::styled("  k            ", Style::default().fg(Color::Green)),
            Span::raw("Discard currently selected file"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("General", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  h, ?         ", Style::default().fg(Color::Green)),
            Span::raw("Show this help dialog"),
        ]),
        Line::from(vec![
            Span::styled("  Esc          ", Style::default().fg(Color::Green)),
            Span::raw("Close dialogs"),
        ]),
        Line::from(vec![
            Span::styled("  q            ", Style::default().fg(Color::Green)),
            Span::raw("Quit application"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press Esc to close this dialog", Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)),
        ]),
    ];

    let help_paragraph = Paragraph::new(help_lines)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, dialog_chunks[0]);
}
