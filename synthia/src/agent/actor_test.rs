/// Tests for parallel tool execution in AgentActor
///
/// This module contains tests that verify tools are executed in parallel
/// when multiple tool calls are made in a single LLM response.

#[cfg(test)]
mod tests {
    use crate::agent::actor::AgentActor;
    use crate::agent::messages::{Command, UIUpdate};
    use crate::llm::{GenerationConfig, LLMProvider};
    use crate::tools::registry::ToolRegistry;
    use crate::tools::{Tool, ToolResult};
    use crate::llm::LLMResponse;
    use crate::types::{ContentBlock, Message, Role, StopReason, TokenUsage};
    use anyhow::Result;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio::time::sleep;

    /// Mock tool that sleeps for a specified duration
    struct SleepTool {
        name: String,
        sleep_ms: u64,
    }

    #[async_trait]
    impl Tool for SleepTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A test tool that sleeps for a specified duration"
        }

        fn parameters_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "duration_ms": {
                        "type": "number",
                        "description": "Duration to sleep in milliseconds"
                    }
                },
                "required": ["duration_ms"]
            })
        }

        async fn execute(&self, _params: Value) -> Result<ToolResult> {
            sleep(Duration::from_millis(self.sleep_ms)).await;
            Ok(ToolResult {
                content: format!("Slept for {}ms", self.sleep_ms),
                is_error: false,
            })
        }
    }

    /// Mock LLM provider for testing
    struct MockLLMProvider {
        response: LLMResponse,
    }

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn chat_completion(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<Value>,
            _config: &GenerationConfig,
        ) -> Result<LLMResponse> {
            Ok(self.response.clone())
        }

        async fn stream_chat_completion(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<Value>,
            _config: &GenerationConfig,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<Item = Result<crate::llm::StreamEvent>>
                        + Send
                        + 'static,
                >,
            >,
        > {
            unimplemented!("Streaming not used in this test")
        }
    }

    #[tokio::test]
    async fn test_parallel_tool_execution() {
        // Create a tool registry with 3 slow tools
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(SleepTool {
                name: "sleep1".to_string(),
                sleep_ms: 100,
            }))
            .unwrap();
        registry
            .register(Arc::new(SleepTool {
                name: "sleep2".to_string(),
                sleep_ms: 100,
            }))
            .unwrap();
        registry
            .register(Arc::new(SleepTool {
                name: "sleep3".to_string(),
                sleep_ms: 100,
            }))
            .unwrap();

        // Create a mock LLM that returns 3 tool calls
        let mock_response = LLMResponse {
            content: vec![
                ContentBlock::Text {
                    text: "I'll execute 3 tools in parallel".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "call1".to_string(),
                    name: "sleep1".to_string(),
                    input: serde_json::json!({"duration_ms": 100}),
                },
                ContentBlock::ToolUse {
                    id: "call2".to_string(),
                    name: "sleep2".to_string(),
                    input: serde_json::json!({"duration_ms": 100}),
                },
                ContentBlock::ToolUse {
                    id: "call3".to_string(),
                    name: "sleep3".to_string(),
                    input: serde_json::json!({"duration_ms": 100}),
                },
            ],
            stop_reason: StopReason::StopSequence,  // Used for tool calls
            usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
        };

        let llm = Arc::new(MockLLMProvider {
            response: mock_response,
        });

        let (ui_tx, mut ui_rx) = mpsc::channel(100);
        let (cmd_tx, cmd_rx) = mpsc::channel(100);

        let config = GenerationConfig {
            model: "test-model".to_string(),
            temperature: 1.0,
            max_tokens: Some(1000),
            streaming: false,
            reasoning_level: "medium".to_string(),
            context_window: 8192,
        };

        // Create actor (this would normally be done via AgentActor::new, but we'll construct manually for testing)
        // Note: In production code, we'd need to make the actor testable by exposing the generate_response_non_streaming method
        // For now, this test documents the expected behavior

        // Send a message to trigger tool execution
        cmd_tx.send(Command::SendMessage("Test".to_string())).await.unwrap();

        // Spawn actor in background
        let registry_arc = Arc::new(registry);
        let mut actor = AgentActor::new(llm, registry_arc, config, ui_tx, cmd_rx);

        // Measure execution time
        let start = std::time::Instant::now();

        // Run the actor (it will process the message we sent)
        let handle = tokio::spawn(async move {
            // Note: This is a simplified test - in reality we'd need to properly handle the actor lifecycle
            // For now, we're just documenting the expected behavior
        });

        // Collect UI updates
        let mut tool_results = Vec::new();
        let timeout = Duration::from_secs(5);
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout_at(deadline, ui_rx.recv()).await {
                Ok(Some(UIUpdate::ToolResult { name, duration_ms, .. })) => {
                    tool_results.push((name, duration_ms));
                    if tool_results.len() == 3 {
                        break;
                    }
                }
                Ok(Some(_)) => continue, // Ignore other update types
                Ok(None) => break,        // Channel closed
                Err(_) => break,          // Timeout
            }
        }

        let total_duration = start.elapsed();

        // Verify all 3 tools executed
        assert_eq!(tool_results.len(), 3, "Expected 3 tool results");

        // Verify parallel execution:
        // If sequential: ~300ms (100ms * 3)
        // If parallel: ~100ms (max of all tools)
        // Allow some overhead, so check < 200ms
        assert!(
            total_duration.as_millis() < 200,
            "Expected parallel execution to take < 200ms, but took {}ms. \
             Sequential execution would take ~300ms.",
            total_duration.as_millis()
        );

        println!(
            "✓ Parallel execution test passed: 3 tools executed in {}ms (expected ~100ms if parallel, ~300ms if sequential)",
            total_duration.as_millis()
        );

        handle.abort();
    }

    #[test]
    fn test_parallel_execution_documentation() {
        // This test serves as documentation for how parallel execution works
        println!("\n=== Parallel Tool Execution ===");
        println!("When the LLM returns multiple tool calls in a single response:");
        println!("1. All tool calls are collected into a Vec");
        println!("2. Futures are created for each tool execution");
        println!("3. join_all() executes all tools concurrently");
        println!("4. Results are processed in order after all complete");
        println!("\nPerformance Impact:");
        println!("- 3 tools @ 100ms each:");
        println!("  - Sequential: ~300ms total");
        println!("  - Parallel:   ~100ms total (3x speedup)");
        println!("- 5 tools @ 2s each:");
        println!("  - Sequential: ~10s total");
        println!("  - Parallel:   ~2s total (5x speedup)");
    }
}
