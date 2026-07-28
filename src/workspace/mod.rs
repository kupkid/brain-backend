pub mod fs_backend;

#[allow(unused_imports)] // SCAFFOLD — re-export for future modules
pub use fs_backend::FsWorkspaceBackend;

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)] // SCAFFOLD — temporary until workspace integration
pub enum WorkspaceError {
    #[error("path traversal detected: {0}")]
    PathTraversal(String),

    #[error("absolute path not allowed: {0}")]
    AbsolutePath(String),

    #[error("symlink escape: {0}")]
    SymlinkEscape(String),

    #[error("file not found: {0}")]
    NotFound(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // SCAFFOLD — temporary until workspace integration
pub struct FileEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
}

#[allow(dead_code)] // SCAFFOLD — temporary until workspace integration
pub trait WorkspaceBackend: Send + Sync {
    fn read_file(&self, project_uuid: &[u8], relative_path: &str) -> Result<Vec<u8>, WorkspaceError>;

    fn write_file(&self, project_uuid: &[u8], relative_path: &str, content: &[u8]) -> Result<(), WorkspaceError>;

    fn list_dir(&self, project_uuid: &[u8], relative_path: &str) -> Result<Vec<FileEntry>, WorkspaceError>;

    fn exists(&self, project_uuid: &[u8], relative_path: &str) -> Result<bool, WorkspaceError>;

    fn workspace_root(&self, project_uuid: &[u8]) -> PathBuf;

    /// Canonicalize and validate a path is within workspace
    fn validate_path(&self, project_uuid: &[u8], relative_path: &str) -> Result<PathBuf, WorkspaceError> {
        // Reject absolute paths
        if Path::new(relative_path).is_absolute() {
            return Err(WorkspaceError::AbsolutePath(relative_path.to_string()));
        }

        // Reject .. traversal
        if relative_path.contains("..") {
            return Err(WorkspaceError::PathTraversal(relative_path.to_string()));
        }

        let root = self.workspace_root(project_uuid);
        let full_path = root.join(relative_path);

        // Canonicalize to resolve symlinks
        let canonical = full_path.canonicalize().unwrap_or_else(|_| full_path.clone());

        // Verify path starts with workspace root
        let canonical_root = root.canonicalize().unwrap_or(root);
        if !canonical.starts_with(&canonical_root) {
            return Err(WorkspaceError::SymlinkEscape(relative_path.to_string()));
        }

        Ok(canonical)
    }
}
