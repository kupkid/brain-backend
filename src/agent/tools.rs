use crate::agent::tool_trait::Tool;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct ToolBox {
    tools: Vec<Box<dyn Tool>>,
}

impl Default for ToolBox {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolBox {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn add(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn call(
        &self,
        name: &str,
        args: &serde_json::Value,
    ) -> Result<crate::agent::tool_trait::ToolOutput, String> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| format!("unknown tool: {name}"))?;
        tool.execute(args)
    }

    pub fn schema(&self) -> serde_json::Value {
        let defs: Vec<serde_json::Value> = self
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters(),
                    }
                })
            })
            .collect();
        serde_json::json!(defs)
    }

    pub fn names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }
}

pub fn build_default_tools(
    conn: &Arc<Mutex<Connection>>,
    run_id: i64,
    workspace_dir: std::path::PathBuf,
    timeout_secs: u64,
) -> ToolBox {
    use super::tools_browser::BrowserNavigate;
    use super::tools_file::{FileOps, GrepFile, ListDir, ReadFile, WriteFile};
    use super::tools_shell::ShellExec;
    use super::tools_todo::{TodoCreate, TodoList, TodoUpdate};

    let mut tb = ToolBox::new();
    tb.add(Box::new(ShellExec::new(
        workspace_dir.clone(),
        timeout_secs,
    )));
    tb.add(Box::new(ReadFile {
        inner: FileOps::new(workspace_dir.clone()),
    }));
    tb.add(Box::new(WriteFile {
        inner: FileOps::new(workspace_dir.clone()),
    }));
    tb.add(Box::new(ListDir {
        inner: FileOps::new(workspace_dir.clone()),
    }));
    tb.add(Box::new(GrepFile {
        inner: FileOps::new(workspace_dir),
    }));
    tb.add(Box::new(BrowserNavigate::new()));
    tb.add(Box::new(TodoCreate::new(Arc::clone(conn), run_id)));
    tb.add(Box::new(TodoUpdate::new(Arc::clone(conn), run_id)));
    tb.add(Box::new(TodoList::new(Arc::clone(conn), run_id)));
    tb
}
