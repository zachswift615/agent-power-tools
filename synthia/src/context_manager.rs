use crate::llm::LLMProvider;
use crate::types::{ContentBlock, Message, Role};
use anyhow::Result;
use std::sync::Arc;

const MAX_MESSAGES: usize = 100;
const SUMMARY_THRESHOLD: usize = 80;
const SUMMARY_CHUNK_SIZE: usize = 20;

pub struct ContextManager {
    messages: Vec<Message>,
    max_messages: usize,
    summary_threshold: usize,
    llm_provider: Arc<dyn LLMProvider>,
    current_token_count: usize,      // Track current context tokens
    max_token_limit: usize,          // Model's max context window
    token_threshold_percent: f32,    // Auto-compact at this % (default 0.8)
}

#[derive(Debug, Clone)]
pub struct TokenStats {
    pub current: usize,
    pub max: usize,
    pub threshold: usize,
    pub usage_percent: f32,
}

impl ContextManager {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            messages: Vec::new(),
            max_messages: MAX_MESSAGES,
            summary_threshold: SUMMARY_THRESHOLD,
            llm_provider,
            current_token_count: 0,
            max_token_limit: 8192,  // Default, should be configurable per model
            token_threshold_percent: 0.8,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Set the max token limit for this model
    pub fn set_max_token_limit(&mut self, limit: usize) {
        self.max_token_limit = limit;
        tracing::info!("Context manager max token limit set to {}", limit);
    }

    /// Update token count after each response
    pub fn update_token_count(&mut self, input_tokens: usize, output_tokens: usize) {
        self.current_token_count = input_tokens + output_tokens;

        let threshold = (self.max_token_limit as f32 * self.token_threshold_percent) as usize;
        let usage_percent = (self.current_token_count as f32 / self.max_token_limit as f32) * 100.0;

        tracing::debug!(
            "Token usage: {} / {} ({:.1}%)",
            self.current_token_count,
            self.max_token_limit,
            usage_percent
        );

        if self.current_token_count >= threshold {
            tracing::info!("Token threshold reached ({}/{} = {:.1}%), compaction recommended",
                self.current_token_count, self.max_token_limit, usage_percent);
        }
    }

    /// Check if auto-compaction should trigger
    pub fn should_compact(&self) -> bool {
        let threshold = (self.max_token_limit as f32 * self.token_threshold_percent) as usize;
        self.current_token_count >= threshold
    }

    /// Get current token usage stats
    pub fn get_token_stats(&self) -> TokenStats {
        let threshold = (self.max_token_limit as f32 * self.token_threshold_percent) as usize;
        let usage_percent = (self.current_token_count as f32 / self.max_token_limit as f32) * 100.0;

        TokenStats {
            current: self.current_token_count,
            max: self.max_token_limit,
            threshold,
            usage_percent,
        }
    }

    pub async fn compact_if_needed(&mut self) -> Result<()> {
        // Use token-based threshold instead of message count
        if self.should_compact() {
            self.summarize_oldest_messages().await?;

            // After compaction, estimate new token count (conservative: assume 50% reduction)
            self.current_token_count = (self.current_token_count as f32 * 0.5) as usize;
            tracing::info!("Estimated token count after compaction: {}", self.current_token_count);
        }

        // Keep hard message limit as a safety fallback
        if self.messages.len() >= self.max_messages {
            // Hard truncate
            let to_remove = self.messages.len() - self.max_messages;
            self.messages.drain(0..to_remove);
        }

        Ok(())
    }

