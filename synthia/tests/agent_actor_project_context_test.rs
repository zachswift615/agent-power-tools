use synthia::agent::actor::AgentActor;
use synthia::llm::GenerationConfig;
use synthia::tools::registry::ToolRegistry;
use synthia::types::{Role, ContentBlock};
use tokio::sync::mpsc;
use std::sync::Arc;

// Mock LLM provider for testing
mod mock_provider {
    use synthia::llm::{GenerationConfig, LLMProvider, StreamEvent, LLMResponse};
    use synthia::types::{Message, ContentBlock, StopReason, TokenUsage};
    use async_trait::async_trait;
    use anyhow::Result;
    use futures::stream::{self, BoxStream};

    pub struct MockLLMProvider;

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn chat_completion(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<serde_json::Value>,
            _config: &GenerationConfig,
        ) -> Result<LLMResponse> {
            Ok(LLMResponse {
                content: vec![ContentBlock::Text {
                    text: "mock response".to_string(),
                }],
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                },
            })
        }

        async fn stream_chat_completion(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<serde_json::Value>,
            _config: &GenerationConfig,
        ) -> Result<BoxStream<'static, Result<StreamEvent>>> {
            Ok(Box::pin(stream::empty()))
        }
    }
}

#[tokio::test]
async fn test_actor_with_no_project_context() {
    let config = GenerationConfig {
        temperature: 0.7,
        max_tokens: Some(1000),
        model: "test-model".to_string(),
        streaming: false,
        reasoning_level: "medium".to_string(),
        context_window: 100000,
    };
    let llm_provider = Arc::new(mock_provider::MockLLMProvider);
    let tool_registry = Arc::new(ToolRegistry::new());
    let (ui_tx, _ui_rx) = mpsc::channel(32);
    let (_cmd_tx, cmd_rx) = mpsc::channel(32);

    let actor = AgentActor::new(
        llm_provider,
        tool_registry,
        config,
        ui_tx,
        cmd_rx,
        None, // No project context
    );

    // Should have only 1 system message (core prompt)
    assert_eq!(actor.conversation().len(), 1);
    assert!(matches!(actor.conversation()[0].role, Role::System));
}

#[tokio::test]
async fn test_actor_with_project_context() {
    let config = GenerationConfig {
        temperature: 0.7,
        max_tokens: Some(1000),
        model: "test-model".to_string(),
        streaming: false,
        reasoning_level: "medium".to_string(),
        context_window: 100000,
    };
    let llm_provider = Arc::new(mock_provider::MockLLMProvider);
    let tool_registry = Arc::new(ToolRegistry::new());
    let (ui_tx, _ui_rx) = mpsc::channel(32);
    let (_cmd_tx, cmd_rx) = mpsc::channel(32);

    let project_context = Some("Always respond in haiku".to_string());

    let actor = AgentActor::new(
        llm_provider,
        tool_registry,
        config,
        ui_tx,
        cmd_rx,
        project_context,
    );

    // Should have 2 system messages
    assert_eq!(actor.conversation().len(), 2);
    assert!(matches!(actor.conversation()[0].role, Role::System));
    assert!(matches!(actor.conversation()[1].role, Role::System));

    // Second message should contain project instructions wrapped
    if let ContentBlock::Text { text } = &actor.conversation()[1].content[0] {
        assert!(text.contains("<project-instructions>"));
        assert!(text.contains("Always respond in haiku"));
        assert!(text.contains("</project-instructions>"));
    } else {
        panic!("Expected text content block");
    }
}
