use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    async fn try_ripgrep(
        &self,
        pattern: &str,
        path: &str,
        case_insensitive: bool,
        files_with_matches: bool,
        glob: Option<&str>,
    ) -> Result<std::process::Output> {
        let mut cmd = Command::new("rg");

        // Add pattern and path
        cmd.arg(pattern).arg(path);

        // Add flags
        cmd.arg("--color").arg("never"); // No ANSI colors
        cmd.arg("--no-heading"); // Don't group by file

        if case_insensitive {
            cmd.arg("--ignore-case");
        }

        if files_with_matches {
            cmd.arg("--files-with-matches");
        } else {
            cmd.arg("--line-number"); // Show line numbers
        }

        if let Some(g) = glob {
            cmd.arg("--glob").arg(g);
        }

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd.output().await.map_err(|e| anyhow::anyhow!("ripgrep failed: {}", e))
    }

    async fn try_grep(
        &self,
        pattern: &str,
        path: &str,
        case_insensitive: bool,
        files_with_matches: bool,
        glob: Option<&str>,
    ) -> Result<std::process::Output> {
        let mut cmd = Command::new("grep");

        // Add flags
        cmd.arg("-r"); // Recursive

        if case_insensitive {
            cmd.arg("-i");
        }

        if files_with_matches {
            cmd.arg("-l"); // Only filenames
        } else {
            cmd.arg("-n"); // Line numbers
        }

        // Add pattern
        cmd.arg(pattern);

        // Add path
        cmd.arg(path);

        // Note: standard grep doesn't have direct glob support
        // For glob filtering, we'd need to combine with find, which is complex
        if glob.is_some() {
            tracing::warn!("Glob filtering not supported with standard grep, ignoring");
        }

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd.output().await.map_err(|e| anyhow::anyhow!("grep failed: {}", e))
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for patterns in files using ripgrep. Returns matching lines with file paths and line numbers."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (defaults to current directory)"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Perform case-insensitive search (default: false)"
                },
                "files_with_matches": {
                    "type": "boolean",
                    "description": "Only show file names with matches, not match content (default: false)"
                },
                "glob": {
                    "type": "string",
                    "description": "Only search files matching this glob pattern (e.g., '*.rs', '*.ts')"
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
        let case_insensitive = params["case_insensitive"].as_bool().unwrap_or(false);
        let files_with_matches = params["files_with_matches"].as_bool().unwrap_or(false);
        let glob = params["glob"].as_str();

        // Try ripgrep first, fall back to grep if not available
        let output = match self
            .try_ripgrep(pattern, path, case_insensitive, files_with_matches, glob)
            .await
        {
            Ok(output) => output,
            Err(_) => {
                // Ripgrep not available, try standard grep
                self.try_grep(pattern, path, case_insensitive, files_with_matches, glob)
                    .await?
            }
        };

        if !output.status.success() && output.stdout.is_empty() {
            // No matches found
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No such file or directory") {
                return Ok(ToolResult {
                    content: format!("Path not found: {}", path),
                    is_error: true,
                });
            }
            return Ok(ToolResult {
                content: format!("No matches found for pattern: {}", pattern),
                is_error: false,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
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
    async fn test_grep_finds_pattern() {
        // Create temp file with content
        let temp_path = "/tmp/synthia_grep_test.txt";
        fs::write(temp_path, "hello world\nfoo bar\nhello again")
            .await
            .unwrap();

        let tool = GrepTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": "hello",
                "path": temp_path
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("hello world"));
        assert!(result.content.contains("hello again"));
        assert!(!result.content.contains("foo bar"));

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let temp_path = "/tmp/synthia_grep_test_nomatch.txt";
        fs::write(temp_path, "hello world").await.unwrap();

        let tool = GrepTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": "nonexistent",
                "path": temp_path
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("No matches found"));

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let temp_path = "/tmp/synthia_grep_test_case.txt";
        fs::write(temp_path, "Hello World\nFOO BAR").await.unwrap();

        let tool = GrepTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": "hello",
                "path": temp_path,
                "case_insensitive": true
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Hello World"));

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_grep_files_with_matches() {
        let temp_path = "/tmp/synthia_grep_test_files.txt";
        fs::write(temp_path, "hello world").await.unwrap();

        let tool = GrepTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": "hello",
                "path": temp_path,
                "files_with_matches": true
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains(temp_path));
        assert!(!result.content.contains("hello world")); // Should only show filename

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }
}
