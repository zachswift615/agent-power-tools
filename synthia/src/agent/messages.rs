#[derive(Debug, Clone)]
pub enum Command {
    SendMessage(String),
    Cancel,
    Shutdown,
    SaveSession,
    NewSession,
    LoadSession(String),
    ListSessions,
}

#[derive(Debug, Clone)]
pub enum UIUpdate {
    AssistantText(String),
    AssistantTextDelta(String), // For streaming text chunks
    AssistantThinking, // Indicator that agent is thinking
    ToolExecutionStarted { name: String, id: String },
    ToolResult {
        name: String,
        id: String,
        input: serde_json::Value,
        output: String,
        is_error: bool,
        duration_ms: u64
    },
    Error(String),
    Complete,
    SessionSaved { session_id: String },
    SessionLoaded { session_id: String },
    SessionList { sessions: Vec<crate::session::SessionInfo> },
    ConversationCleared, // Signal UI to clear displayed conversation
}
