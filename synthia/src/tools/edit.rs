use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs;

pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Replace old_string with new_string in a file"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "String to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "String to replace with"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'file_path' parameter"))?;
        let old_string = params["old_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_string' parameter"))?;
        let new_string = params["new_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_string' parameter"))?;

        // Expand tilde and environment variables
        let path = super::expand_path(file_path)?;

        let content = fs::read_to_string(&path).await?;

        if !content.contains(old_string) {
            return Ok(ToolResult {
                content: format!("String '{}' not found in file", old_string),
                is_error: true,
            });
        }

        let new_content = content.replace(old_string, new_string);
        fs::write(&path, new_content).await?;

        Ok(ToolResult {
            content: format!("Successfully edited {}", path.display()),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_edit_file() {
        let temp_path = "/tmp/synthia_test_edit.txt";
        fs::write(temp_path, "hello world").await.unwrap();

        let tool = EditTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path,
                "old_string": "world",
                "new_string": "Synthia"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);

        let content = fs::read_to_string(temp_path).await.unwrap();
        assert_eq!(content, "hello Synthia");

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_edit_string_not_found() {
        let temp_path = "/tmp/synthia_test_edit_notfound.txt";
        fs::write(temp_path, "hello world").await.unwrap();

        let tool = EditTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path,
                "old_string": "nonexistent",
                "new_string": "replacement"
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("not found"));

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }
}
