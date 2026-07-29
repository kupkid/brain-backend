use std::sync::Arc;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub run_id: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

impl AgentEvent {
    pub fn todo_update(run_id: i64, task_id: &str, status: &str) -> Self {
        Self {
            run_id,
            event_type: "todo_update".to_string(),
            payload: serde_json::json!({"task_id": task_id, "status": status}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn tool_call(run_id: i64, name: &str, args: &serde_json::Value) -> Self {
        Self {
            run_id,
            event_type: "tool_call".to_string(),
            payload: serde_json::json!({"name": name, "arguments": args}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn tool_result(run_id: i64, name: &str, success: bool, summary: &str) -> Self {
        Self {
            run_id,
            event_type: "tool_result".to_string(),
            payload: serde_json::json!({"name": name, "success": success, "summary": summary}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn state_change(run_id: i64, from: &str, to: &str) -> Self {
        Self {
            run_id,
            event_type: "state_change".to_string(),
            payload: serde_json::json!({"from": from, "to": to}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn message(run_id: i64, content: &str) -> Self {
        Self {
            run_id,
            event_type: "message".to_string(),
            payload: serde_json::json!({"content": content}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn token_update(run_id: i64, tokens: usize) -> Self {
        Self {
            run_id,
            event_type: "token_update".to_string(),
            payload: serde_json::json!({"tokens_used": tokens}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(run_id: i64, msg: &str) -> Self {
        Self {
            run_id,
            event_type: "error".to_string(),
            payload: serde_json::json!({"message": msg}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn done(run_id: i64, summary: &str, tokens: usize) -> Self {
        Self {
            run_id,
            event_type: "done".to_string(),
            payload: serde_json::json!({"summary": summary, "total_tokens": tokens}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: AgentEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.sender.subscribe()
    }

    pub fn subscribe_run(&self, _run_id: i64) -> broadcast::Receiver<AgentEvent> {
        // WebSocket handler filters by run_id in its loop
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self { Self::new(1024) }
}

pub type SharedEventBus = Arc<EventBus>;
