use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const SESSION_FILE: &str = ".config/audit-box/sessions";

pub fn get_session_file_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;
    Ok(home.join(SESSION_FILE))
}

pub fn save_session(tmpdir: &Path) -> io::Result<()> {
    let session_path = get_session_file_path()?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = session_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write the tmpdir path to the session file
    let mut file = fs::File::create(&session_path)?;
    writeln!(file, "{}", tmpdir.display())?;

    Ok(())
}

pub fn load_session() -> io::Result<PathBuf> {
    let session_path = get_session_file_path()?;

    if !session_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No active session found. Please run 'audit-box new' to create a new session.",
        ));
    }

    let content = fs::read_to_string(&session_path)?;
    let tmpdir = PathBuf::from(content.trim());

    // Check if the directory still exists
    if !tmpdir.exists() {
        // Clean up the stale session file
        let _ = fs::remove_file(&session_path);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Session directory '{}' no longer exists. Please run 'audit-box new' to create a new session.",
                tmpdir.display()
            ),
        ));
    }

    Ok(tmpdir)
}

pub fn create_session_dir() -> io::Result<PathBuf> {
    // Create a unique temporary directory in /tmp
    let tmpdir = tempfile::Builder::new()
        .prefix("bwrap-overlay-")
        .tempdir_in("/tmp")?;

    // Keep the temp directory (don't delete on drop) and get its path
    #[allow(deprecated)]
    let tmpdir_path = tmpdir.into_path();

    // Create overlay and work subdirectories
    fs::create_dir_all(tmpdir_path.join("overlay"))?;
    fs::create_dir_all(tmpdir_path.join("work"))?;

    Ok(tmpdir_path)
}

#[allow(dead_code)]
pub fn clear_session() -> io::Result<()> {
    let session_path = get_session_file_path()?;
    if session_path.exists() {
        fs::remove_file(&session_path)?;
    }
    Ok(())
}
