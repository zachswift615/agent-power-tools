use super::{Tool, ToolResult};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result<()> {
        let tool_name = tool.name().to_string();
        if self.tools.contains_key(&tool_name) {
            return Err(anyhow!(
                "Tool name collision: '{}' is already registered",
                tool_name
            ));
        }
        self.tools.insert(tool_name, tool);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub async fn execute(&self, name: &str, params: Value) -> Result<ToolResult> {
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;
        tool.execute(params).await
    }

    pub fn definitions(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.parameters_schema(),
                })
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test"
        }
        fn description(&self) -> &str {
            "Test tool"
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({})
        }
        async fn execute(&self, _params: Value) -> Result<ToolResult> {
            Ok(ToolResult {
                content: "executed".to_string(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool)).unwrap();

        let result = registry
            .execute("test", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result.content, "executed");
    }

    #[tokio::test]
    async fn test_registry_missing_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("missing", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_collision_detection() {
        let mut registry = ToolRegistry::new();

        // First registration should succeed
        let result = registry.register(Arc::new(TestTool));
        assert!(result.is_ok());

        // Second registration with same name should fail
        let result = registry.register(Arc::new(TestTool));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Tool name collision"));
    }

    #[test]
    fn test_default_trait() {
        let registry = ToolRegistry::default();
        assert_eq!(registry.tools.len(), 0);
    }
}
