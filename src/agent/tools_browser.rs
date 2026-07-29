use crate::agent::tool_trait::{Tool, ToolOutput, ToolImportance};

pub struct BrowserNavigate;

impl Default for BrowserNavigate {
    fn default() -> Self { Self::new() }
}

impl BrowserNavigate {
    pub fn new() -> Self { Self }
}

impl Tool for BrowserNavigate {
    fn name(&self) -> &str { "browser_navigate" }
    fn description(&self) -> &str {
        "Fetch a web page and return its text content. Stub: uses HTTP, future: Lightpanda."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to navigate to" }
            },
            "required": ["url"]
        })
    }
    fn execute(&self, args: &serde_json::Value) -> Result<ToolOutput, String> {
        let url = args["url"].as_str().ok_or("missing 'url'")?;
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("client error: {e}"))?;
        let response = client.get(url).send().map_err(|e| format!("request error: {e}"))?;
        let body = response.text().map_err(|e| format!("body error: {e}"))?;
        let text = if body.len() > 8000 { &body[..8000] } else { &body };
        Ok(ToolOutput::text(text, ToolImportance::Normal))
    }
}
