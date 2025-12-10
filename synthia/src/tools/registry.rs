use super::cache::ToolCache;
use super::{Tool, ToolResult};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use crate::agent::messages::UIUpdate;
use crate::permission_manager::{PermissionManager, PermissionDecision};
use std::sync::Mutex;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    cache: ToolCache,
    ui_tx: Option<Sender<UIUpdate>>,
    permission_manager: Arc<Mutex<PermissionManager>>,
}

impl ToolRegistry {
    pub fn new(permission_manager: Arc<Mutex<PermissionManager>>) -> Self {
        Self {
            tools: HashMap::new(),
            cache: ToolCache::new(100), // Cache last 100 results
            ui_tx: None,
            permission_manager,
        }
    }

    pub fn set_ui_sender(&mut self, ui_tx: Sender<UIUpdate>) {
        self.ui_tx = Some(ui_tx);
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
        // 1. Check permission first
        let decision = self.permission_manager
            .lock()
            .map_err(|e| anyhow!("Failed to acquire permission manager lock: {}", e))?
            .check_permission(name, &params);

        tracing::debug!("Permission check for tool '{}': {:?}", name, decision);

        match decision {
            PermissionDecision::Deny => {
                tracing::warn!("Tool '{}' denied by permission system", name);
                return Ok(ToolResult {
                    content: "Operation denied by permissions".to_string(),
                    is_error: true,
                });
            }
            PermissionDecision::Allow => {
                // For Allow: bypass approval flow, go straight to cache/execute
                // For edit/write: show informational diff
                if (name == "edit" || name == "write") && self.ui_tx.is_some() {
                    tracing::debug!("Tool '{}' auto-approved, showing informational diff", name);

                    // Compute diff for informational display
                    let diff_result = if name == "edit" {
                        self.compute_edit_diff(&params).await
                    } else {
                        self.compute_write_diff(&params).await
                    };

                    if let Ok(diff) = diff_result {
                        if let Some(ui_tx) = &self.ui_tx {
                            let _ = ui_tx
                                .send(UIUpdate::InformationalDiff {
                                    tool_name: name.to_string(),
                                    file_path: params["file_path"]
                                        .as_str()
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    diff,
                                })
                                .await;
                        }
                    }
                }
                tracing::debug!("Tool '{}' allowed by permission system, bypassing approval", name);
            }
            PermissionDecision::Ask => {
                // For Ask: if edit/write, use approval flow
                if name == "edit" || name == "write" {
                    if self.ui_tx.is_some() {
                        tracing::debug!("Tool '{}' requires approval, routing to approval flow", name);
                        if name == "edit" {
                            return self.execute_edit_with_approval(params).await;
                        } else {
                            return self.execute_write_with_approval(params).await;
                        }
                    }
                } else {
                    // New permission prompt for other tools
                    if let Some(ui_tx) = &self.ui_tx {
                        tracing::debug!("Tool '{}' requires permission prompt", name);
                        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

                        let operation_details = match name {
                            "bash" => {
                                format!("Command: {}\nDirectory: {}",
                                    params["command"].as_str().unwrap_or("unknown"),
                                    std::env::current_dir()
                                        .unwrap_or_default()
                                        .to_string_lossy())
                            }
                            "read" => {
                                format!("Read file: {}",
                                    params["file_path"].as_str().unwrap_or("unknown"))
                            }
                            "git" => {
                                format!("Git command: {}",
                                    params["command"].as_str().unwrap_or("unknown"))
                            }
                            "webfetch" => {
                                format!("Fetch URL: {}",
                                    params["url"].as_str().unwrap_or("unknown"))
                            }
                            _ => format!("Operation: {} with params", name),
                        };

                        let suggested_pattern = self.permission_manager
                            .lock()
                            .map_err(|e| anyhow!("Failed to acquire permission manager lock: {}", e))?
                            .suggest_pattern(name, &params);

                        ui_tx
                            .send(UIUpdate::PermissionPrompt {
                                tool_name: name.to_string(),
                                operation_details,
                                suggested_pattern,
                                response_tx,
                            })
                            .await?;

                        match response_rx.await? {
                            crate::agent::messages::PermissionResponse::Yes => {
                                // Execute once
                                tracing::debug!("Permission granted for tool '{}'", name);
                            }
                            crate::agent::messages::PermissionResponse::YesAndDontAsk(_) => {
                                // Build actual permission pattern from tool and params
                                let pattern = self.permission_manager
                                    .lock()
                                    .map_err(|e| anyhow!("Failed to acquire permission manager lock: {}", e))?
                                    .build_pattern(name, &params);

                                self.permission_manager
                                    .lock()
                                    .map_err(|e| anyhow!("Failed to acquire permission manager lock: {}", e))?
                                    .add_permission(pattern)?;
                                tracing::info!("Permission saved for tool '{}'", name);
                            }
                            crate::agent::messages::PermissionResponse::No => {
                                tracing::debug!("Permission denied by user for tool '{}'", name);
                                return Ok(ToolResult {
                                    content: "Operation cancelled by user".to_string(),
                                    is_error: false,
                                });
                            }
                        }
                    }
                }
                // Non-edit/write tools without UI proceed to normal execution
                tracing::debug!("Tool '{}' requires ask but no UI available, executing normally", name);
            }
        }

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

    async fn compute_edit_diff(&self, params: &Value) -> Result<String> {
        use crate::tools::diff::compute_diff;

        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing file_path"))?;
        let old_string = params["old_string"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing old_string"))?;
        let new_string = params["new_string"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing new_string"))?;

        // Read current file content
        let content = tokio::fs::read_to_string(file_path).await?;

        if !content.contains(old_string) {
            return Err(anyhow!("String '{}' not found in file", old_string));
        }

        // Compute diff
        let new_content = content.replace(old_string, new_string);
        Ok(compute_diff(&content, &new_content))
    }

    async fn compute_write_diff(&self, params: &Value) -> Result<String> {
        use crate::tools::diff::compute_diff;

        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing file_path"))?;
        let new_content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content"))?;

        // Check if file exists
        let old_content = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(_) => String::new(),
        };

        // Compute diff
        if old_content.is_empty() {
            // New file - show as all additions
            Ok(new_content
                .lines()
                .map(|line| format!("+ {}", line))
                .collect::<Vec<_>>()
                .join("\n"))
        } else {
            // Existing file - show diff
            Ok(compute_diff(&old_content, new_content))
        }
    }

    async fn execute_edit_with_approval(&self, params: Value) -> Result<ToolResult> {
        // TODO: Check config.ui.edit_approval before prompting
        // For MVP, always prompt
        use crate::tools::diff::compute_diff;

        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing file_path"))?;
        let old_string = params["old_string"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing old_string"))?;
        let new_string = params["new_string"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing new_string"))?;

        // Read current file content
        let content = tokio::fs::read_to_string(file_path).await?;

        if !content.contains(old_string) {
            return Ok(ToolResult {
                content: format!("String '{}' not found in file", old_string),
                is_error: true,
            });
        }

        // Compute diff
        let new_content = content.replace(old_string, new_string);
        let diff = compute_diff(&content, &new_content);

        // Create approval channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Send preview to UI
        if let Some(ui_tx) = &self.ui_tx {
            ui_tx
                .send(UIUpdate::EditPreview {
                    file_path: file_path.to_string(),
                    old_string: old_string.to_string(),
                    new_string: new_string.to_string(),
                    diff,
                    response_tx,
                })
                .await?;
        }

        // Wait for user response
        match response_rx.await {
            Ok(crate::agent::messages::ApprovalResponse::Approve) => {
                // Execute the edit
                let tool = self.get("edit").ok_or_else(|| anyhow!("Edit tool not found"))?;
                tool.execute(params).await
            }
            Ok(crate::agent::messages::ApprovalResponse::ApproveDontAsk(pattern)) => {
                // Add permission and execute
                self.permission_manager
                    .lock()
                    .map_err(|e| anyhow!("Failed to acquire permission manager lock: {}", e))?
                    .add_permission(pattern)?;

                // Execute the edit
                let tool = self.get("edit").ok_or_else(|| anyhow!("Edit tool not found"))?;
                tool.execute(params).await
            }
            Ok(crate::agent::messages::ApprovalResponse::Reject) => {
                // User rejected
                Ok(ToolResult {
                    content: "Edit cancelled by user".to_string(),
                    is_error: false,
                })
            }
            Err(_) => {
                // Channel closed (user disconnected?)
                Ok(ToolResult {
                    content: "Edit approval cancelled".to_string(),
                    is_error: true,
                })
            }
        }
    }

    async fn execute_write_with_approval(&self, params: Value) -> Result<ToolResult> {
        use crate::tools::diff::compute_diff;

        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing file_path"))?;
        let new_content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content"))?;

        // Check if file exists
        let (old_content, action_label) = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => (content, "OVERWRITE"),
            Err(_) => (String::new(), "CREATE"),
        };

        // Compute diff
        let diff = if old_content.is_empty() {
            // New file - show as all additions
            new_content.lines()
                .map(|line| format!("+ {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            // Existing file - show diff
            compute_diff(&old_content, new_content)
        };

        // Create approval channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Send preview to UI (reuse EditPreview for now)
        if let Some(ui_tx) = &self.ui_tx {
            ui_tx
                .send(UIUpdate::EditPreview {
                    file_path: format!("{} [{}]", file_path, action_label),
                    old_string: if old_content.is_empty() { "[NEW FILE]".to_string() } else { "[EXISTING FILE]".to_string() },
                    new_string: format!("{} lines", new_content.lines().count()),
                    diff,
                    response_tx,
                })
                .await?;
        }

        // Wait for user response
        match response_rx.await {
            Ok(crate::agent::messages::ApprovalResponse::Approve) => {
                // Execute the write
                let tool = self.get("write").ok_or_else(|| anyhow!("Write tool not found"))?;
                tool.execute(params).await
            }
            Ok(crate::agent::messages::ApprovalResponse::ApproveDontAsk(pattern)) => {
                // Add permission and execute
                self.permission_manager
                    .lock()
                    .map_err(|e| anyhow!("Failed to acquire permission manager lock: {}", e))?
                    .add_permission(pattern)?;

                // Execute the write
                let tool = self.get("write").ok_or_else(|| anyhow!("Write tool not found"))?;
                tool.execute(params).await
            }
            Ok(crate::agent::messages::ApprovalResponse::Reject) => {
                // User rejected
                Ok(ToolResult {
                    content: "Write cancelled by user".to_string(),
                    is_error: false,
                })
            }
            Err(_) => {
                // Channel closed (user disconnected?)
                Ok(ToolResult {
                    content: "Write approval cancelled".to_string(),
                    is_error: true,
                })
            }
        }
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
    #[allow(dead_code)]
    pub fn invalidate_file_cache(&self) {
        self.cache.invalidate_tool("read");
        self.cache.invalidate_tool("grep");
        self.cache.invalidate_tool("glob");
        tracing::debug!("Invalidated file-based tool caches");
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.stats()
    }

    /// Clear all cached results
    #[allow(dead_code)]
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        // Create a temporary permission manager for tests
        use std::env;
        let temp_dir = env::temp_dir();
        let project_root = temp_dir.join("test_registry_default");
        let permission_manager = Arc::new(Mutex::new(
            PermissionManager::new(project_root).expect("Failed to create permission manager")
        ));
        Self::new(permission_manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use async_trait::async_trait;

    /// Helper function to create a ToolRegistry for testing
    fn create_test_registry() -> ToolRegistry {
        use std::env;
        let temp_dir = env::temp_dir();
        let project_root = temp_dir.join(format!("test_registry_{}", uuid::Uuid::new_v4()));
        let permission_manager = Arc::new(Mutex::new(
            PermissionManager::new(project_root).expect("Failed to create permission manager")
        ));
        ToolRegistry::new(permission_manager)
    }

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
        let mut registry = create_test_registry();
        registry.register(Arc::new(TestTool)).unwrap();

        let result = registry
            .execute("test", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result.content, "executed");
    }

    #[tokio::test]
    async fn test_registry_missing_tool() {
        let registry = create_test_registry();
        let result = registry.execute("missing", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_collision_detection() {
        let mut registry = create_test_registry();

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
        let mut registry = create_test_registry();

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
        let mut registry = create_test_registry();

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
        let mut registry = create_test_registry();
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
        let mut registry = create_test_registry();

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
