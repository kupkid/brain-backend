use std::process::Command;
use crate::agent::tool_trait::{Tool, ToolOutput, ToolImportance};

pub struct ShellExec {
    workspace_dir: std::path::PathBuf,
    #[allow(dead_code)]
    timeout_secs: u64,
}

impl ShellExec {
    pub fn new(workspace_dir: std::path::PathBuf, timeout_secs: u64) -> Self {
        Self { workspace_dir, timeout_secs }
    }

    const DENY_LIST: &'static [&'static str] = &[
        "rm -rf /", "rm -rf /*", "mkfs", "dd if=", ":(){ :|:&",
        "> /dev/sda", "chmod -R 777 /",
    ];
}

impl Tool for ShellExec {
    fn name(&self) -> &str { "shell_exec" }
    fn description(&self) -> &str {
        "Execute a shell command in the project workspace. Returns stdout, stderr, exit code."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" }
            },
            "required": ["command"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let cmd = args["command"].as_str().ok_or("missing 'command'")?;

        for pattern in Self::DENY_LIST {
            if cmd.contains(pattern) {
                return Err(format!("blocked: command matches deny-list '{pattern}'"));
            }
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(&self.workspace_dir)
            .output()
            .map_err(|e| format!("exec error: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let killed = !output.status.success() && stdout.is_empty() && stderr.is_empty();

        let stdout_trimmed: &str = if stdout.len() > 2000 { &stdout[..2000] } else { &stdout };
        let stderr_trimmed: &str = if stderr.len() > 1000 { &stderr[..1000] } else { &stderr };

        Ok(ToolOutput::new(serde_json::json!({
            "stdout": stdout_trimmed,
            "stderr": stderr_trimmed,
            "exit_code": output.status.code().unwrap_or(-1),
            "killed": killed,
        }), ToolImportance::Low))
    }
}
