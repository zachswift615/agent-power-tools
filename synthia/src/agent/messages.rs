use crate::types::Message;

#[derive(Debug, Clone)]
pub enum Command {
    SendMessage(String),
    Cancel,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum UIUpdate {
    AssistantText(String),
    ToolExecutionStarted { name: String, id: String },
    ToolExecutionCompleted { name: String, id: String, duration_ms: u64 },
    Error(String),
    Complete,
}
