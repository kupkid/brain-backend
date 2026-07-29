use crate::agent::tool_trait::{Tool, ToolOutput, ToolImportance};

pub struct FileOps {
    workspace_dir: std::path::PathBuf,
}

impl FileOps {
    pub fn new(workspace_dir: std::path::PathBuf) -> Self {
        Self { workspace_dir }
    }

    fn validate_path(&self, path: &str) -> Result<std::path::PathBuf, String> {
        if path.contains("..") {
            return Err("path traversal rejected".to_string());
        }
        if std::path::Path::new(path).is_absolute() {
            return Err("absolute path rejected".to_string());
        }
        // Resolve workspace_dir to absolute first
        let ws_root = self.workspace_dir.canonicalize().unwrap_or_else(|_| self.workspace_dir.clone());
        let full = ws_root.join(path);
        // For write: file may not exist yet, so just check parent
        let parent = full.parent().unwrap_or(&full);
        let _ = parent.canonicalize().map_err(|e| format!("parent dir error: {e}"))?;
        let canonical = full.canonicalize().unwrap_or(full);
        if !canonical.starts_with(&ws_root) {
            return Err("path escapes workspace".to_string());
        }
        Ok(canonical)
    }
}

pub struct ReadFile { pub inner: FileOps }
pub struct WriteFile { pub inner: FileOps }
pub struct ListDir { pub inner: FileOps }
pub struct GrepFile { pub inner: FileOps }

impl Tool for ReadFile {
    fn name(&self) -> &str { "read_file" }
    fn description(&self) -> &str { "Read a file's contents. Returns content and line count." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Relative path within workspace" }
            },
            "required": ["path"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let path = args["path"].as_str().ok_or("missing 'path'")?;
        let full = self.inner.validate_path(path)?;
        let content = std::fs::read_to_string(&full).map_err(|e| format!("read error: {e}"))?;
        let lines = content.lines().count();
        let result = serde_json::json!({
            "content": content,
            "lines": lines,
            "path": path,
        });
        Ok(ToolOutput::new(result, ToolImportance::Normal))
    }
}

impl Tool for WriteFile {
    fn name(&self) -> &str { "write_file" }
    fn description(&self) -> &str { "Write content to a file. Creates parent directories." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Relative path within workspace" },
                "content": { "type": "string", "description": "File content to write" }
            },
            "required": ["path", "content"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let path = args["path"].as_str().ok_or("missing 'path'")?;
        let content = args["content"].as_str().ok_or("missing 'content'")?;
        let full = self.inner.validate_path(path)?;
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
        }
        std::fs::write(&full, content).map_err(|e| format!("write error: {e}"))?;
        let result = serde_json::json!({
            "path": path,
            "bytes_written": content.len(),
        });
        Ok(ToolOutput::new(result, ToolImportance::Normal))
    }
}

impl Tool for ListDir {
    fn name(&self) -> &str { "list_dir" }
    fn description(&self) -> &str { "List directory contents with file types and sizes." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Relative directory path" }
            },
            "required": ["path"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let path = args["path"].as_str().unwrap_or(".");
        let full = if path == "." {
            self.inner.workspace_dir.clone()
        } else {
            self.inner.validate_path(path)?
        };
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&full).map_err(|e| format!("readdir error: {e}"))? {
            let entry = entry.map_err(|e| format!("entry error: {e}"))?;
            let meta = entry.metadata().map_err(|e| format!("meta error: {e}"))?;
            let kind = if meta.is_dir() { "dir" } else { "file" };
            entries.push(serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "type": kind,
                "size": meta.len(),
            }));
        }
        Ok(ToolOutput::new(serde_json::json!({"entries": entries}), ToolImportance::Normal))
    }
}

impl Tool for GrepFile {
    fn name(&self) -> &str { "grep" }
    fn description(&self) -> &str { "Search file contents by regex pattern. Returns matching lines." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to search" },
                "pattern": { "type": "string", "description": "Regex pattern" }
            },
            "required": ["path", "pattern"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let path = args["path"].as_str().ok_or("missing 'path'")?;
        let pattern = args["pattern"].as_str().ok_or("missing 'pattern'")?;
        let full = self.inner.validate_path(path)?;
        let content = std::fs::read_to_string(&full).map_err(|e| format!("read error: {e}"))?;
        let re = regex::Regex::new(pattern).map_err(|e| format!("invalid regex: {e}"))?;
        let matches: Vec<serde_json::Value> = content.lines()
            .enumerate()
            .filter(|(_, line)| re.is_match(line))
            .map(|(i, line)| serde_json::json!({"line": i + 1, "text": line}))
            .collect();
        Ok(ToolOutput::new(serde_json::json!({"matches": matches}), ToolImportance::Normal))
    }
}
