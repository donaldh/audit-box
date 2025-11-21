use crate::file_operations;
use crate::types::{ActivePane, DialogButton, FileEntry, FileStatus};
use notify::Event as NotifyEvent;
use notify::EventKind;
use ratatui::widgets::ListState;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

pub struct App {
    pub files: Vec<FileEntry>,
    pub list_state: ListState,
    pub base_path: PathBuf,
    pub overlay_path: PathBuf,
    pub active_pane: ActivePane,
    pub file_content: Vec<String>,
    pub content_scroll: usize,
    pub is_diff_view: bool,
    pub show_confirm_dialog: bool,
    pub show_discard_dialog: bool,
    pub show_help_dialog: bool,
    pub dialog_button: DialogButton,
    fs_events: Receiver<Result<NotifyEvent, notify::Error>>,
    pending_updates: Vec<PathBuf>,
}

impl App {
    pub fn new(
        overlay_path: &Path,
        base_path: PathBuf,
        fs_events: Receiver<Result<NotifyEvent, notify::Error>>,
    ) -> io::Result<Self> {
        let mut files = Vec::new();
        file_operations::scan_directory(overlay_path, overlay_path, &base_path, 0, &mut files)?;

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
            show_help_dialog: false,
            dialog_button: DialogButton::Ok,
            fs_events,
            pending_updates: Vec::new(),
        };

        app.load_selected_file_content();
        Ok(app)
    }

    pub fn next(&mut self) {
        let visible = self.get_visible_files();
        if visible.is_empty() {
            return;
        }

        let current_idx = self.list_state.selected();
        let next_idx = if let Some(current) = current_idx {
            // Find current position in visible list
            if let Some(pos) = visible.iter().position(|(idx, _)| *idx == current) {
                // Move to next visible item, or wrap to first
                if pos >= visible.len() - 1 {
                    visible[0].0
                } else {
                    visible[pos + 1].0
                }
            } else {
                // Current selection not visible, go to first
                visible[0].0
            }
        } else {
            visible[0].0
        };

        self.list_state.select(Some(next_idx));
        self.load_selected_file_content();
    }

    pub fn previous(&mut self) {
        let visible = self.get_visible_files();
        if visible.is_empty() {
            return;
        }

        let current_idx = self.list_state.selected();
        let prev_idx = if let Some(current) = current_idx {
            // Find current position in visible list
            if let Some(pos) = visible.iter().position(|(idx, _)| *idx == current) {
                // Move to previous visible item, or wrap to last
                if pos == 0 {
                    visible[visible.len() - 1].0
                } else {
                    visible[pos - 1].0
                }
            } else {
                // Current selection not visible, go to first
                visible[0].0
            }
        } else {
            visible[0].0
        };

        self.list_state.select(Some(prev_idx));
        self.load_selected_file_content();
    }

    pub fn load_selected_file_content(&mut self) {
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
                            self.file_content = file_operations::generate_diff(
                                &entry,
                                &self.base_path,
                            );
                        }
                    }
                } else {
                    self.is_diff_view = false;
                    self.file_content = vec!["<Directory>".to_string()];
                }
            }
        }
    }

    pub fn scroll_content_down(&mut self) {
        if self.content_scroll < self.file_content.len().saturating_sub(1) {
            self.content_scroll += 1;
        }
    }

    pub fn scroll_content_up(&mut self) {
        if self.content_scroll > 0 {
            self.content_scroll -= 1;
        }
    }

    pub fn toggle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::FileList => ActivePane::FileContent,
            ActivePane::FileContent => ActivePane::FileList,
        };
    }

    pub fn toggle_selection(&mut self) {
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

    pub fn collapse_directory(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.files.get(selected).cloned() {
                if entry.is_dir && !entry.collapsed {
                    // Collapse the directory
                    self.files[selected].collapsed = true;
                } else if !entry.is_dir || entry.collapsed {
                    // If it's a file or already collapsed, move to parent directory
                    self.move_to_parent();
                }
            }
        }
    }

    pub fn expand_directory(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.files.get(selected).cloned() {
                if entry.is_dir && entry.collapsed {
                    // Expand the directory
                    self.files[selected].collapsed = false;
                }
            }
        }
    }

    fn move_to_parent(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.files.get(selected) {
                let current_path = &entry.path;
                let current_depth = entry.depth;

                // Find the parent directory by looking backwards for a directory with depth-1
                for i in (0..selected).rev() {
                    if self.files[i].is_dir
                        && self.files[i].depth == current_depth.saturating_sub(1)
                        && current_path.starts_with(&self.files[i].path) {
                        self.list_state.select(Some(i));
                        self.load_selected_file_content();
                        break;
                    }
                }
            }
        }
    }

    pub fn get_selected_files(&self) -> Vec<FileEntry> {
        self.files
            .iter()
            .filter(|e| e.selected && !e.is_dir)
            .cloned()
            .collect()
    }

    pub fn get_visible_files(&self) -> Vec<(usize, &FileEntry)> {
        let mut visible = Vec::new();
        let mut collapsed_dirs: Vec<(PathBuf, usize)> = Vec::new();

        for (idx, entry) in self.files.iter().enumerate() {
            // Remove collapsed dirs from stack if we've moved past their depth
            collapsed_dirs.retain(|(_, depth)| entry.depth > *depth);

            // Check if this entry is inside a collapsed directory
            let is_hidden = collapsed_dirs.iter().any(|(dir_path, _)| {
                entry.path.starts_with(dir_path) && entry.path != *dir_path
            });

            if !is_hidden {
                visible.push((idx, entry));

                // If this is a collapsed directory, add it to the stack
                if entry.is_dir && entry.collapsed {
                    collapsed_dirs.push((entry.path.clone(), entry.depth));
                }
            }
        }

        visible
    }

    pub fn apply_changes(&self) -> io::Result<()> {
        let selected = self.get_selected_files();
        file_operations::apply_changes(&selected, &self.overlay_path, &self.base_path)
    }

    pub fn discard_selected_file(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.files.get(selected) {
                let path = entry.path.clone();
                file_operations::discard_file(&path)?;
            }
        }
        Ok(())
    }

    pub fn refresh_file_list(&mut self) -> io::Result<()> {
        let current_selection = self.list_state.selected();
        let selected_path = current_selection
            .and_then(|i| self.files.get(i))
            .map(|e| e.path.clone());

        // Rescan the overlay directory
        let mut files = Vec::new();
        file_operations::scan_directory(
            &self.overlay_path,
            &self.overlay_path,
            &self.base_path,
            0,
            &mut files,
        )?;

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

    pub fn check_fs_events(&mut self) {
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

    pub fn process_pending_updates(&mut self) -> io::Result<()> {
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
                file_operations::update_or_add_file(
                    &mut self.files,
                    &path,
                    &self.overlay_path,
                    &self.base_path,
                )?;
            } else {
                // File was deleted - remove it
                if let Some(_removed_idx) =
                    file_operations::remove_file_from_list(&mut self.files, &path)
                {
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
}
