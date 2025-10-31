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
}

struct App {
    files: Vec<FileEntry>,
    list_state: ListState,
    base_path: PathBuf,
    active_pane: ActivePane,
    file_content: Vec<String>,
    content_scroll: usize,
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
            if let Some(entry) = self.files.get(selected) {
                if !entry.is_dir {
                    if let Ok(content) = fs::read_to_string(&entry.path) {
                        self.file_content = content.lines().map(|s| s.to_string()).collect();
                    } else {
                        self.file_content = vec!["<Unable to read file>".to_string()];
                    }
                } else {
                    self.file_content = vec!["<Directory>".to_string()];
                }
            }
        }
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

                    let content = vec![
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
                        .title("Files [Tab: switch, ↑↓: navigate, q: quit]"),
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
                .map(|line| Line::from(line.as_str()))
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
