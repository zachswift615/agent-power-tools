use super::json_parser::JsonParser;
use super::provider::{GenerationConfig, LLMProvider, LLMResponse, StreamEvent, StreamResult};
use crate::types::{ContentBlock, Message, Role, StopReason, TokenUsage};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::pin::Pin;

pub struct OpenAICompatibleProvider {
    client: Client,
    api_base: String,
    api_key: Option<String>,
    json_parser: JsonParser,
}

impl OpenAICompatibleProvider {
    pub fn new(api_base: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_base,
            api_key,
            json_parser: JsonParser::new(),
        }
    }

    fn convert_messages(&self, messages: Vec<Message>, reasoning_level: &str) -> Vec<Value> {
        let mut result = Vec::new();

        for (idx, msg) in messages.iter().enumerate() {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
            };

            // Separate text content and tool blocks
            let mut text_parts = Vec::new();
            let mut tool_calls = Vec::new();
            let mut tool_results = Vec::new();

            for block in &msg.content {
                match block {
                    ContentBlock::Text { text } => {
                        text_parts.push(text.clone());
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
                        tool_results.push((tool_use_id.clone(), content.clone(), *is_error));
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
                let mut content = text_parts.join("\n");

                // Inject reasoning level into first system message
                if idx == 0 && role == "system" {
                    content = format!("{}\n\nReasoning: {}", content, reasoning_level);
                }

                result.push(json!({
                    "role": role,
                    "content": content
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

        let converted_messages = self.convert_messages(messages, &config.reasoning_level);

        // Log the first message to verify reasoning level injection
        if let Some(first_msg) = converted_messages.first() {
            tracing::debug!("First message after conversion: {}", serde_json::to_string_pretty(&first_msg).unwrap_or_else(|_| "{}".to_string()));
        }

        let mut request_body = json!({
            "model": config.model,
            "messages": converted_messages,
            "temperature": config.temperature,
        });

        if let Some(max_tokens) = config.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }

        if !tools.is_empty() {
            // Convert tool definitions to OpenAI format
            // Tools from registry come as: {"name": "...", "description": "...", "input_schema": {...}}
            // OpenAI expects: {"type": "function", "function": {"name": "...", "description": "...", "parameters": {...}}}
            let openai_tools: Vec<Value> = tools
                .into_iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool["name"],
                        "description": tool["description"],
                        "parameters": tool["input_schema"]
                    }
                }))
                .collect();
            request_body["tools"] = json!(openai_tools);
        }

        let mut req = self.client.post(&url).json(&request_body);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send().await?;
        let response_json: Value = response.json().await?;

        // Debug: Log the response to see what LM Studio is returning
        tracing::debug!("LM Studio response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_else(|_| "Failed to serialize".to_string()));

        // C1: Safe JSON parsing with proper error handling
        let choices = response_json
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| anyhow::anyhow!("No choices in response. Full response: {}", serde_json::to_string_pretty(&response_json).unwrap_or_else(|_| "{}".to_string())))?;

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

                // Parse arguments JSON string using robust parser
                let input: Value = self.json_parser.parse_robust(arguments_str)
                    .unwrap_or_else(|e| {
                        tracing::error!(
                            "Failed to parse tool arguments for '{}': {}\nRaw: {}",
                            name, e, arguments_str
                        );
                        json!({})
                    });

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

    async fn stream_chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Vec<Value>,
        config: &GenerationConfig,
    ) -> Result<StreamResult> {
        let url = format!("{}/chat/completions", self.api_base);

        let mut request_body = json!({
            "model": config.model,
            "messages": self.convert_messages(messages, &config.reasoning_level),
            "temperature": config.temperature,
            "stream": true,
        });

        if let Some(max_tokens) = config.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }

        if !tools.is_empty() {
            let openai_tools: Vec<Value> = tools
                .into_iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool["name"],
                        "description": tool["description"],
                        "parameters": tool["input_schema"]
                    }
                }))
                .collect();
            request_body["tools"] = json!(openai_tools);
        }

        let mut req = self.client.post(&url).json(&request_body);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("HTTP error {}: {}", status, error_text));
        }

        // Convert response to SSE stream
        let stream = response.bytes_stream();
        let event_stream = Self::parse_sse_stream(stream);

        Ok(Box::pin(event_stream))
    }
}

