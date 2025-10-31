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
    if !app.show_confirm_dialog {
        return;
    }

    let selected_files = app.get_selected_files();

    // Create centered dialog area
    let area = f.area();
    let dialog_width = area.width.min(60);
    let dialog_height = (selected_files.len() as u16 + 8).min(area.height - 4);
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
        .title("Apply Changes")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    f.render_widget(dialog_block, dialog_area);

    // Split dialog into content and buttons
    let dialog_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(dialog_area);

    // Render selected files list
    let file_list: Vec<Line> = if selected_files.is_empty() {
        vec![Line::from("No files selected")]
    } else {
        let mut lines = vec![Line::from("The following files will be applied:")];
        lines.push(Line::from(""));
        for file in selected_files.iter() {
            let rel_path = file.path.strip_prefix(&app.overlay_path).unwrap();
            lines.push(Line::from(format!("  • {}", rel_path.display())));
        }
        lines
    };

    let file_paragraph = Paragraph::new(file_list).wrap(Wrap { trim: false });
    f.render_widget(file_paragraph, dialog_chunks[0]);

    // Render buttons
    let ok_style = if app.dialog_button == DialogButton::Ok {
        Style::default().bg(Color::Green).fg(Color::Black)
    } else {
        Style::default()
    };
    let cancel_style = if app.dialog_button == DialogButton::Cancel {
        Style::default().bg(Color::Red).fg(Color::Black)
    } else {
        Style::default()
    };

    let buttons = Paragraph::new(Line::from(vec![
        Span::raw("   "),
        Span::styled(" OK ", ok_style),
        Span::raw("   "),
        Span::styled(" Cancel ", cancel_style),
    ]))
    .alignment(Alignment::Center);

    f.render_widget(buttons, dialog_chunks[1]);
}
