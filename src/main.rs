use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{
    Config, Event as NotifyEvent, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

#[derive(Parser, Debug)]
#[command(name = "audit-box")]
#[command(about = "TUI tool for managing overlay filesystem changes", long_about = None)]
struct Args {
    /// Path to the overlay filesystem directory
    #[arg(long)]
    overlay: PathBuf,

    /// Path to the base filesystem directory
    #[arg(long)]
    base: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
enum FileStatus {
    New,
    Modified,
}

#[derive(Debug, Clone, PartialEq)]
enum ActivePane {
    FileList,
    FileContent,
}

#[derive(Debug, Clone, PartialEq)]
enum DialogButton {
    Ok,
    Cancel,
}

#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
    status: FileStatus,
    selected: bool,
}

struct App {
    files: Vec<FileEntry>,
    list_state: ListState,
    base_path: PathBuf,
    overlay_path: PathBuf,
    active_pane: ActivePane,
    file_content: Vec<String>,
    content_scroll: usize,
    is_diff_view: bool,
    show_confirm_dialog: bool,
    show_discard_dialog: bool,
    dialog_button: DialogButton,
    fs_events: Receiver<Result<NotifyEvent, notify::Error>>,
    pending_updates: Vec<PathBuf>,
}

impl App {
    fn new(overlay_path: &Path, base_path: PathBuf, fs_events: Receiver<Result<NotifyEvent, notify::Error>>) -> io::Result<Self> {
        let mut files = Vec::new();
        Self::scan_directory(overlay_path, overlay_path, &base_path, 0, &mut files)?;

        let mut list_state = ListState::default();
        if !files.is_empty() {
            list_state.select(Some(0));
        }

        let mut app = App {
            files,
            list_state,
            base_path,
            overlay_path: overlay_path.to_path_buf(),
            active_pane: ActivePane::FileList,
            file_content: Vec::new(),
            content_scroll: 0,
            is_diff_view: false,
            show_confirm_dialog: false,
            show_discard_dialog: false,
            dialog_button: DialogButton::Ok,
            fs_events,
            pending_updates: Vec::new(),
        };

        app.load_selected_file_content();
        Ok(app)
    }

    fn scan_directory(
        overlay_root: &Path,
        dir: &Path,
        base_root: &Path,
        depth: usize,
        entries: &mut Vec<FileEntry>,
    ) -> io::Result<()> {
        let mut items: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect();

        items.sort_by_key(|e| e.path());

        for entry in items {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();

            // Calculate relative path from overlay root
            let rel_path = path.strip_prefix(overlay_root).unwrap();
            let base_path = base_root.join(rel_path);

            // Determine status: New if doesn't exist in base, Modified if it exists
            let status = if base_path.exists() {
                FileStatus::Modified
            } else {
                FileStatus::New
            };

            entries.push(FileEntry {
                path: path.clone(),
                name,
                is_dir,
                depth,
                expanded: false,
                status,
                selected: false,
            });

            if is_dir {
                Self::scan_directory(overlay_root, &path, base_root, depth + 1, entries)?;
            }
        }

        Ok(())
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.files.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.load_selected_file_content();
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.files.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.load_selected_file_content();
    }

    fn load_selected_file_content(&mut self) {
        self.content_scroll = 0;
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.files.get(selected).cloned() {
                if !entry.is_dir {
                    match entry.status {
                        FileStatus::New => {
                            // For new files, just show the content
                            self.is_diff_view = false;
                            if let Ok(content) = fs::read_to_string(&entry.path) {
                                self.file_content = content.lines().map(|s| s.to_string()).collect();
                            } else {
                                self.file_content = vec!["<Unable to read file>".to_string()];
                            }
                        }
                        FileStatus::Modified => {
                            // For modified files, generate and show a diff
                            self.is_diff_view = true;
                            self.file_content = self.generate_diff(&entry);
                        }
                    }
                } else {
                    self.is_diff_view = false;
                    self.file_content = vec!["<Directory>".to_string()];
                }
            }
        }
    }

    fn generate_diff(&self, entry: &FileEntry) -> Vec<String> {
        // Calculate the path in the base filesystem
        let overlay_root = entry.path.ancestors().nth(entry.depth + 1).unwrap_or(&entry.path);
        let rel_path = entry.path.strip_prefix(overlay_root).unwrap_or(&entry.path);
        let base_file = self.base_path.join(rel_path);

        // Read both files
        let base_content = fs::read_to_string(&base_file).unwrap_or_default();
        let overlay_content = fs::read_to_string(&entry.path).unwrap_or_default();

        // Generate diff
        let diff = TextDiff::from_lines(&base_content, &overlay_content);

        let mut result = Vec::new();
        result.push(format!("--- {}", base_file.display()));
        result.push(format!("+++ {}", entry.path.display()));
        result.push(String::new());

        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            let line = format!("{}{}", sign, change.value().trim_end());
            result.push(line);
        }

        result
    }

    fn scroll_content_down(&mut self) {
        if self.content_scroll < self.file_content.len().saturating_sub(1) {
            self.content_scroll += 1;
        }
    }

    fn scroll_content_up(&mut self) {
        if self.content_scroll > 0 {
            self.content_scroll -= 1;
        }
    }

    fn toggle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::FileList => ActivePane::FileContent,
            ActivePane::FileContent => ActivePane::FileList,
        };
    }

    fn toggle_selection(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.files.get(selected).cloned() {
                if entry.is_dir {
                    // For directories, toggle the directory itself
                    let new_state = !entry.selected;
                    self.files[selected].selected = new_state;

                    let dir_path = entry.path.clone();
                    let dir_depth = entry.depth;

                    // Apply to all children (both files and directories)
                    for i in (selected + 1)..self.files.len() {
                        let child = &self.files[i];
                        if child.depth <= dir_depth || !child.path.starts_with(&dir_path) {
                            break;
                        }
                        self.files[i].selected = new_state;
                    }
                } else {
                    // For files, toggle and handle parent deselection if needed
                    let new_state = !entry.selected;
                    self.files[selected].selected = new_state;

                    // If deselecting a file, deselect all parent directories
                    if !new_state {
                        let file_path = entry.path.clone();
                        for i in 0..selected {
                            if self.files[i].is_dir && file_path.starts_with(&self.files[i].path) {
                                self.files[i].selected = false;
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_selected_files(&self) -> Vec<FileEntry> {
        self.files
            .iter()
            .filter(|e| e.selected && !e.is_dir)
            .cloned()
            .collect()
    }

    fn apply_changes(&self) -> io::Result<()> {
        let selected = self.get_selected_files();

        for entry in selected {
            let rel_path = entry.path.strip_prefix(&self.overlay_path).unwrap();
            let dest_path = self.base_path.join(rel_path);

            // Create parent directories if needed
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy the file
            fs::copy(&entry.path, &dest_path)?;

            // Verify the copy by comparing file contents
            let source_content = fs::read(&entry.path)?;
            let dest_content = fs::read(&dest_path)?;

            if source_content == dest_content {
                // Files are identical, safe to delete source
                fs::remove_file(&entry.path)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Verification failed for {}", entry.path.display()),
                ));
            }
        }

        Ok(())
    }

    fn refresh_file_list(&mut self) -> io::Result<()> {
        let current_selection = self.list_state.selected();
        let selected_path = current_selection
            .and_then(|i| self.files.get(i))
            .map(|e| e.path.clone());

        // Rescan the overlay directory
        let mut files = Vec::new();
        Self::scan_directory(&self.overlay_path, &self.overlay_path, &self.base_path, 0, &mut files)?;

        // Try to restore selection to the same file
        let new_selection = if let Some(ref path) = selected_path {
            files.iter().position(|e| e.path == *path)
        } else {
            None
        };

        self.files = files;
        if let Some(idx) = new_selection {
            self.list_state.select(Some(idx));
        } else if !self.files.is_empty() {
            self.list_state.select(Some(0));
        }

        self.load_selected_file_content();
        Ok(())
    }

    fn check_fs_events(&mut self) {
        // Check for filesystem events without blocking
        while let Ok(event) = self.fs_events.try_recv() {
            if let Ok(event) = event {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        // Collect all affected paths
                        for path in event.paths {
                            if !self.pending_updates.contains(&path) {
                                self.pending_updates.push(path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn process_pending_updates(&mut self) -> io::Result<()> {
        if self.pending_updates.is_empty() {
            return Ok(());
        }

        let current_selection = self.list_state.selected();
        let selected_path = current_selection
            .and_then(|i| self.files.get(i))
            .map(|e| e.path.clone());

        // Collect paths to process
        let paths: Vec<PathBuf> = self.pending_updates.drain(..).collect();

        for path in paths {
            if path.is_dir() {
                // For directories, refresh the entire list (simpler for now)
                self.refresh_file_list()?;
                return Ok(());
            } else if path.exists() {
                // File exists - update or add it
                self.update_or_add_file(&path)?;
            } else {
                // File was deleted - remove it
                self.remove_file(&path);
            }
        }

        // Restore selection if possible
        if let Some(ref path) = selected_path {
            if let Some(idx) = self.files.iter().position(|e| e.path == *path) {
                self.list_state.select(Some(idx));
            }
        }

        // Reload content if the selected file changed
        self.load_selected_file_content();

        Ok(())
    }

    fn update_or_add_file(&mut self, path: &Path) -> io::Result<()> {
        let rel_path = path.strip_prefix(&self.overlay_path).unwrap_or(path);
        let base_file = self.base_path.join(rel_path);

        let status = if base_file.exists() {
            FileStatus::Modified
        } else {
            FileStatus::New
        };

        let depth = rel_path.components().count() - 1;
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let new_entry = FileEntry {
            path: path.to_path_buf(),
            name,
            is_dir: false,
            depth,
            expanded: false,
            status,
            selected: false,
        };

        // Find if the file already exists in the list
        if let Some(idx) = self.files.iter().position(|e| e.path == *path) {
            // Update existing entry, but preserve selection state
            let was_selected = self.files[idx].selected;
            self.files[idx] = new_entry;
            self.files[idx].selected = was_selected;
        } else {
            // Insert new entry in sorted position
            let insert_pos = self.files
                .iter()
                .position(|e| e.path > *path)
                .unwrap_or(self.files.len());
            self.files.insert(insert_pos, new_entry);
        }

        Ok(())
    }

    fn remove_file(&mut self, path: &Path) {
        if let Some(idx) = self.files.iter().position(|e| e.path == *path) {
            self.files.remove(idx);

            // Adjust selection if needed
            if let Some(selected) = self.list_state.selected() {
                if selected >= self.files.len() && !self.files.is_empty() {
                    self.list_state.select(Some(self.files.len() - 1));
                } else if self.files.is_empty() {
                    self.list_state.select(None);
                }
            }
        }
    }

    fn discard_selected_file(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.files.get(selected) {
                let path = entry.path.clone();

                // Delete the file from overlay filesystem
                if path.is_file() {
                    fs::remove_file(&path)?;
                    // The filesystem watcher will handle updating the UI
                } else if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                    // The filesystem watcher will handle updating the UI
                }
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup filesystem watcher
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(&args.overlay, RecursiveMode::Recursive)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(&args.overlay, args.base, rx)?;

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        // Check for filesystem events and process targeted updates
        app.check_fs_events();
        app.process_pending_updates()?;

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(f.area());

            // File list pane
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

            f.render_stateful_widget(items, chunks[0], &mut app.list_state);

            // File content pane
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

            f.render_widget(paragraph, chunks[1]);

            // Render confirmation dialog if visible
            if app.show_confirm_dialog {
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
                    .constraints([
                        Constraint::Min(3),
                        Constraint::Length(3),
                    ])
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

                let file_paragraph = Paragraph::new(file_list)
                    .wrap(Wrap { trim: false });
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

            // Render discard confirmation dialog if visible
            if app.show_discard_dialog {
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
                            .constraints([
                                Constraint::Min(3),
                                Constraint::Length(3),
                            ])
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

                        let message_paragraph = Paragraph::new(message)
                            .wrap(Wrap { trim: false });
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
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.show_confirm_dialog {
                    // Handle apply dialog navigation
                    match key.code {
                        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                            app.dialog_button = match app.dialog_button {
                                DialogButton::Ok => DialogButton::Cancel,
                                DialogButton::Cancel => DialogButton::Ok,
                            };
                        }
                        KeyCode::Enter => {
                            if app.dialog_button == DialogButton::Ok {
                                if let Err(e) = app.apply_changes() {
                                    eprintln!("Error applying changes: {}", e);
                                }
                            }
                            app.show_confirm_dialog = false;
                            app.dialog_button = DialogButton::Ok;
                        }
                        KeyCode::Esc => {
                            app.show_confirm_dialog = false;
                            app.dialog_button = DialogButton::Ok;
                        }
                        _ => {}
                    }
                } else if app.show_discard_dialog {
                    // Handle discard dialog navigation
                    match key.code {
                        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                            app.dialog_button = match app.dialog_button {
                                DialogButton::Ok => DialogButton::Cancel,
                                DialogButton::Cancel => DialogButton::Ok,
                            };
                        }
                        KeyCode::Enter => {
                            if app.dialog_button == DialogButton::Ok {
                                if let Err(e) = app.discard_selected_file() {
                                    eprintln!("Error discarding file: {}", e);
                                }
                            }
                            app.show_discard_dialog = false;
                            app.dialog_button = DialogButton::Ok;
                        }
                        KeyCode::Esc => {
                            app.show_discard_dialog = false;
                            app.dialog_button = DialogButton::Ok;
                        }
                        _ => {}
                    }
                } else {
                    // Handle normal navigation
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('a') => {
                            app.show_confirm_dialog = true;
                        }
                        KeyCode::Char('k') => {
                            app.show_discard_dialog = true;
                        }
                        KeyCode::Tab => app.toggle_pane(),
                        KeyCode::Char(' ') => {
                            if app.active_pane == ActivePane::FileList {
                                app.toggle_selection();
                            }
                        }
                        KeyCode::Down => {
                            match app.active_pane {
                                ActivePane::FileList => app.next(),
                                ActivePane::FileContent => app.scroll_content_down(),
                            }
                        }
                        KeyCode::Up => {
                            match app.active_pane {
                                ActivePane::FileList => app.previous(),
                                ActivePane::FileContent => app.scroll_content_up(),
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
