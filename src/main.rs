use clap::Parser;
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

}
