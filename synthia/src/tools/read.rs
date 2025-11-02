use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs;

pub struct ReadTool {
    max_output_chars: usize,
    warn_at_chars: usize,
}

impl ReadTool {
    pub fn new(max_output_chars: usize, warn_at_chars: usize) -> Self {
        Self {
            max_output_chars,
            warn_at_chars,
        }
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

        // Check if file exceeds maximum size
        if content.len() > self.max_output_chars {
            let error_msg = format!(
                "ERROR: File too large to read in full\n\n\
                File: {}\n\
                Size: {} chars (~{} tokens)\n\
                Limit: {} chars (~{} tokens)\n\n\
                Options:\n\
                1. Use grep/rg to search for specific content in the file\n\
                2. Use 'head' to read the first N lines: head -n 100 {}\n\
                3. Use 'tail' to read the last N lines: tail -n 100 {}\n\
                4. Ask me to summarize specific sections or search for keywords\n\
                5. Increase max_read_output_chars in synthia.toml if absolutely necessary\n\n\
                Tip: For large files, it's better to search for what you need rather than reading everything.",
                path.display(),
                content.len(),
                content.len() / 4,
                self.max_output_chars,
                self.max_output_chars / 4,
                path.display(),
                path.display()
            );

            return Ok(ToolResult {
                content: error_msg,
                is_error: true,
            });
        }

        // Warn if file is large (but under the limit)
        let warning = if content.len() > self.warn_at_chars {
            format!(
                "⚠️  WARNING: Large file ({} chars / ~{} tokens)\n\
                Consider using grep/head/tail for large files to reduce token usage.\n\n",
                content.len(),
                content.len() / 4
            )
        } else {
            String::new()
        };

        // Prepend file path and optional warning to content
        let output = format!("File: {}\n\n{}{}", path.display(), warning, content);

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

        let tool = ReadTool::new(200_000, 100_000);
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
        let tool = ReadTool::new(200_000, 100_000);
        let result = tool
            .execute(serde_json::json!({
                "file_path": "/tmp/nonexistent_file.txt"
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }

    #[tokio::test]
    async fn test_read_file_too_large() {
        // Create a large temp file that exceeds limit
        let temp_path = "/tmp/synthia_test_large.txt";
        let large_content = "x".repeat(300); // 300 chars
        fs::write(temp_path, &large_content).await.unwrap();

        let tool = ReadTool::new(100, 50); // Small limits for testing
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("ERROR: File too large to read in full"));
        assert!(result.content.contains("Options:"));

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_read_file_warning() {
        // Create a file that triggers warning but is under limit
        let temp_path = "/tmp/synthia_test_warning.txt";
        let content = "x".repeat(75); // Between warn (50) and max (100)
        fs::write(temp_path, &content).await.unwrap();

        let tool = ReadTool::new(100, 50); // Small limits for testing
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("⚠️  WARNING: Large file"));
        assert!(result.content.contains(&content)); // Content should still be there

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }
}
