use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub output: String,
    pub success: bool,
}

pub struct ToolRegistry {
    tools: Vec<ToolDef>,
}

struct ToolDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
    handler: Box<dyn Fn(&str) -> Result<String, String> + Send + Sync>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register(
        mut self,
        name: &str,
        description: &str,
        parameters: serde_json::Value,
        handler: impl Fn(&str) -> Result<String, String> + Send + Sync + 'static,
    ) -> Self {
        self.tools.push(ToolDef {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
            handler: Box::new(handler),
        });
        self
    }

    pub fn call(&self, name: &str, args: &str) -> Result<String, String> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| format!("unknown tool: {name}"))?;
        (tool.handler)(args)
    }

    pub fn schema(&self) -> serde_json::Value {
        let tools: Vec<serde_json::Value> = self
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        serde_json::json!(tools)
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name.clone()).collect()
    }
}
