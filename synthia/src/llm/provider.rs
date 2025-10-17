use crate::types::{ContentBlock, Message, StopReason, TokenUsage};
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub model: String,
    pub streaming: bool,
}

#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    TextDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, arguments_delta: String },
    Done { stop_reason: StopReason, usage: TokenUsage },
    Error(String),
}

pub type StreamResult = Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Vec<Value>,
        config: &GenerationConfig,
    ) -> Result<LLMResponse>;

    async fn stream_chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Vec<Value>,
        config: &GenerationConfig,
    ) -> Result<StreamResult>;
}
