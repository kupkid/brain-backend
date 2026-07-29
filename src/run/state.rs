use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse_status(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum TransitionError {
    #[error("invalid transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },

    #[error("missing summary for completed run")]
    MissingSummary,

    #[error("missing error message for failed run")]
    MissingError,

    #[error("missing cancellation reason")]
    MissingCancelReason,

    #[error("pending tool invocations: {count} still running")]
    PendingTools { count: usize },

    #[error("run not found")]
    NotFound,
}

pub struct RunStateMachine;

#[derive(Debug)]
pub struct Transition {
    pub from: RunStatus,
    pub to: RunStatus,
    pub reason: Option<String>,
}

impl RunStateMachine {
    pub fn validate_transition(
        current: &RunStatus,
        target: &RunStatus,
        has_summary: bool,
        has_error: bool,
        has_reason: bool,
        pending_tool_count: usize,
    ) -> Result<Transition, TransitionError> {
        let valid = match (current, target) {
            (RunStatus::Pending, RunStatus::Running) => true,
            (RunStatus::Pending, RunStatus::Cancelled) => has_reason,

            (RunStatus::Running, RunStatus::Completed) => has_summary && pending_tool_count == 0,
            (RunStatus::Running, RunStatus::Failed) => has_error && pending_tool_count == 0,
            (RunStatus::Running, RunStatus::Paused) => true,
            (RunStatus::Running, RunStatus::Cancelled) => has_reason,

            (RunStatus::Paused, RunStatus::Running) => pending_tool_count == 0,
            (RunStatus::Paused, RunStatus::Cancelled) => has_reason,

            _ => false,
        };

        if !valid {
            return Err(TransitionError::InvalidTransition {
                from: current.as_str().to_string(),
                to: target.as_str().to_string(),
            });
        }

        Ok(Transition {
            from: current.clone(),
            to: target.clone(),
            reason: None, // Reason stored separately
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transitions() {
        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Pending,
                &RunStatus::Running,
                false,
                false,
                false,
                0
            )
            .is_ok()
        );

        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Running,
                &RunStatus::Completed,
                true,
                false,
                false,
                0
            )
            .is_ok()
        );

        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Running,
                &RunStatus::Failed,
                false,
                true,
                false,
                0
            )
            .is_ok()
        );

        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Running,
                &RunStatus::Paused,
                false,
                false,
                false,
                0
            )
            .is_ok()
        );

        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Paused,
                &RunStatus::Running,
                false,
                false,
                false,
                0
            )
            .is_ok()
        );
    }

    #[test]
    fn invalid_transitions() {
        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Pending,
                &RunStatus::Completed,
                false,
                false,
                false,
                0
            )
            .is_err()
        );

        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Completed,
                &RunStatus::Running,
                false,
                false,
                false,
                0
            )
            .is_err()
        );
    }

    #[test]
    fn pending_tools_block_transition() {
        assert!(
            RunStateMachine::validate_transition(
                &RunStatus::Running,
                &RunStatus::Completed,
                true,
                false,
                false,
                2
            )
            .is_err()
        );
    }
}
