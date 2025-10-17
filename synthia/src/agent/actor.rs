use super::messages::{Command, UIUpdate};
use crate::llm::{GenerationConfig, LLMProvider, StreamEvent};
use crate::session::Session;
use crate::tools::registry::ToolRegistry;
use crate::types::{ContentBlock, Message, Role, StopReason};
use anyhow::Result;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
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
                    // TODO: Implement cancellation
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
        loop {
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
            let input = serde_json::from_str(&args_str).unwrap_or_else(|e| {
                tracing::error!("Failed to parse tool arguments for tool '{}' (id: {}): {}. Raw arguments: {}", name, id, e, args_str);
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

        // Execute tool calls
        for block in &content {
            if let ContentBlock::ToolUse { id, name, input } = block {
                let start = std::time::Instant::now();
                let result = self.tool_registry.execute(name, input.clone()).await?;
                let duration_ms = start.elapsed().as_millis() as u64;

                // Truncate output to first 500 chars for UI display
                let output_preview = if result.content.len() > 500 {
                    format!("{}...", &result.content[..500])
                } else {
                    result.content.clone()
                };

                self.ui_tx
                    .send(UIUpdate::ToolResult {
                        name: name.clone(),
                        id: id.clone(),
                        input: input.clone(),
                        output: output_preview,
                        is_error: result.is_error,
                        duration_ms,
                    })
                    .await?;

                // Add tool result to conversation
                let tool_result = Message {
                    role: Role::User,
                    content: vec![ContentBlock::ToolResult {
                        tool_use_id: id.clone(),
                        content: result.content,
                        is_error: result.is_error,
                    }],
                };
                self.conversation.push(tool_result.clone());
                self.session.add_message(tool_result);
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

        // Process response content
        for block in &response.content {
            match block {
                ContentBlock::Text { text } => {
                    self.ui_tx.send(UIUpdate::AssistantText(text.clone())).await?;
                }
                ContentBlock::ToolUse { id, name, input } => {
                    self.ui_tx
                        .send(UIUpdate::ToolExecutionStarted {
                            name: name.clone(),
                            id: id.clone(),
                        })
                        .await?;

                    let start = std::time::Instant::now();
                    let result = self.tool_registry.execute(name, input.clone()).await?;
                    let duration_ms = start.elapsed().as_millis() as u64;

                    // Truncate output to first 500 chars for UI display
                    let output_preview = if result.content.len() > 500 {
                        format!("{}...", &result.content[..500])
                    } else {
                        result.content.clone()
                    };

                    self.ui_tx
                        .send(UIUpdate::ToolResult {
                            name: name.clone(),
                            id: id.clone(),
                            input: input.clone(),
                            output: output_preview,
                            is_error: result.is_error,
                            duration_ms,
                        })
                        .await?;

                    // Add tool result to conversation
                    let tool_result = Message {
                        role: Role::User,
                        content: vec![ContentBlock::ToolResult {
                            tool_use_id: id.clone(),
                            content: result.content,
                            is_error: result.is_error,
                        }],
                    };
                    self.conversation.push(tool_result.clone());
                    self.session.add_message(tool_result);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