impl OpenAICompatibleProvider {
    fn parse_sse_stream(
        stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    ) -> impl Stream<Item = Result<StreamEvent>> + Send {
        // Use a shared state approach
        use futures::stream::unfold;

        struct State {
            buffer: String,
            tool_calls: HashMap<String, (String, String)>,
            stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
        }

        let initial_state = State {
            buffer: String::new(),
            tool_calls: HashMap::new(),
            stream: Box::pin(stream),
        };

        unfold(initial_state, |mut state| async move {
            loop {
                // Try to process buffered data first
                if let Some(pos) = state.buffer.find("\n\n") {
                    let message = state.buffer[..pos].to_string();
                    state.buffer = state.buffer[pos + 2..].to_string();

                    // Parse SSE format: "data: {...}"
                    for line in message.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data.trim() == "[DONE]" {
                                let usage = TokenUsage {
                                    input_tokens: 0,
                                    output_tokens: 0,
                                };
                                return Some((Ok(StreamEvent::Done {
                                    stop_reason: StopReason::EndTurn,
                                    usage,
                                }), state));
                            }

                            match serde_json::from_str::<Value>(data) {
                                Ok(json) => {
                                    if let Some(error) = json.get("error") {
                                        return Some((Ok(StreamEvent::Error(
                                            error.get("message")
                                                .and_then(|m| m.as_str())
                                                .unwrap_or("Unknown error")
                                                .to_string()
                                        )), state));
                                    }

                                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                        if let Some(choice) = choices.get(0) {
                                            let delta = choice.get("delta");

                                            // Handle text content
                                            if let Some(content) = delta.and_then(|d| d.get("content")).and_then(|c| c.as_str()) {
                                                if !content.is_empty() {
                                                    return Some((Ok(StreamEvent::TextDelta(content.to_string())), state));
                                                }
                                            }

                                            // Handle tool calls
                                            if let Some(tool_calls_array) = delta.and_then(|d| d.get("tool_calls")).and_then(|tc| tc.as_array()) {
                                                for tool_call in tool_calls_array {
                                                    let id = tool_call.get("id").and_then(|i| i.as_str()).map(|s| s.to_string());
                                                    let function = tool_call.get("function");

                                                    if let Some(id) = id {
                                                        if let Some(name) = function.and_then(|f| f.get("name")).and_then(|n| n.as_str()) {
                                                            state.tool_calls.insert(id.clone(), (name.to_string(), String::new()));
                                                            return Some((Ok(StreamEvent::ToolCallStart {
                                                                id: id.clone(),
                                                                name: name.to_string(),
                                                            }), state));
                                                        }
                                                    }

                                                    if let Some(args) = function.and_then(|f| f.get("arguments")).and_then(|a| a.as_str()) {
                                                        let tool_id = if let Some(explicit_id) = tool_call.get("id").and_then(|i| i.as_str()) {
                                                            explicit_id.to_string()
                                                        } else {
                                                            state.tool_calls.keys().next().cloned().unwrap_or_default()
                                                        };

                                                        if let Some((_, accumulated_args)) = state.tool_calls.get_mut(&tool_id) {
                                                            accumulated_args.push_str(args);
                                                            return Some((Ok(StreamEvent::ToolCallDelta {
                                                                id: tool_id.clone(),
                                                                arguments_delta: args.to_string(),
                                                            }), state));
                                                        }
                                                    }
                                                }
                                            }

                                            // Check finish_reason
                                            if let Some(finish_reason) = choice.get("finish_reason").and_then(|fr| fr.as_str()) {
                                                let stop_reason = match finish_reason {
                                                    "stop" => StopReason::EndTurn,
                                                    "length" => StopReason::MaxTokens,
                                                    "tool_calls" => StopReason::StopSequence,
                                                    _ => StopReason::EndTurn,
                                                };

                                                let usage = json.get("usage").map(|u| TokenUsage {
                                                    input_tokens: u.get("prompt_tokens").and_then(|pt| pt.as_u64()).unwrap_or(0) as u32,
                                                    output_tokens: u.get("completion_tokens").and_then(|ct| ct.as_u64()).unwrap_or(0) as u32,
                                                }).unwrap_or_else(|| TokenUsage { input_tokens: 0, output_tokens: 0 });

                                                return Some((Ok(StreamEvent::Done {
                                                    stop_reason,
                                                    usage,
                                                }), state));
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to parse SSE chunk: {} (data: {})", e, data);
                                }
                            }
                        }
                    }
                    continue;
                }

                // Need more data, fetch next chunk
                match state.stream.next().await {
                    Some(Ok(chunk)) => {
                        state.buffer.push_str(&String::from_utf8_lossy(&chunk));
                        continue;
                    }
                    Some(Err(e)) => {
                        return Some((Err(anyhow!("Stream error: {}", e)), state));
                    }
                    None => {
                        // Stream ended
                        return None;
                    }
                }
            }
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

        let converted = provider.convert_messages(messages, "medium");
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "user");
        assert_eq!(converted[0]["content"], "Hello");
    }

    #[test]
    fn test_reasoning_level_injection_into_system_message() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234/v1".to_string(),
            None,
        );

