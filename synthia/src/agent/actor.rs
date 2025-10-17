use super::messages::{Command, UIUpdate};
use crate::llm::{GenerationConfig, LLMProvider, LLMResponse};
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
}

impl AgentActor {
    pub fn new(
        llm_provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: GenerationConfig,
        ui_tx: Sender<UIUpdate>,
        cmd_rx: Receiver<Command>,
    ) -> Self {
        Self {
            llm_provider,
            tool_registry,
            conversation: Vec::new(),
            config,
            ui_tx,
            cmd_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Agent actor starting");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::SendMessage(text) => {
                    self.conversation.push(Message {
                        role: Role::User,
                        content: vec![ContentBlock::Text { text }],
                    });
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
                    break;
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
            self.conversation.push(Message {
                role: Role::Assistant,
                content: response.content.clone(),
            });

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
                        self.conversation.push(Message {
                            role: Role::User,
                            content: vec![ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: result.content,
                                is_error: result.is_error,
                            }],
                        });
                    }
                    _ => {}
                }
            }

            // Check stop reason
            if matches!(response.stop_reason, StopReason::EndTurn) {
                self.ui_tx.send(UIUpdate::Complete).await?;
                break;
            }
        }

        Ok(())
    }
}
