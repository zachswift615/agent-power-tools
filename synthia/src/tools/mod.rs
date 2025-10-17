pub mod bash;
pub mod edit;
pub mod read;
pub mod registry;
pub mod write;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, params: Value) -> Result<ToolResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock"
        }
        fn description(&self) -> &str {
            "A mock tool"
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({})
        }
        async fn execute(&self, _params: Value) -> Result<ToolResult> {
            Ok(ToolResult {
                content: "mock result".to_string(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_tool_trait() {
        let tool = MockTool;
        assert_eq!(tool.name(), "mock");
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(!result.is_error);
    }
}