        let messages = vec![Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: "You are a helpful assistant.".to_string(),
            }],
        }];

        let converted = provider.convert_messages(messages, "high");
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "system");
        assert_eq!(converted[0]["content"], "You are a helpful assistant.\n\nReasoning: high");
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

        let converted = provider.convert_messages(messages, "medium");
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

        let converted = provider.convert_messages(messages, "medium");
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "tool");
        assert_eq!(converted[0]["tool_call_id"], "call_123");
        assert_eq!(converted[0]["content"], "Temperature: 72F");
    }

    #[tokio::test]
    async fn test_parse_sse_stream_text_delta() {
        use bytes::Bytes;
        use futures::stream;
        use futures::pin_mut;

        // Simulate SSE stream with text deltas
        let sse_data = vec![
            Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n"),
            Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n"),
            Bytes::from("data: {\"choices\":[{\"finish_reason\":\"stop\"}]}\n\n"),
            Bytes::from("data: [DONE]\n\n"),
        ];

        let byte_stream = stream::iter(sse_data.into_iter().map(Ok::<_, reqwest::Error>));
        let event_stream = OpenAICompatibleProvider::parse_sse_stream(byte_stream);
        pin_mut!(event_stream);

        // First event: "Hello"
        let event1 = event_stream.next().await.unwrap().unwrap();
        assert!(matches!(event1, StreamEvent::TextDelta(ref s) if s == "Hello"));

        // Second event: " world"
        let event2 = event_stream.next().await.unwrap().unwrap();
        assert!(matches!(event2, StreamEvent::TextDelta(ref s) if s == " world"));

        // Third event: Done with stop reason
        let event3 = event_stream.next().await.unwrap().unwrap();
        assert!(matches!(event3, StreamEvent::Done { stop_reason: StopReason::EndTurn, .. }));

        // Fourth event: [DONE] marker
        let event4 = event_stream.next().await.unwrap().unwrap();
        assert!(matches!(event4, StreamEvent::Done { .. }));
    }

    #[tokio::test]
    async fn test_parse_sse_stream_error() {
        use bytes::Bytes;
        use futures::stream;
        use futures::pin_mut;

        let sse_data = vec![
            Bytes::from("data: {\"error\":{\"message\":\"API error occurred\"}}\n\n"),
        ];

        let byte_stream = stream::iter(sse_data.into_iter().map(Ok::<_, reqwest::Error>));
        let event_stream = OpenAICompatibleProvider::parse_sse_stream(byte_stream);
        pin_mut!(event_stream);

        let event = event_stream.next().await.unwrap().unwrap();
        assert!(matches!(event, StreamEvent::Error(ref s) if s == "API error occurred"));
    }
}
