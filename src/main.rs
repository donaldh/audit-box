use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
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

#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
}

struct App {
    files: Vec<FileEntry>,
    list_state: ListState,
    base_path: PathBuf,
}

impl App {
    fn new(overlay_path: &Path, base_path: PathBuf) -> io::Result<Self> {
        let mut files = Vec::new();
        Self::scan_directory(overlay_path, overlay_path, 0, &mut files)?;

        let mut list_state = ListState::default();
        if !files.is_empty() {
            list_state.select(Some(0));
        }

        Ok(App {
            files,
            list_state,
            base_path,
        })
    }

    fn scan_directory(
        base: &Path,
        dir: &Path,
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

            entries.push(FileEntry {
                path: path.clone(),
                name,
                is_dir,
                depth,
                expanded: false,
            });

            if is_dir {
                Self::scan_directory(base, &path, depth + 1, entries)?;
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
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.area());

            let items: Vec<ListItem> = app
                .files
                .iter()
                .map(|entry| {
                    let indent = "  ".repeat(entry.depth);
                    let icon = if entry.is_dir { "📁" } else { "📄" };
                    let content = format!("{}{} {}", indent, icon, entry.name);

                    ListItem::new(Line::from(vec![Span::raw(content)]))
                })
                .collect();

            let items = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Overlay Filesystem Contents (↑↓: navigate, q: quit)"),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(items, chunks[0], &mut app.list_state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    _ => {}
                }
            }
        }
    }
}
