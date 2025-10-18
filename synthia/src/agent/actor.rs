use super::messages::{Command, UIUpdate};
use crate::llm::json_parser::JsonParser;
use crate::llm::{GenerationConfig, LLMProvider, StreamEvent};
use crate::session::Session;
use crate::tools::registry::ToolRegistry;
use crate::types::{ContentBlock, Message, Role, StopReason};
use anyhow::Result;
use futures::future::join_all;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct AgentActor {
    llm_provider: Arc<dyn LLMProvider>,
    tool_registry: Arc<ToolRegistry>,
    conversation: Vec<Message>,
    config: GenerationConfig,
    ui_tx: Sender<UIUpdate>,
    cmd_rx: Receiver<Command>,
    session: Session,
    auto_save: bool,
    cancel_requested: bool,
    tool_call_count: usize, // Track tool calls in current turn
    json_parser: JsonParser, // For robust JSON parsing
}

impl AgentActor {
    /// Create the system prompt that teaches the model to use tools proactively
    fn create_system_prompt() -> Message {
        Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: r#"You are Synthia, an AI assistant with access to powerful tools. ALWAYS use tools proactively instead of asking the user to do things manually.

CRITICAL RULES:
- When you need information from a file, use the 'read' tool immediately
- When you need to run a command, use the 'bash' tool immediately
- When you need to search files, use 'grep' or 'glob' tools
- NEVER ask "would you like me to..." or "shall I..." - just do it
- NEVER ask the user to paste file contents - use the read tool
- NEVER ask the user to run commands - use the bash tool

AVAILABLE TOOLS:
- read: Read file contents (use instead of asking for file contents)
- write: Create new files
- edit: Modify existing files
- bash: Run shell commands (use instead of asking user to check terminal)
- grep: Search file contents with patterns
- glob: Find files matching patterns
- git: Git operations (status, diff, commit, etc.)
- webfetch: Fetch web content
- powertools: Code navigation (goto definition, find references)
- workshop: Context and session management

EXAMPLES OF CORRECT BEHAVIOR:
User: "What's in the README?"
You: "I'll read that file for you." [immediately use read tool]

User: "Are there any Python errors?"
You: "Let me check the logs." [immediately use bash or grep tool]

User: "Check if the server is running"
You: "I'll check the running processes." [immediately use bash tool]

Be direct, confident, and proactive. Use tools without hesitation."#.to_string(),
            }],
        }
    }

    pub fn new(
        llm_provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: GenerationConfig,
        ui_tx: Sender<UIUpdate>,
        cmd_rx: Receiver<Command>,
    ) -> Self {
        let session = Session::new(config.model.clone());

        // Start with system prompt
        let conversation = vec![Self::create_system_prompt()];

        Self {
            llm_provider,
            tool_registry,
            conversation,
            config,
            ui_tx,
            cmd_rx,
            session,
            auto_save: true,
            cancel_requested: false,
            tool_call_count: 0,
            json_parser: JsonParser::new(),
        }
    }

    pub fn with_session(
        llm_provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: GenerationConfig,
        ui_tx: Sender<UIUpdate>,
        cmd_rx: Receiver<Command>,
        session: Session,
    ) -> Self {
        let mut conversation = session.messages.clone();

        // Ensure system prompt is first (prepend if not present)
        let has_system_prompt = conversation.first()
            .map(|msg| matches!(msg.role, Role::System))
            .unwrap_or(false);

        if !has_system_prompt {
            conversation.insert(0, Self::create_system_prompt());
        }

        Self {
            llm_provider,
            tool_registry,
            conversation,
            config,
            ui_tx,
            cmd_rx,
            session,
            auto_save: true,
            cancel_requested: false,
            tool_call_count: 0,
            json_parser: JsonParser::new(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session.id
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Agent actor starting with session: {}", self.session.id);

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::SendMessage(text) => {
                    let message = Message {
                        role: Role::User,
                        content: vec![ContentBlock::Text { text }],
                    };
                    self.conversation.push(message.clone());
                    self.session.add_message(message);

                    if let Err(e) = self.generate_response().await {
                        self.ui_tx
                            .send(UIUpdate::Error(format!("Agent error: {}", e)))
                            .await?;
                    }
                }
                Command::Cancel => {
                    tracing::info!("Cancellation requested");
                    self.cancel_requested = true;
                    self.ui_tx.send(UIUpdate::Error("Generation canceled by user".to_string())).await?;
                }
                Command::Shutdown => {
                    tracing::info!("Shutdown requested");
                    // Save session before shutting down
                    if let Err(e) = self.session.save() {
                        tracing::error!("Failed to save session on shutdown: {}", e);
                    }
                    break;
                }
                Command::SaveSession => {
                    if let Err(e) = self.session.save() {
                        self.ui_tx
                            .send(UIUpdate::Error(format!("Failed to save session: {}", e)))
                            .await?;
                    } else {
                        self.ui_tx
                            .send(UIUpdate::SessionSaved {
                                session_id: self.session.id.clone(),
                            })
                            .await?;
                    }
                }
                Command::NewSession => {
                    // Save current session first
                    if let Err(e) = self.session.save() {
                        tracing::error!("Failed to save current session: {}", e);
                    }

                    // Create new session
                    self.session = Session::new(self.config.model.clone());
                    self.conversation.clear();
                    self.conversation.push(Self::create_system_prompt()); // Add system prompt to new session

                    // Tell UI to clear displayed conversation
                    self.ui_tx.send(UIUpdate::ConversationCleared).await?;

                    self.ui_tx
                        .send(UIUpdate::SessionLoaded {
                            session_id: self.session.id.clone(),
                        })
                        .await?;
                }
                Command::LoadSession(session_id) => {
                    match Session::load(&session_id) {
                        Ok(session) => {
                            // Save current session first
                            if let Err(e) = self.session.save() {
                                tracing::error!("Failed to save current session: {}", e);
                            }

                            self.conversation = session.messages.clone();
                            self.session = session;

                            self.ui_tx
                                .send(UIUpdate::SessionLoaded {
                                    session_id: self.session.id.clone(),
                                })
                                .await?;
                        }
                        Err(e) => {
                            self.ui_tx
                                .send(UIUpdate::Error(format!("Failed to load session: {}", e)))
                                .await?;
                        }
                    }
                }
                Command::ListSessions => {
                    match crate::session::list_sessions() {
                        Ok(sessions) => {
                            self.ui_tx
                                .send(UIUpdate::SessionList { sessions })
                                .await?;
                        }
                        Err(e) => {
                            self.ui_tx
                                .send(UIUpdate::Error(format!("Failed to list sessions: {}", e)))
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn generate_response(&mut self) -> Result<()> {
        // Reset cancellation flag and tool call count at the start
        self.cancel_requested = false;
        self.tool_call_count = 0;
        const MAX_TOOL_CALLS: usize = 50; // Prevent infinite loops

        loop {
            // Check for pending cancel/newsession commands (non-blocking)
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                match cmd {
                    Command::Cancel => {
                        tracing::info!("Cancel command received during generation");
                        self.cancel_requested = true;
                        self.ui_tx.send(UIUpdate::Error("Generation canceled by user".to_string())).await?;
                    }
                    Command::NewSession => {
                        tracing::info!("NewSession command received during generation - canceling first");
                        self.cancel_requested = true;
                        // We'll handle the actual new session after cancellation completes
                        // For now, just cancel and let the user try again
                        self.ui_tx.send(UIUpdate::Error("Please wait for current generation to cancel before starting new session".to_string())).await?;
                    }
                    _ => {
                        // Other commands received during generation are ignored
                        tracing::warn!("Received command during generation, ignoring until complete");
                    }
                }
            }

            // Check for cancellation
            if self.cancel_requested {
                tracing::info!("Generation canceled, breaking loop");
                break;
            }

            // Check tool call limit to prevent runaway loops
            if self.tool_call_count >= MAX_TOOL_CALLS {
                let error_msg = format!(
                    "Tool call limit exceeded ({} calls). Stopping to prevent infinite loop. \
                    This usually happens when the same tool fails repeatedly or gets stuck in a loop.",
                    MAX_TOOL_CALLS
                );
                tracing::warn!("{}", error_msg);
                self.ui_tx.send(UIUpdate::Error(error_msg)).await?;
                break;
            }

            // Use streaming or non-streaming based on config
            if self.config.streaming {
                self.generate_response_streaming().await?;
            } else {
                self.generate_response_non_streaming().await?;
            }

            // Check if we should continue (tool calls need another round)
            let should_continue = self.conversation.last()
                .map(|msg| {
                    msg.content.iter().any(|block| matches!(block, ContentBlock::ToolResult { .. }))
                })
                .unwrap_or(false);

            if !should_continue {
                self.ui_tx.send(UIUpdate::Complete).await?;

                // Auto-save session after complete response
                if self.auto_save {
                    if let Err(e) = self.session.save() {
                        tracing::error!("Failed to auto-save session: {}", e);
                    } else {
                        tracing::debug!("Session auto-saved: {}", self.session.id);
                    }
                }
                break;
            }
        }

        Ok(())
    }

    async fn generate_response_streaming(&mut self) -> Result<()> {
        // Send thinking indicator
        self.ui_tx.send(UIUpdate::AssistantThinking).await?;

        let mut stream = self
            .llm_provider
            .stream_chat_completion(
                self.conversation.clone(),
                self.tool_registry.definitions(),
                &self.config,
            )
            .await?;

        let mut accumulated_text = String::new();
        let mut tool_calls: HashMap<String, (String, String)> = HashMap::new(); // id -> (name, accumulated_args)

        while let Some(event_result) = stream.next().await {
            // Check for pending cancel commands (non-blocking)
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                if matches!(cmd, Command::Cancel | Command::NewSession) {
                    tracing::info!("Cancel/NewSession command received during streaming");
                    self.cancel_requested = true;
                }
            }

            // Check for cancellation in the streaming loop
            if self.cancel_requested {
                tracing::info!("Cancellation detected in stream, breaking");
                break;
            }

            match event_result {
                Ok(event) => match event {
                    StreamEvent::TextDelta(delta) => {
                        accumulated_text.push_str(&delta);
                        self.ui_tx.send(UIUpdate::AssistantTextDelta(delta)).await?;
                    }
                    StreamEvent::ToolCallStart { id, name } => {
                        tool_calls.insert(id.clone(), (name.clone(), String::new()));
                        self.ui_tx
                            .send(UIUpdate::ToolExecutionStarted {
                                name: name.clone(),
                                id: id.clone(),
                            })
                            .await?;
                    }
                    StreamEvent::ToolCallDelta { id, arguments_delta } => {
                        if let Some((_, args)) = tool_calls.get_mut(&id) {
                            args.push_str(&arguments_delta);
                        }
                    }
                    StreamEvent::Done { stop_reason: _reason, usage: _ } => {
                        break;
                    }
                    StreamEvent::Error(err) => {
                        self.ui_tx.send(UIUpdate::Error(format!("Stream error: {}", err))).await?;
                        return Err(anyhow::anyhow!("Stream error: {}", err));
                    }
                },
                Err(e) => {
                    self.ui_tx.send(UIUpdate::Error(format!("Stream error: {}", e))).await?;
                    return Err(e);
                }
            }
        }

        // Build the assistant message from accumulated data
        let mut content = Vec::new();

        if !accumulated_text.is_empty() {
            content.push(ContentBlock::Text { text: accumulated_text });
        }

        for (id, (name, args_str)) in tool_calls {
            let input = self.json_parser.parse_robust(&args_str).unwrap_or_else(|e| {
                tracing::error!("Failed to parse tool arguments for tool '{}' (id: {}): {}\nRaw arguments: {}", name, id, e, args_str);
                serde_json::json!({})
            });
            content.push(ContentBlock::ToolUse { id, name, input });
        }

        let assistant_message = Message {
            role: Role::Assistant,
            content: content.clone(),
        };
        self.conversation.push(assistant_message.clone());
        self.session.add_message(assistant_message);

        // Collect all tool calls from this response
        let mut tool_calls = Vec::new();
        for block in &content {
            if let ContentBlock::ToolUse { id, name, input } = block {
                tool_calls.push((id.clone(), name.clone(), input.clone()));
            }
        }

        // Execute all tools in parallel
        if !tool_calls.is_empty() {
            // Check for cancellation before starting tool execution
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                if matches!(cmd, Command::Cancel | Command::NewSession) {
                    tracing::info!("Cancel/NewSession command received before tool execution");
                    self.cancel_requested = true;
                    return Ok(());
                }
            }

            if self.cancel_requested {
                return Ok(());
            }

            tracing::info!("Executing {} tools in parallel", tool_calls.len());
            self.tool_call_count += tool_calls.len(); // Track tool calls to prevent infinite loops

            let futures: Vec<_> = tool_calls
                .iter()
                .map(|(id, name, input)| {
                    let registry = self.tool_registry.clone();
                    let name = name.clone();
                    let input = input.clone();
                    let id = id.clone();

                    async move {
                        let start = Instant::now();
                        let result = registry.execute(&name, input.clone()).await;
                        let duration_ms = start.elapsed().as_millis() as u64;
                        (id, name, input, result, duration_ms)
                    }
                })
                .collect();

            let results = join_all(futures).await;

            // Process results in order
            for (id, name, input, result, duration_ms) in results {
                // Check for cancellation between result processing
                while let Ok(cmd) = self.cmd_rx.try_recv() {
                    if matches!(cmd, Command::Cancel | Command::NewSession) {
                        tracing::info!("Cancel/NewSession command received during result processing");
                        self.cancel_requested = true;
                        return Ok(());
                    }
                }

                if self.cancel_requested {
                    return Ok(());
                }

                match result {
                    Ok(tool_result) => {
                        // Truncate output to first 500 chars for UI display
                        let output_preview = if tool_result.content.len() > 500 {
                            format!("{}...", &tool_result.content[..500])
                        } else {
                            tool_result.content.clone()
                        };

                        self.ui_tx
                            .send(UIUpdate::ToolResult {
                                name: name.clone(),
                                id: id.clone(),
                                input: input.clone(),
                                output: output_preview,
                                is_error: tool_result.is_error,
                                duration_ms,
                            })
                            .await?;

                        // Add tool result to conversation
                        let result_message = Message {
                            role: Role::User,
                            content: vec![ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: tool_result.content,
                                is_error: tool_result.is_error,
                            }],
                        };
                        self.conversation.push(result_message.clone());
                        self.session.add_message(result_message);
                    }
                    Err(e) => {
                        let error_msg = e.to_string();

                        // Check if this is a parameter error (malformed input)
                        let is_param_error = error_msg.contains("Missing")
                            || error_msg.contains("required")
                            || error_msg.contains("parameter");

                        let error_content = if is_param_error {
                            format!(
                                "Error: {}. Please check the tool schema and retry with valid JSON parameters.",
                                error_msg
                            )
                        } else {
                            format!("Error: {}", error_msg)
                        };

                        tracing::warn!("Tool execution failed for '{}': {}", name, error_msg);

                        self.ui_tx
                            .send(UIUpdate::ToolResult {
                                name: name.clone(),
                                id: id.clone(),
                                input: input.clone(),
                                output: error_content.clone(),
                                is_error: true,
                                duration_ms,
                            })
                            .await?;

                        // Add error result to conversation so LLM can retry
                        let result_message = Message {
                            role: Role::User,
                            content: vec![ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: error_content,
                                is_error: true,
                            }],
                        };
                        self.conversation.push(result_message.clone());
                        self.session.add_message(result_message);
                    }
                }
            }
        }

        Ok(())
    }

    async fn generate_response_non_streaming(&mut self) -> Result<()> {
        let response = self
            .llm_provider
            .chat_completion(
                self.conversation.clone(),
                self.tool_registry.definitions(),
                &self.config,
            )
            .await?;

        // Add the full assistant response as a single message
        let assistant_message = Message {
            role: Role::Assistant,
            content: response.content.clone(),
        };
        self.conversation.push(assistant_message.clone());
        self.session.add_message(assistant_message);

        // Display text content immediately
        for block in &response.content {
            if let ContentBlock::Text { text } = block {
                self.ui_tx.send(UIUpdate::AssistantText(text.clone())).await?;
            }
        }

        // Collect all tool calls from this response
        let mut tool_calls = Vec::new();
        for block in &response.content {
            if let ContentBlock::ToolUse { id, name, input } = block {
                // Send UI notification that tool execution is starting
                self.ui_tx
                    .send(UIUpdate::ToolExecutionStarted {
                        name: name.clone(),
                        id: id.clone(),
                    })
                    .await?;
                tool_calls.push((id.clone(), name.clone(), input.clone()));
            }
        }

        // Execute all tools in parallel
        if !tool_calls.is_empty() {
            // Check for cancellation before starting tool execution
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                if matches!(cmd, Command::Cancel | Command::NewSession) {
                    tracing::info!("Cancel/NewSession command received before tool execution (non-streaming)");
                    self.cancel_requested = true;
                    return Ok(());
                }
            }

            if self.cancel_requested {
                return Ok(());
            }

            tracing::info!("Executing {} tools in parallel (non-streaming)", tool_calls.len());
            self.tool_call_count += tool_calls.len(); // Track tool calls to prevent infinite loops

            let futures: Vec<_> = tool_calls
                .iter()
                .map(|(id, name, input)| {
                    let registry = self.tool_registry.clone();
                    let name = name.clone();
                    let input = input.clone();
                    let id = id.clone();

                    async move {
                        let start = Instant::now();
                        let result = registry.execute(&name, input.clone()).await;
                        let duration_ms = start.elapsed().as_millis() as u64;
                        (id, name, input, result, duration_ms)
                    }
                })
                .collect();

            let results = join_all(futures).await;

            // Process results in order
            for (id, name, input, result, duration_ms) in results {
                // Check for cancellation between result processing
                while let Ok(cmd) = self.cmd_rx.try_recv() {
                    if matches!(cmd, Command::Cancel | Command::NewSession) {
                        tracing::info!("Cancel/NewSession command received during result processing (non-streaming)");
                        self.cancel_requested = true;
                        return Ok(());
                    }
                }

                if self.cancel_requested {
                    return Ok(());
                }

                match result {
                    Ok(tool_result) => {
                        // Truncate output to first 500 chars for UI display
                        let output_preview = if tool_result.content.len() > 500 {
                            format!("{}...", &tool_result.content[..500])
                        } else {
                            tool_result.content.clone()
                        };

                        self.ui_tx
                            .send(UIUpdate::ToolResult {
                                name: name.clone(),
                                id: id.clone(),
                                input: input.clone(),
                                output: output_preview,
                                is_error: tool_result.is_error,
                                duration_ms,
                            })
                            .await?;

                        // Add tool result to conversation
                        let result_message = Message {
                            role: Role::User,
                            content: vec![ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: tool_result.content,
                                is_error: tool_result.is_error,
                            }],
                        };
                        self.conversation.push(result_message.clone());
                        self.session.add_message(result_message);
                    }
                    Err(e) => {
                        let error_msg = e.to_string();

                        // Check if this is a parameter error (malformed input)
                        let is_param_error = error_msg.contains("Missing")
                            || error_msg.contains("required")
                            || error_msg.contains("parameter");

                        let error_content = if is_param_error {
                            format!(
                                "Error: {}. Please check the tool schema and retry with valid JSON parameters.",
                                error_msg
                            )
                        } else {
                            format!("Error: {}", error_msg)
                        };

                        tracing::warn!("Tool execution failed for '{}': {}", name, error_msg);

                        self.ui_tx
                            .send(UIUpdate::ToolResult {
                                name: name.clone(),
                                id: id.clone(),
                                input: input.clone(),
                                output: error_content.clone(),
                                is_error: true,
                                duration_ms,
                            })
                            .await?;

                        // Add error result to conversation so LLM can retry
                        let result_message = Message {
                            role: Role::User,
                            content: vec![ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: error_content,
                                is_error: true,
                            }],
                        };
                        self.conversation.push(result_message.clone());
                        self.session.add_message(result_message);
                    }
                }
            }
        }

        Ok(())
    }
}