    async fn summarize_oldest_messages(&mut self) -> Result<()> {
        // Keep first message (system) and last 60%
        let keep_recent = (self.messages.len() as f32 * 0.6) as usize;
        let summarize_start = 1; // Skip system message
        let summarize_end = self.messages.len() - keep_recent;

        if summarize_end <= summarize_start {
            return Ok(()); // Nothing to summarize
        }

        let to_summarize = &self.messages[summarize_start..summarize_end];

        // Create summary using LLM
        let summary_prompt = format!(
            "Summarize this conversation segment concisely, preserving key decisions, tool calls, and outcomes:\n\n{}",
            self.format_messages_for_summary(to_summarize)
        );

        let summary_response = self
            .llm_provider
            .chat_completion(
                vec![Message {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: summary_prompt,
                    }],
                }],
                vec![], // No tools
                &crate::llm::GenerationConfig {
                    model: "qwen2.5-coder-7b-instruct".to_string(),
                    temperature: 0.3, // Low temp for factual summary
                    max_tokens: Some(500),
                    streaming: false,
                    reasoning_level: "medium".to_string(),
                    context_window: 8192, // Default context window
                },
            )
            .await?;

        // Extract summary text
        let summary_text = summary_response
            .content
            .iter()
            .find_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "[Summary generation failed]".to_string());

        // Replace old messages with summary
        let summary_message = Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: format!("[Conversation Summary]: {}", summary_text),
            }],
        };

        self.messages.drain(summarize_start..summarize_end);
        self.messages.insert(summarize_start, summary_message);

        tracing::info!(
            "Summarized {} messages into 1 summary message",
            summarize_end - summarize_start
        );

        Ok(())
    }

    fn format_messages_for_summary(&self, messages: &[Message]) -> String {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::System => "System",
                };

                let content = msg
                    .content
                    .iter()
                    .map(|block| match block {
                        ContentBlock::Text { text } => text.clone(),
                        ContentBlock::ToolUse { name, .. } => {
                            format!("[Called tool: {}]", name)
                        }
                        ContentBlock::ToolResult {
                            content, is_error, ..
                        } => {
                            if *is_error {
                                format!("[Tool error: {}]", content)
                            } else {
                                format!(
                                    "[Tool result: {}]",
                                    content.chars().take(100).collect::<String>()
                                )
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("{}: {}", role, content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::LLMResponse;
    use async_trait::async_trait;
    use serde_json::Value;
    use crate::types::{StopReason, TokenUsage};

    // Mock LLM provider for testing
    struct MockLLMProvider;

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn chat_completion(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<Value>,
            _config: &crate::llm::GenerationConfig,
        ) -> Result<LLMResponse> {
            Ok(LLMResponse {
                content: vec![ContentBlock::Text {
                    text: "This is a summary of the conversation.".to_string(),
                }],
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                },
            })
        }

        async fn stream_chat_completion(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<Value>,
            _config: &crate::llm::GenerationConfig,
        ) -> Result<crate::llm::provider::StreamResult> {
            unimplemented!("Not needed for tests")
        }
    }

    #[tokio::test]
    async fn test_add_message() {
        let provider = Arc::new(MockLLMProvider);
        let mut context_manager = ContextManager::new(provider);

        let message = Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        };

        context_manager.add_message(message);
        assert_eq!(context_manager.get_messages().len(), 1);
    }

    #[tokio::test]
    async fn test_compact_at_token_threshold() {
        let provider = Arc::new(MockLLMProvider);
        let mut context_manager = ContextManager::new(provider);
        context_manager.set_max_token_limit(1000); // Small limit for testing

        // Add system message first
        context_manager.add_message(Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: "System prompt".to_string(),
            }],
        });

        // Add several messages
        for i in 0..10 {
            context_manager.add_message(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: format!("Message {}", i),
                }],
            });
        }

        let initial_count = context_manager.get_messages().len();

        // Simulate reaching 80% token usage
        context_manager.update_token_count(600, 200); // 800 tokens = 80%

        assert!(context_manager.should_compact());

        // Trigger compaction
        context_manager.compact_if_needed().await.unwrap();

        // Token count should be reduced (estimated to 50%)
        let stats = context_manager.get_token_stats();
        assert!(stats.current < 800);
        assert_eq!(stats.current, 400); // 50% of 800

        // Message count should also be reduced
        assert!(context_manager.get_messages().len() < initial_count);
    }

    #[tokio::test]
    async fn test_hard_truncate_at_max() {
        let provider = Arc::new(MockLLMProvider);
        let mut context_manager = ContextManager::new(provider);

        // Add system message
        context_manager.add_message(Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: "System prompt".to_string(),
            }],
        });

        // Add more than max messages
        for i in 0..MAX_MESSAGES + 10 {
            context_manager.add_message(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: format!("Message {}", i),
                }],
            });
        }

        context_manager.compact_if_needed().await.unwrap();

        // Should be at or under max
        assert!(context_manager.get_messages().len() <= MAX_MESSAGES);
    }

    #[test]
    fn test_format_messages_for_summary() {
        let provider = Arc::new(MockLLMProvider);
        let context_manager = ContextManager::new(provider);

        let messages = vec![
            Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            },
            Message {
                role: Role::Assistant,
                content: vec![ContentBlock::ToolUse {
                    id: "1".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({"file_path": "test.txt"}),
                }],
            },
            Message {
                role: Role::User,
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: "1".to_string(),
                    content: "File contents here".to_string(),
                    is_error: false,
                }],
            },
        ];

        let formatted = context_manager.format_messages_for_summary(&messages);

        assert!(formatted.contains("User: Hello"));
        assert!(formatted.contains("Assistant: [Called tool: read]"));
        assert!(formatted.contains("User: [Tool result:"));
    }
}
