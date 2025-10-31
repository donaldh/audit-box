use crate::types::{FileEntry, FileStatus};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::io;
use std::path::Path;

pub fn scan_directory(
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
            status,
            selected: false,
        });

        if is_dir {
            scan_directory(overlay_root, &path, base_root, depth + 1, entries)?;
        }
    }

    Ok(())
}

pub fn generate_diff(entry: &FileEntry, base_path: &Path) -> Vec<String> {
    // Calculate the path in the base filesystem
    let overlay_root = entry.path.ancestors().nth(entry.depth + 1).unwrap_or(&entry.path);
    let rel_path = entry.path.strip_prefix(overlay_root).unwrap_or(&entry.path);
    let base_file = base_path.join(rel_path);

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

pub fn apply_changes(
    selected_files: &[FileEntry],
    overlay_path: &Path,
    base_path: &Path,
) -> io::Result<()> {
    for entry in selected_files {
        let rel_path = entry.path.strip_prefix(overlay_path).unwrap();
        let dest_path = base_path.join(rel_path);

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

pub fn discard_file(path: &Path) -> io::Result<()> {
    if path.is_file() {
        fs::remove_file(path)?;
    } else if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn update_or_add_file(
    files: &mut Vec<FileEntry>,
    path: &Path,
    overlay_path: &Path,
    base_path: &Path,
) -> io::Result<()> {
    let rel_path = path.strip_prefix(overlay_path).unwrap_or(path);
    let base_file = base_path.join(rel_path);

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
        status,
        selected: false,
    };

    // Find if the file already exists in the list
    if let Some(idx) = files.iter().position(|e| e.path == *path) {
        // Update existing entry, but preserve selection state
        let was_selected = files[idx].selected;
        files[idx] = new_entry;
        files[idx].selected = was_selected;
    } else {
        // Insert new entry in sorted position
        let insert_pos = files
            .iter()
            .position(|e| e.path > *path)
            .unwrap_or(files.len());
        files.insert(insert_pos, new_entry);
    }

    Ok(())
}

pub fn remove_file_from_list(files: &mut Vec<FileEntry>, path: &Path) -> Option<usize> {
    if let Some(idx) = files.iter().position(|e| e.path == *path) {
        files.remove(idx);
        Some(idx)
    } else {
        None
    }
}
