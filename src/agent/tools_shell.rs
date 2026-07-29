use crate::agent::tool_trait::{Tool, ToolImportance, ToolOutput};
use std::process::Command;

pub struct ShellExec {
    workspace_dir: std::path::PathBuf,
    #[allow(dead_code)]
    timeout_secs: u64,
    rtk_available: bool,
}

impl ShellExec {
    pub fn new(workspace_dir: std::path::PathBuf, timeout_secs: u64) -> Self {
        let rtk_available = which_rtk();
        Self {
            workspace_dir,
            timeout_secs,
            rtk_available,
        }
    }

    const DENY_LIST: &'static [&'static str] = &[
        "rm -rf /",
        "rm -rf /*",
        "mkfs",
        "dd if=",
        ":(){ :|:&",
        "> /dev/sda",
        "chmod -R 777 /",
    ];

    /// Auto-rewrite command to use rtk for known commands.
    /// Transparent: agent writes `ls`, we run `rtk ls`.
    fn rewrite_with_rtk(&self, cmd: &str) -> String {
        if !self.rtk_available {
            return cmd.to_string();
        }

        let trimmed = cmd.trim();

        // Exact matches (no args or simple flags)
        if trimmed == "ls" || trimmed == "ls -la" || trimmed == "ls -l" || trimmed == "ls -a" {
            return format!("rtk {trimmed}");
        }

        // Prefix matches: rewrite first word if it's a known command
        let known_prefixes = [
            "ls ", "cat ", "head ", "tail ", "grep ", "rg ", "find ", "tree ", "git ", "cargo ",
            "npm ", "pytest ", "go ", "docker ", "kubectl ",
        ];

        for prefix in &known_prefixes {
            if trimmed.starts_with(prefix) {
                return format!("rtk {trimmed}");
            }
        }

        // `read` as a command → rtk read
        if trimmed.starts_with("read ") {
            return format!("rtk {trimmed}");
        }

        // python → python3 (python2 not installed)
        if trimmed == "python" || trimmed.starts_with("python ") {
            return trimmed.replacen("python", "python3", 1);
        }

        cmd.to_string()
    }
}

fn which_rtk() -> bool {
    Command::new("which")
        .arg("rtk")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

impl Tool for ShellExec {
    fn name(&self) -> &str {
        "shell_exec"
    }
    fn description(&self) -> &str {
        "Execute a shell command in the project workspace. Returns stdout, stderr, exit code. Automatically uses rtk for compact output on common commands (ls, cat, grep, git, cargo, etc)."
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

        let rewritten = self.rewrite_with_rtk(cmd);

        let output = Command::new("sh")
            .arg("-c")
            .arg(&rewritten)
            .current_dir(&self.workspace_dir)
            .output()
            .map_err(|e| format!("exec error: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut result = String::new();
        if stdout.len() > 2000 {
            result.push_str(&stdout[..2000]);
            result.push_str("\n... (truncated)");
        } else if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n--- stderr ---\n");
            }
            let s = if stderr.len() > 500 {
                &stderr[..500]
            } else {
                &stderr
            };
            result.push_str(s);
        }
        if result.is_empty() {
            result = format!("exit {}", output.status.code().unwrap_or(-1));
        }
        Ok(ToolOutput::text(&result, ToolImportance::Low))
    }
}
