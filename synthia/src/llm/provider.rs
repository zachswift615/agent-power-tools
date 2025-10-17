use crate::types::{ContentBlock, Message, StopReason, TokenUsage};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Vec<Value>,
        config: &GenerationConfig,
    ) -> Result<LLMResponse>;
}
