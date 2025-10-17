use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }

    async fn try_fd(&self, pattern: &str, path: &str) -> Result<std::process::Output> {
        let mut cmd = Command::new("fd");
        cmd.arg("--type").arg("f"); // Only files
        cmd.arg("--color").arg("never"); // No colors
        cmd.arg(pattern); // Pattern
        cmd.arg(path); // Search path

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd.output().await.map_err(|e| anyhow::anyhow!("fd failed: {}", e))
    }

    async fn try_find(&self, pattern: &str, path: &str) -> Result<std::process::Output> {
        let mut cmd = Command::new("find");
        cmd.arg(path);
        cmd.arg("-type").arg("f");

        // Convert glob pattern to find-compatible pattern
        if pattern.contains('*') {
            cmd.arg("-name").arg(pattern);
        } else {
            cmd.arg("-name").arg(format!("*{}*", pattern));
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd.output().await.map_err(|e| anyhow::anyhow!("find failed: {}", e))
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern (e.g., '*.rs', 'src/**/*.ts'). Returns list of matching file paths."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against (e.g., '*.rs', '**/*.json')"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (defaults to current directory)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let pattern = params["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;

        let path = params["path"].as_str().unwrap_or(".");

        // Try fd first, fall back to find if not available
        let output = match self.try_fd(pattern, path).await {
            Ok(output) => output,
            Err(_) => {
                // fd not available, try find
                self.try_find(pattern, path).await?
            }
        };

        if !output.status.success() && output.stdout.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No such file or directory") {
                return Ok(ToolResult {
                    content: format!("Path not found: {}", path),
                    is_error: true,
                });
            }
            return Ok(ToolResult {
                content: format!("No files found matching pattern: {}", pattern),
                is_error: false,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return Ok(ToolResult {
                content: format!("No files found matching pattern: {}", pattern),
                is_error: false,
            });
        }

        Ok(ToolResult {
            content: stdout.to_string(),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_glob_finds_files() {
        // Create temp files
        let temp_dir = "/tmp/synthia_glob_test";
        fs::create_dir_all(temp_dir).await.unwrap();
        fs::write(format!("{}/test1.txt", temp_dir), "content")
            .await
            .unwrap();
        fs::write(format!("{}/test2.txt", temp_dir), "content")
            .await
            .unwrap();
        fs::write(format!("{}/test.rs", temp_dir), "content")
            .await
            .unwrap();

        let tool = GlobTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": "*.txt",
                "path": temp_dir
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("test1.txt"));
        assert!(result.content.contains("test2.txt"));
        assert!(!result.content.contains("test.rs"));

        // Cleanup
        fs::remove_dir_all(temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_glob_no_matches() {
        let temp_dir = "/tmp/synthia_glob_test_nomatch";
        fs::create_dir_all(temp_dir).await.unwrap();
        fs::write(format!("{}/test.txt", temp_dir), "content")
            .await
            .unwrap();

        let tool = GlobTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": "*.rs",
                "path": temp_dir
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("No files found"));

        // Cleanup
        fs::remove_dir_all(temp_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_glob_pattern_match() {
        let temp_dir = "/tmp/synthia_glob_test_pattern";
        fs::create_dir_all(temp_dir).await.unwrap();
        fs::write(format!("{}/foo.rs", temp_dir), "content")
            .await
            .unwrap();
        fs::write(format!("{}/bar.rs", temp_dir), "content")
            .await
            .unwrap();
        fs::write(format!("{}/baz.txt", temp_dir), "content")
            .await
            .unwrap();

        let tool = GlobTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": "*.rs",
                "path": temp_dir
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("foo.rs"));
        assert!(result.content.contains("bar.rs"));
        assert!(!result.content.contains("baz.txt"));

        // Cleanup
        fs::remove_dir_all(temp_dir).await.unwrap();
    }
}
