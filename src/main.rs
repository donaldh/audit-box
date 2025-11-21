mod app;
mod file_operations;
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
    /// Review and manage overlay filesystem changes
    Review {
        /// Path to the overlay filesystem directory
        #[arg(long)]
        overlay: PathBuf,

        /// Path to the base filesystem directory
        #[arg(long)]
        base: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Commands::Review { overlay, base } => {
            run_review(&overlay, base)?;
        }
    }

    Ok(())
}

fn run_review(overlay: &PathBuf, base: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Setup filesystem watcher
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(overlay, RecursiveMode::Recursive)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(overlay, base, rx)?;

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
                        _ => {}
                    }
                }
            }
        }
    }
}
