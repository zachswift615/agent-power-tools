use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use tokio::fs;

pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating or overwriting it"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        let path = Path::new(file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, content).await?;

        Ok(ToolResult {
            content: format!("Successfully wrote to {}", file_path),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_write_file() {
        let temp_path = "/tmp/synthia_test_write.txt";

        let tool = WriteTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path,
                "content": "new content"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);

        let content = fs::read_to_string(temp_path).await.unwrap();
        assert_eq!(content, "new content");

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let temp_path = "/tmp/synthia_test_nested/subdir/write.txt";

        let tool = WriteTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path,
                "content": "nested content"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);

        let content = fs::read_to_string(temp_path).await.unwrap();
        assert_eq!(content, "nested content");

        // Cleanup
        fs::remove_dir_all("/tmp/synthia_test_nested").await.unwrap();
    }

    #[tokio::test]
    async fn test_write_overwrites_existing_file() {
        let temp_path = "/tmp/synthia_test_overwrite.txt";

        // Create file with initial content
        fs::write(temp_path, "initial content").await.unwrap();

        // Verify initial content
        let initial = fs::read_to_string(temp_path).await.unwrap();
        assert_eq!(initial, "initial content");

        // Overwrite with new content
        let tool = WriteTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path,
                "content": "new content"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);

        // Verify content was overwritten
        let content = fs::read_to_string(temp_path).await.unwrap();
        assert_eq!(content, "new content");

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }
}
