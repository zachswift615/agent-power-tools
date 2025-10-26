use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use tokio::fs;

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read a file from the filesystem"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["file_path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;

        // Expand tilde and environment variables
        let path = super::expand_path(file_path)?;

        let content = match fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ToolResult {
                    content: format!("File not found: {}", path.display()),
                    is_error: true,
                });
            }
            Err(e) => return Err(e.into()),
        };

        // Prepend file path to content for clarity
        let output = format!("File: {}\n\n{}", path.display(), content);

        Ok(ToolResult {
            content: output,
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_read_file() {
        // Create temp file
        let temp_path = "/tmp/synthia_test_read.txt";
        fs::write(temp_path, "test content").await.unwrap();

        let tool = ReadTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.starts_with("File: /tmp/synthia_test_read.txt"));
        assert!(result.content.contains("test content"));

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_read_missing_file() {
        let tool = ReadTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": "/tmp/nonexistent_file.txt"
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }
}
