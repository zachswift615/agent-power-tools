#[derive(Debug, Clone)]
pub enum Command {
    SendMessage(String),
    Cancel,
    Shutdown,
    SaveSession,
    NewSession,
    LoadSession(String),
    ListSessions,
    SetSessionName(String),
    SetReasoningLevel(String),
    ShowMenu,
    CompactContext,
    ViewContextStats,
}

#[derive(Debug, Clone)]
pub enum ApprovalResponse {
    Approve,
    Reject,
}

#[derive(Debug)]
pub enum PermissionResponse {
    Yes,
    YesAndDontAsk(String),  // Contains pattern to add
    No,
}

#[derive(Debug)]
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
    EditPreview {
        file_path: String,
        old_string: String,
        new_string: String,
        diff: String,
        response_tx: tokio::sync::oneshot::Sender<ApprovalResponse>,
    },
    MenuDisplayRequested,  // Signal UI to display menu
    SystemMessage(String), // System notifications (e.g., auto-compaction)
    TokenStatsUpdate(crate::context_manager::TokenStats), // Token usage stats update
    PermissionPrompt {
        tool_name: String,
        operation_details: String,
        suggested_pattern: String,
        response_tx: tokio::sync::oneshot::Sender<PermissionResponse>,
    },
    InformationalDiff {
        tool_name: String,
        file_path: String,
        diff: String,
    },
}
