use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
    active_pane: ActivePane,
    file_content: Vec<String>,
    content_scroll: usize,
    is_diff_view: bool,
}

impl App {
    fn new(overlay_path: &Path, base_path: PathBuf) -> io::Result<Self> {
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
            active_pane: ActivePane::FileList,
            file_content: Vec::new(),
            content_scroll: 0,
            is_diff_view: false,
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(&args.overlay, args.base)?;

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
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
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
