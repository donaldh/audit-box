use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    New,
    Modified,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePane {
    FileList,
    FileContent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogButton {
    Ok,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub status: FileStatus,
    pub selected: bool,
}
