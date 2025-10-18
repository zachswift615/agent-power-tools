use super::cache::ToolCache;
use super::{Tool, ToolResult};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    cache: ToolCache,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            cache: ToolCache::new(100), // Cache last 100 results
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
        // Check cache first for deterministic tools
        if Self::is_deterministic(name) {
            if let Some(cached) = self.cache.get(name, &params) {
                tracing::debug!("Tool cache hit: {}", name);
                return Ok(cached);
            }
        }

        // Execute tool
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;
        let result = tool.execute(params.clone()).await?;

        // Cache result if tool is deterministic
        if Self::is_deterministic(name) {
            self.cache.put(name, &params, result.clone());
        }

        Ok(result)
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

    /// Check if a tool is deterministic (same inputs -> same outputs)
    fn is_deterministic(tool_name: &str) -> bool {
        matches!(tool_name, "read" | "grep" | "glob" | "powertools")
    }

    /// Invalidate cache when files change (e.g., after write, edit, git operations)
    pub fn invalidate_file_cache(&self) {
        self.cache.invalidate_tool("read");
        self.cache.invalidate_tool("grep");
        self.cache.invalidate_tool("glob");
        tracing::debug!("Invalidated file-based tool caches");
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.stats()
    }

    /// Clear all cached results
    pub fn clear_cache(&self) {
        self.cache.clear();
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

    #[tokio::test]
    async fn test_cache_deterministic_tools() {
        let mut registry = ToolRegistry::new();

        // Create a simple deterministic tool that tracks execution count
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let exec_count = Arc::new(AtomicU32::new(0));
        let exec_count_clone = exec_count.clone();

        struct CachedTestTool {
            exec_count: Arc<AtomicU32>,
        }

        #[async_trait]
        impl Tool for CachedTestTool {
            fn name(&self) -> &str {
                "read" // Use a deterministic tool name
            }
            fn description(&self) -> &str {
                "Test tool"
            }
            fn parameters_schema(&self) -> Value {
                serde_json::json!({})
            }
            async fn execute(&self, _params: Value) -> Result<ToolResult> {
                self.exec_count.fetch_add(1, Ordering::SeqCst);
                Ok(ToolResult {
                    content: "executed".to_string(),
                    is_error: false,
                })
            }
        }

        registry
            .register(Arc::new(CachedTestTool {
                exec_count: exec_count_clone,
            }))
            .unwrap();

        let params = serde_json::json!({"file": "test.txt"});

        // First execution - cache miss
        let result1 = registry.execute("read", params.clone()).await.unwrap();
        assert_eq!(result1.content, "executed");
        assert_eq!(exec_count.load(Ordering::SeqCst), 1);

        // Second execution with same params - cache hit (shouldn't execute again)
        let result2 = registry.execute("read", params.clone()).await.unwrap();
        assert_eq!(result2.content, "executed");
        assert_eq!(exec_count.load(Ordering::SeqCst), 1); // Still 1!

        // Different params - cache miss
        let result3 = registry
            .execute("read", serde_json::json!({"file": "other.txt"}))
            .await
            .unwrap();
        assert_eq!(result3.content, "executed");
        assert_eq!(exec_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let mut registry = ToolRegistry::new();

        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let exec_count = Arc::new(AtomicU32::new(0));
        let exec_count_clone = exec_count.clone();

        struct InvalidationTestTool {
            exec_count: Arc<AtomicU32>,
        }

        #[async_trait]
        impl Tool for InvalidationTestTool {
            fn name(&self) -> &str {
                "read"
            }
            fn description(&self) -> &str {
                "Test tool"
            }
            fn parameters_schema(&self) -> Value {
                serde_json::json!({})
            }
            async fn execute(&self, _params: Value) -> Result<ToolResult> {
                self.exec_count.fetch_add(1, Ordering::SeqCst);
                Ok(ToolResult {
                    content: "executed".to_string(),
                    is_error: false,
                })
            }
        }

        registry
            .register(Arc::new(InvalidationTestTool {
                exec_count: exec_count_clone,
            }))
            .unwrap();

        let params = serde_json::json!({"file": "test.txt"});

        // Execute and cache
        registry.execute("read", params.clone()).await.unwrap();
        assert_eq!(exec_count.load(Ordering::SeqCst), 1);

        // Cache hit
        registry.execute("read", params.clone()).await.unwrap();
        assert_eq!(exec_count.load(Ordering::SeqCst), 1);

        // Invalidate cache
        registry.invalidate_file_cache();

        // Should execute again after invalidation
        registry.execute("read", params.clone()).await.unwrap();
        assert_eq!(exec_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool)).unwrap();

        // Check initial stats
        let stats = registry.cache_stats();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);

        // Execute doesn't cache non-deterministic tools
        registry.execute("test", serde_json::json!({})).await.unwrap();
        let stats = registry.cache_stats();
        assert_eq!(stats.size, 0);
    }

    #[tokio::test]
    async fn test_non_deterministic_tools_not_cached() {
        let mut registry = ToolRegistry::new();

        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let exec_count = Arc::new(AtomicU32::new(0));
        let exec_count_clone = exec_count.clone();

        struct NonDeterministicTool {
            exec_count: Arc<AtomicU32>,
        }

        #[async_trait]
        impl Tool for NonDeterministicTool {
            fn name(&self) -> &str {
                "bash" // Non-deterministic tool
            }
            fn description(&self) -> &str {
                "Test tool"
            }
            fn parameters_schema(&self) -> Value {
                serde_json::json!({})
            }
            async fn execute(&self, _params: Value) -> Result<ToolResult> {
                self.exec_count.fetch_add(1, Ordering::SeqCst);
                Ok(ToolResult {
                    content: "executed".to_string(),
                    is_error: false,
                })
            }
        }

        registry
            .register(Arc::new(NonDeterministicTool {
                exec_count: exec_count_clone,
            }))
            .unwrap();

        let params = serde_json::json!({"command": "echo test"});

        // Execute twice with same params
        registry.execute("bash", params.clone()).await.unwrap();
        registry.execute("bash", params.clone()).await.unwrap();

        // Should execute both times (not cached)
        assert_eq!(exec_count.load(Ordering::SeqCst), 2);
    }
}
