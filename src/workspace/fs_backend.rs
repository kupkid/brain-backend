use std::path::PathBuf;

use super::{FileEntry, WorkspaceBackend, WorkspaceError};

pub struct FsWorkspaceBackend {
    base_dir: PathBuf,
}

impl FsWorkspaceBackend {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn uuid_hex(&self, project_uuid: &[u8]) -> String {
        hex::encode(project_uuid)
    }
}

impl WorkspaceBackend for FsWorkspaceBackend {
    fn workspace_root(&self, project_uuid: &[u8]) -> PathBuf {
        self.base_dir
            .join("projects")
            .join(self.uuid_hex(project_uuid))
            .join("workspace")
    }

    fn read_file(&self, project_uuid: &[u8], relative_path: &str) -> Result<Vec<u8>, WorkspaceError> {
        let full_path = self.validate_path(project_uuid, relative_path)?;
        Ok(std::fs::read(&full_path)?)
    }

    fn write_file(&self, project_uuid: &[u8], relative_path: &str, content: &[u8]) -> Result<(), WorkspaceError> {
        let full_path = self.validate_path(project_uuid, relative_path)?;

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&full_path, content)?;
        Ok(())
    }

    fn list_dir(&self, project_uuid: &[u8], relative_path: &str) -> Result<Vec<FileEntry>, WorkspaceError> {
        let full_path = if relative_path.is_empty() {
            self.workspace_root(project_uuid)
        } else {
            self.validate_path(project_uuid, relative_path)?
        };

        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&full_path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path();
            let relative = path.strip_prefix(self.workspace_root(project_uuid))
                .unwrap_or(&path)
                .to_path_buf();

            entries.push(FileEntry {
                path: relative,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata.modified()
                    .ok()
                    .map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.to_rfc3339()
                    }),
            });
        }

        Ok(entries)
    }

    fn exists(&self, project_uuid: &[u8], relative_path: &str) -> Result<bool, WorkspaceError> {
        let full_path = self.validate_path(project_uuid, relative_path)?;
        Ok(full_path.exists())
    }
}
