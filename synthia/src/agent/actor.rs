use super::messages::{Command, UIUpdate};
use crate::llm::{GenerationConfig, LLMProvider, LLMResponse};
use crate::session::Session;
use crate::tools::registry::ToolRegistry;
use crate::types::{ContentBlock, Message, Role, StopReason};
use anyhow::Result;
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
    pub fn new(
        llm_provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: GenerationConfig,
        ui_tx: Sender<UIUpdate>,
        cmd_rx: Receiver<Command>,
    ) -> Self {
        let session = Session::new(config.model.clone());
        Self {
            llm_provider,
            tool_registry,
            conversation: Vec::new(),
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
        let conversation = session.messages.clone();
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

                        self.ui_tx
                            .send(UIUpdate::ToolExecutionCompleted {
                                name: name.clone(),
                                id: id.clone(),
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

            // Check stop reason
            if matches!(response.stop_reason, StopReason::EndTurn) {
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
}
