use super::provider::{GenerationConfig, LLMProvider, LLMResponse};
use crate::types::{ContentBlock, Message, Role, StopReason, TokenUsage};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

pub struct OpenAICompatibleProvider {
    client: Client,
    api_base: String,
    api_key: Option<String>,
}

impl OpenAICompatibleProvider {
    pub fn new(api_base: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_base,
            api_key,
        }
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<Value> {
        messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                };

                let content: Vec<Value> = msg
                    .content
                    .into_iter()
                    .map(|block| match block {
                        ContentBlock::Text { text } => {
                            json!({ "type": "text", "text": text })
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            json!({
                                "type": "tool_use",
                                "id": id,
                                "name": name,
                                "input": input
                            })
                        }
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => {
                            json!({
                                "type": "tool_result",
                                "tool_use_id": tool_use_id,
                                "content": content,
                                "is_error": is_error
                            })
                        }
                    })
                    .collect();

                json!({ "role": role, "content": content })
            })
            .collect()
    }
}

#[async_trait]
impl LLMProvider for OpenAICompatibleProvider {
    async fn chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Vec<Value>,
        config: &GenerationConfig,
    ) -> Result<LLMResponse> {
        let url = format!("{}/chat/completions", self.api_base);

        let mut request_body = json!({
            "model": config.model,
            "messages": self.convert_messages(messages),
            "temperature": config.temperature,
        });

        if let Some(max_tokens) = config.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }

        if !tools.is_empty() {
            request_body["tools"] = json!(tools);
        }

        let mut req = self.client.post(&url).json(&request_body);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send().await?;
        let response_json: Value = response.json().await?;

        // Parse response (simplified for now)
        let choice = &response_json["choices"][0];
        let message = &choice["message"];

        let content = if let Some(text) = message["content"].as_str() {
            vec![ContentBlock::Text {
                text: text.to_string(),
            }]
        } else {
            vec![]
        };

        let stop_reason = match choice["finish_reason"].as_str() {
            Some("stop") => StopReason::EndTurn,
            Some("length") => StopReason::MaxTokens,
            _ => StopReason::EndTurn,
        };

        let usage = TokenUsage {
            input_tokens: response_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: response_json["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            stop_reason,
            usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;

    #[test]
    fn test_convert_messages() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234/v1".to_string(),
            None,
        );

        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        }];

        let converted = provider.convert_messages(messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "user");
    }
}
