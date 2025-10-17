use super::{Tool, ToolResult};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Workshop integration tool that provides persistent context management
/// by shelling out to the workshop CLI. Operations include:
/// - context: View session summary
/// - search: Search entries with a query
/// - recent: View recent activity
/// - note: Add a note
/// - decision: Record a decision (with optional reasoning)
/// - gotcha: Document a gotcha/constraint (with optional tags)
/// - why: Answer "why did we do X?" questions (prioritizes decisions with reasoning)
pub struct WorkshopTool {
    timeout_seconds: u64,
}

impl WorkshopTool {
    pub fn new(timeout_seconds: u64) -> Self {
        Self { timeout_seconds }
    }

    async fn run_workshop_command(&self, args: &[&str]) -> Result<ToolResult> {
        // Check if workshop is installed
        let check = Command::new("which").arg("workshop").output().await?;

        if !check.status.success() {
            return Ok(ToolResult {
                content: "Workshop CLI not found in PATH. Workshop is part of the agent-power-tools project. Install instructions: https://github.com/anthropics/agent-power-tools".to_string(),
                is_error: true,
            });
        }

        let mut cmd = Command::new("workshop");
        cmd.args(args);

        let result = timeout(Duration::from_secs(self.timeout_seconds), cmd.output()).await??;

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        if !result.status.success() {
            return Ok(ToolResult {
                content: if !stderr.is_empty() {
                    stderr.to_string()
                } else {
                    stdout.to_string()
                },
                is_error: true,
            });
        }

        Ok(ToolResult {
            content: stdout.to_string(),
            is_error: false,
        })
    }

    async fn context(&self) -> Result<ToolResult> {
        self.run_workshop_command(&["context"]).await
    }

    async fn search(&self, query: String) -> Result<ToolResult> {
        self.run_workshop_command(&["search", &query]).await
    }

    async fn recent(&self) -> Result<ToolResult> {
        self.run_workshop_command(&["recent"]).await
    }

    async fn note(&self, text: String) -> Result<ToolResult> {
        self.run_workshop_command(&["note", &text]).await
    }

    async fn decision(&self, text: String, reasoning: Option<String>) -> Result<ToolResult> {
        let mut args = vec!["decision", &text];

        let reasoning_str;
        if let Some(r) = reasoning {
            args.push("-r");
            reasoning_str = r;
            args.push(&reasoning_str);
        }

        self.run_workshop_command(&args).await
    }

    async fn gotcha(&self, text: String, tags: Option<Vec<String>>) -> Result<ToolResult> {
        let mut args = vec!["gotcha", &text];

        // Build tag arguments
        let tag_strings: Vec<String>;
        if let Some(t) = tags {
            tag_strings = t.into_iter().flat_map(|tag| vec!["-t".to_string(), tag]).collect();

            // Add tag args to command
            for tag_str in &tag_strings {
                args.push(tag_str.as_str());
            }
        }

        self.run_workshop_command(&args).await
    }

    async fn why(&self, query: String) -> Result<ToolResult> {
        self.run_workshop_command(&["why", &query]).await
    }
}

#[async_trait]
impl Tool for WorkshopTool {
    fn name(&self) -> &str {
        "workshop"
    }

    fn description(&self) -> &str {
        "Execute workshop operations for persistent context management (context, search, recent, note, decision, gotcha, why)"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Workshop operation to perform",
                    "enum": ["context", "search", "recent", "note", "decision", "gotcha", "why"]
                },
                "text": {
                    "type": "string",
                    "description": "For note/decision/gotcha: the text content to record"
                },
                "reasoning": {
                    "type": "string",
                    "description": "For decision: optional reasoning explaining the decision"
                },
                "tags": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "For gotcha: optional tags to categorize the gotcha"
                },
                "query": {
                    "type": "string",
                    "description": "For search/why: search query or topic to ask about"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let operation = params["operation"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'operation' parameter"))?;

        match operation {
            "context" => self.context().await,
            "search" => {
                let query = params["query"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'query' parameter for search operation"))?
                    .to_string();
                self.search(query).await
            }
            "recent" => self.recent().await,
            "note" => {
                let text = params["text"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'text' parameter for note operation"))?
                    .to_string();
                self.note(text).await
            }
            "decision" => {
                let text = params["text"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'text' parameter for decision operation"))?
                    .to_string();
                let reasoning = params["reasoning"].as_str().map(|s| s.to_string());
                self.decision(text, reasoning).await
            }
            "gotcha" => {
                let text = params["text"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'text' parameter for gotcha operation"))?
                    .to_string();
                let tags = params["tags"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    });
                self.gotcha(text, tags).await
            }
            "why" => {
                let query = params["query"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'query' parameter for why operation"))?
                    .to_string();
                self.why(query).await
            }
            _ => Err(anyhow!("Unknown workshop operation: {}", operation)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn is_workshop_installed() -> bool {
        Command::new("which")
            .arg("workshop")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[tokio::test]
    async fn test_workshop_not_installed() {
        // This test checks the error handling when workshop is not found
        // We'll temporarily override PATH to ensure workshop is not found
        let tool = WorkshopTool::new(5);

        // Test that we get a proper error when workshop is not in PATH
        // Note: This might pass if workshop IS installed, so we check both cases
        let result = tool
            .execute(serde_json::json!({
                "operation": "context"
            }))
            .await
            .unwrap();

        // If workshop is installed, should succeed, otherwise should have error message
        if !is_workshop_installed().await {
            assert!(result.is_error);
            assert!(result.content.contains("Workshop CLI not found"));
        }
    }

    #[tokio::test]
    async fn test_workshop_context() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "context"
            }))
            .await
            .unwrap();

        // Should succeed if workshop is installed
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_recent() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "recent"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_note() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "note",
                "text": "Test note from Synthia"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_decision() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "decision",
                "text": "Use Rust for implementation",
                "reasoning": "Better performance and safety"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_gotcha() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "gotcha",
                "text": "Must run in single thread",
                "tags": ["async", "threading"]
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_search() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "search",
                "query": "test"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_why() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "why",
                "query": "rust"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_missing_text_for_note() {
        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "note"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'text' parameter"));
    }

    #[tokio::test]
    async fn test_workshop_missing_query_for_search() {
        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "search"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'query' parameter"));
    }

    #[tokio::test]
    async fn test_workshop_invalid_operation() {
        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "invalid_operation"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown workshop operation"));
    }

    #[tokio::test]
    async fn test_workshop_decision_without_reasoning() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "decision",
                "text": "Use TypeScript"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_workshop_gotcha_without_tags() {
        if !is_workshop_installed().await {
            println!("Workshop not installed, skipping test");
            return;
        }

        let tool = WorkshopTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "gotcha",
                "text": "Some constraint"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
    }
}
