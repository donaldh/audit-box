mod app;
mod file_operations;
mod session;
mod types;
mod ui;

use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use types::{ActivePane, DialogButton};

#[derive(Parser, Debug)]
#[command(name = "audit-box")]
#[command(about = "TUI tool for managing overlay filesystem changes", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Create a new audit-box session with temporary overlay directories
    New {
        /// Path to the base filesystem directory (defaults to current directory)
        #[arg(long)]
        base: Option<PathBuf>,
    },
    /// Review and manage overlay filesystem changes
    Review {
        /// Path to the overlay filesystem directory (uses saved session if not specified)
        #[arg(long)]
        overlay: Option<PathBuf>,

        /// Path to the base filesystem directory (uses saved session if not specified)
        #[arg(long)]
        base: Option<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Commands::New { base } => {
            run_new(base)?;
        }
        Commands::Review { overlay, base } => {
            run_review(overlay, base)?;
        }
    }

    Ok(())
}

fn run_new(base: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve base path
    let base_path = base.unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Check if base path exists
    if !base_path.exists() {
        return Err(format!("Base path '{}' does not exist", base_path.display()).into());
    }

    // Create the session directories
    let tmpdir = session::create_session_dir()?;

    // Save the session
    session::save_session(&tmpdir)?;

    println!("Created new audit-box session:");
    println!("  Session directory: {}", tmpdir.display());
    println!("  Overlay directory: {}", tmpdir.join("overlay").display());
    println!("  Work directory: {}", tmpdir.join("work").display());
    println!("  Base filesystem: {}", base_path.display());
    println!();
    println!("You can now use 'audit-box review' to review changes.");
    println!();
    println!("To use this session with bubblewrap:");
    println!("  bwrap --ro-bind / / \\");
    println!("        --tmpfs /tmp \\");
    println!("        --unshare-pid \\");
    println!("        --overlay-src {} \\", base_path.display());
    println!("        --overlay {} {} {} \\",
             tmpdir.join("overlay").display(),
             tmpdir.join("work").display(),
             base_path.display());
    println!("        --dev /dev \\");
    println!("        --new-session \\");
    println!("        /bin/bash");

    Ok(())
}

fn run_review(overlay: Option<PathBuf>, base: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve overlay and base paths
    let (overlay_path, base_path) = match (overlay, base) {
        (Some(overlay), Some(base)) => {
            // Both provided explicitly
            (overlay, base)
        }
        (None, None) => {
            // Load from saved session
            let tmpdir = session::load_session()?;
            let overlay = tmpdir.join("overlay");

            // For now, we'll need base to be provided or use current directory
            // In the future, we might want to save the base path in the session too
            let base = std::env::current_dir()?;

            (overlay, base)
        }
        _ => {
            return Err("Both --overlay and --base must be provided together, or neither (to use saved session)".into());
        }
    };

    // Validate paths exist
    if !overlay_path.exists() {
        return Err(format!("Overlay path '{}' does not exist", overlay_path.display()).into());
    }
    if !base_path.exists() {
        return Err(format!("Base path '{}' does not exist", base_path.display()).into());
    }

    // Setup filesystem watcher
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(&overlay_path, RecursiveMode::Recursive)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(&overlay_path, base_path, rx)?;

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
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(f.area());

            // Render file list pane
            ui::file_list::render(f, app, chunks[0]);

            // Render content viewer pane
            ui::content_viewer::render(f, app, chunks[1]);

            // Render dialogs (if visible)
            ui::apply_dialog::render(f, app);
            ui::discard_dialog::render(f, app);
            ui::help_dialog::render(f, app);
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
                } else if app.show_help_dialog {
                    // Handle help dialog - close on Esc or any key
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('?') => {
                            app.show_help_dialog = false;
                        }
                        _ => {}
                    }
                } else {
                    // Handle normal navigation
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            app.show_help_dialog = true;
                        }
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
                        KeyCode::Down => match app.active_pane {
                            ActivePane::FileList => app.next(),
                            ActivePane::FileContent => app.scroll_content_down(),
                        },
                        KeyCode::Up => match app.active_pane {
                            ActivePane::FileList => app.previous(),
                            ActivePane::FileContent => app.scroll_content_up(),
                        },
                        KeyCode::Home => {
                            if app.active_pane == ActivePane::FileList {
                                app.jump_to_first();
                            }
                        }
                        KeyCode::End => {
                            if app.active_pane == ActivePane::FileList {
                                app.jump_to_last();
                            }
                        }
                        KeyCode::Left => {
                            if app.active_pane == ActivePane::FileList {
                                app.collapse_directory();
                            }
                        }
                        KeyCode::Right => {
                            if app.active_pane == ActivePane::FileList {
                                app.expand_directory();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
