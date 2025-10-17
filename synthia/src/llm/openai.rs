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
        let mut result = Vec::new();

        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
            };

            // Separate text content and tool blocks
            let mut text_parts = Vec::new();
            let mut tool_calls = Vec::new();
            let mut tool_results = Vec::new();

            for block in msg.content {
                match block {
                    ContentBlock::Text { text } => {
                        text_parts.push(text);
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        tool_calls.push(json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string())
                            }
                        }));
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => {
                        tool_results.push((tool_use_id, content, is_error));
                    }
                }
            }

            // Build the message based on what content we have
            if !tool_results.is_empty() {
                // Tool results become separate "tool" role messages
                for (tool_use_id, content, _is_error) in tool_results {
                    result.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_use_id,
                        "content": content
                    }));
                }
            } else if !tool_calls.is_empty() {
                // Assistant message with tool calls
                let mut message = json!({
                    "role": role,
                    "tool_calls": tool_calls
                });

                // Add text content if present
                if !text_parts.is_empty() {
                    message["content"] = json!(text_parts.join("\n"));
                }

                result.push(message);
            } else if !text_parts.is_empty() {
                // Regular text message
                result.push(json!({
                    "role": role,
                    "content": text_parts.join("\n")
                }));
            }
        }

        result
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

        // C1: Safe JSON parsing with proper error handling
        let choices = response_json
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let choice = choices
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("Empty choices array"))?;

        let message = choice
            .get("message")
            .ok_or_else(|| anyhow::anyhow!("No message in choice"))?;

        // Parse content blocks
        let mut content = Vec::new();

        // C2: Parse text content
        if let Some(text) = message.get("content").and_then(|c| c.as_str()) {
            if !text.is_empty() {
                content.push(ContentBlock::Text {
                    text: text.to_string(),
                });
            }
        }

        // C2: Parse tool_calls into ContentBlock::ToolUse
        if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
            for tool_call in tool_calls {
                let id = tool_call
                    .get("id")
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Tool call missing id"))?
                    .to_string();

                let function = tool_call
                    .get("function")
                    .ok_or_else(|| anyhow::anyhow!("Tool call missing function"))?;

                let name = function
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Tool call function missing name"))?
                    .to_string();

                let arguments_str = function
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Tool call function missing arguments"))?;

                // Parse arguments JSON string
                let input: Value = serde_json::from_str(arguments_str)
                    .unwrap_or_else(|_| json!({}));

                content.push(ContentBlock::ToolUse { id, name, input });
            }
        }

        // C4: Handle finish_reason correctly
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|fr| fr.as_str())
            .unwrap_or("stop");

        let stop_reason = match finish_reason {
            "stop" => StopReason::EndTurn,
            "length" => StopReason::MaxTokens,
            "tool_calls" => StopReason::StopSequence, // Continue processing for tool calls
            _ => StopReason::EndTurn,
        };

        // C1: Safe usage parsing
        let usage = TokenUsage {
            input_tokens: response_json
                .get("usage")
                .and_then(|u| u.get("prompt_tokens"))
                .and_then(|pt| pt.as_u64())
                .unwrap_or(0) as u32,
            output_tokens: response_json
                .get("usage")
                .and_then(|u| u.get("completion_tokens"))
                .and_then(|ct| ct.as_u64())
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
    fn test_convert_messages_text() {
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
        assert_eq!(converted[0]["content"], "Hello");
    }

    #[test]
    fn test_convert_messages_with_tool_use() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234/v1".to_string(),
            None,
        );

        let messages = vec![Message {
            role: Role::Assistant,
            content: vec![
                ContentBlock::Text {
                    text: "I'll use a tool".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "call_123".to_string(),
                    name: "get_weather".to_string(),
                    input: json!({"city": "San Francisco"}),
                },
            ],
        }];

        let converted = provider.convert_messages(messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "assistant");
        assert_eq!(converted[0]["content"], "I'll use a tool");

        let tool_calls = converted[0]["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["id"], "call_123");
        assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_convert_messages_with_tool_result() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234/v1".to_string(),
            None,
        );

        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "call_123".to_string(),
                content: "Temperature: 72F".to_string(),
                is_error: false,
            }],
        }];

        let converted = provider.convert_messages(messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "tool");
        assert_eq!(converted[0]["tool_call_id"], "call_123");
        assert_eq!(converted[0]["content"], "Temperature: 72F");
    }
}
