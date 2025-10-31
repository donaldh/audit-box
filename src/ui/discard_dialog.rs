use crate::app::App;
use crate::types::DialogButton;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if !app.show_discard_dialog {
        return;
    }

    if let Some(selected) = app.list_state.selected() {
        if let Some(entry) = app.files.get(selected) {
            // Create centered dialog area
            let area = f.area();
            let dialog_width = area.width.min(60);
            let dialog_height = 10;
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
                .title("Discard File")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));

            f.render_widget(dialog_block, dialog_area);

            // Split dialog into content and buttons
            let dialog_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(3), Constraint::Length(3)])
                .split(dialog_area);

            // Render confirmation message
            let rel_path = entry.path.strip_prefix(&app.overlay_path).unwrap();
            let file_type = if entry.is_dir { "directory" } else { "file" };
            let message = vec![
                Line::from("Are you sure you want to discard this file?"),
                Line::from(""),
                Line::from(format!("  {} {}", file_type, rel_path.display())),
                Line::from(""),
                Line::from(Span::styled(
                    "This action cannot be undone!",
                    Style::default().fg(Color::Red),
                )),
            ];

            let message_paragraph = Paragraph::new(message).wrap(Wrap { trim: false });
            f.render_widget(message_paragraph, dialog_chunks[0]);

            // Render buttons
            let ok_style = if app.dialog_button == DialogButton::Ok {
                Style::default().bg(Color::Red).fg(Color::Black)
            } else {
                Style::default()
            };
            let cancel_style = if app.dialog_button == DialogButton::Cancel {
                Style::default().bg(Color::Green).fg(Color::Black)
            } else {
                Style::default()
            };

            let buttons = Paragraph::new(Line::from(vec![
                Span::raw("   "),
                Span::styled(" Discard ", ok_style),
                Span::raw("   "),
                Span::styled(" Cancel ", cancel_style),
            ]))
            .alignment(Alignment::Center);

            f.render_widget(buttons, dialog_chunks[1]);
        }
    }
}
