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

    /// Detect excessive code duplication by finding repeated line sequences
    /// Returns true if content has too many repeated blocks (likely a generation loop)
    fn has_excessive_duplication(content: &str) -> bool {
        const MIN_BLOCK_SIZE: usize = 10; // Minimum lines to consider as a block
        const MAX_DUPLICATES: usize = 3;   // More than 3 copies = excessive

        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < MIN_BLOCK_SIZE * 2 {
            return false; // Too short to have significant duplication
        }

        // Check for repeated blocks of various sizes
        for block_size in MIN_BLOCK_SIZE..=50 {
            if block_size * 2 > lines.len() {
                break;
            }

            // Look for repeated blocks starting at each position
            let mut block_counts = std::collections::HashMap::new();
            for start in 0..=(lines.len() - block_size) {
                let block = &lines[start..start + block_size];
                let block_str = block.join("\n");
                *block_counts.entry(block_str).or_insert(0) += 1;
            }

            // If any block appears more than MAX_DUPLICATES times, flag it
            for count in block_counts.values() {
                if *count > MAX_DUPLICATES {
                    tracing::warn!(
                        "Detected excessive duplication: {} copies of {}-line block",
                        count,
                        block_size
                    );
                    return true;
                }
            }
        }

        false
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

        // Safeguard 1: File size validation (prevent catastrophic file generation)
        const MAX_FILE_SIZE: usize = 100 * 1024; // 100 KB
        if content.len() > MAX_FILE_SIZE {
            return Ok(ToolResult {
                content: format!(
                    "Error: Content size ({} bytes) exceeds maximum allowed size ({} bytes / {} KB). \
                    This usually indicates a code generation loop. Please break this into smaller files \
                    or use the 'edit' tool to modify existing files incrementally.",
                    content.len(),
                    MAX_FILE_SIZE,
                    MAX_FILE_SIZE / 1024
                ),
                is_error: true,
            });
        }

        // Safeguard 2: Duplicate code detection (catch repetitive generation)
        if Self::has_excessive_duplication(content) {
            return Ok(ToolResult {
                content: format!(
                    "Error: Content contains excessive duplicate code blocks (detected repeated patterns). \
                    This usually indicates a code generation loop. Please review the content and avoid \
                    generating the same code multiple times. File not written to prevent corruption."
                ),
                is_error: true,
            });
        }

        // Expand tilde and environment variables
        let path = super::expand_path(file_path)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&path, content).await?;

        Ok(ToolResult {
            content: format!("Successfully wrote to {}", path.display()),
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
