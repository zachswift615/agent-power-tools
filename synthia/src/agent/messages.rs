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
    ToolExecutionStarted { name: String, id: String },
    ToolExecutionCompleted { name: String, id: String, duration_ms: u64 },
    Error(String),
    Complete,
    SessionSaved { session_id: String },
    SessionLoaded { session_id: String },
    SessionList { sessions: Vec<crate::session::SessionInfo> },
}
