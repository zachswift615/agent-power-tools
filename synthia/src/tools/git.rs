use super::{Tool, ToolResult};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Git tool that provides common git operations by shelling out to git commands.
/// Each operation is specified via the "operation" parameter:
/// - init: Initialize a new git repository
/// - status: Check working tree status
/// - diff: Show changes (optional: staged, unstaged, or both)
/// - log: Show commit history (configurable limit)
/// - add: Stage files
/// - commit: Create commits with messages
/// - push: Push to remote
pub struct GitTool {
    timeout_seconds: u64,
}

impl GitTool {
    pub fn new(timeout_seconds: u64) -> Self {
        Self { timeout_seconds }
    }

    async fn run_git_command(&self, args: &[&str], cwd: Option<&str>) -> Result<ToolResult> {
        let mut cmd = Command::new("git");
        cmd.args(args);

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let result = timeout(Duration::from_secs(self.timeout_seconds), cmd.output()).await??;

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        // Prepend command for clarity
        let command_str = format!("git {}", args.join(" "));
        let content = if !stderr.is_empty() {
            format!("Command: {}\n\nstdout:\n{}\nstderr:\n{}", command_str, stdout, stderr)
        } else {
            format!("Command: {}\n\n{}", command_str, stdout)
        };

        Ok(ToolResult {
            content,
            is_error: !result.status.success(),
        })
    }

    async fn git_status(&self, cwd: Option<&str>) -> Result<ToolResult> {
        self.run_git_command(&["status"], cwd).await
    }

    async fn git_diff(&self, cwd: Option<&str>, staged: bool) -> Result<ToolResult> {
        let args = if staged {
            vec!["diff", "--cached"]
        } else {
            vec!["diff"]
        };
        self.run_git_command(&args, cwd).await
    }

    async fn git_log(&self, cwd: Option<&str>, limit: Option<u32>) -> Result<ToolResult> {
        let limit_str = limit.unwrap_or(10).to_string();
        let args = vec!["log", "--oneline", "-n", &limit_str];
        self.run_git_command(&args, cwd).await
    }

    async fn git_add(&self, cwd: Option<&str>, files: Vec<String>) -> Result<ToolResult> {
        if files.is_empty() {
            return Err(anyhow!("No files specified for git add"));
        }

        let mut args = vec!["add"];
        let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        args.extend(file_refs);

        self.run_git_command(&args, cwd).await
    }

    async fn git_commit(&self, cwd: Option<&str>, message: String) -> Result<ToolResult> {
        if message.is_empty() {
            return Err(anyhow!("Commit message cannot be empty"));
        }

        let args = vec!["commit", "-m", &message];
        self.run_git_command(&args, cwd).await
    }

    async fn git_push(
        &self,
        cwd: Option<&str>,
        remote: Option<String>,
        branch: Option<String>,
    ) -> Result<ToolResult> {
        let mut args = vec!["push"];

        if let Some(r) = &remote {
            args.push(r.as_str());
            if let Some(b) = &branch {
                args.push(b.as_str());
            }
        }

        self.run_git_command(&args, cwd).await
    }

    async fn git_init(&self, cwd: Option<&str>) -> Result<ToolResult> {
        self.run_git_command(&["init"], cwd).await
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Execute git operations (init, status, diff, log, add, commit, push)"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Git operation to perform",
                    "enum": ["init", "status", "diff", "log", "add", "commit", "push"]
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for git command (optional)"
                },
                "staged": {
                    "type": "boolean",
                    "description": "For diff: show staged changes (default: false)"
                },
                "limit": {
                    "type": "integer",
                    "description": "For log: number of commits to show (default: 10)"
                },
                "files": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "For add: files to stage"
                },
                "message": {
                    "type": "string",
                    "description": "For commit: commit message"
                },
                "remote": {
                    "type": "string",
                    "description": "For push: remote name (default: origin)"
                },
                "branch": {
                    "type": "string",
                    "description": "For push: branch name (optional)"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let operation = params["operation"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'operation' parameter"))?;

        let cwd = params["cwd"].as_str();

        match operation {
            "init" => self.git_init(cwd).await,
            "status" => self.git_status(cwd).await,
            "diff" => {
                let staged = params["staged"].as_bool().unwrap_or(false);
                self.git_diff(cwd, staged).await
            }
            "log" => {
                let limit = params["limit"].as_u64().map(|l| l as u32);
                self.git_log(cwd, limit).await
            }
            "add" => {
                let files = params["files"]
                    .as_array()
                    .ok_or_else(|| anyhow!("Missing 'files' parameter for add operation"))?
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                self.git_add(cwd, files).await
            }
            "commit" => {
                let message = params["message"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'message' parameter for commit operation"))?
                    .to_string();
                self.git_commit(cwd, message).await
            }
            "push" => {
                let remote = params["remote"].as_str().map(|s| s.to_string());
                let branch = params["branch"].as_str().map(|s| s.to_string());
                self.git_push(cwd, remote, branch).await
            }
            _ => Err(anyhow!("Unknown git operation: {}", operation)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tokio::process::Command as TokioCommand;

    async fn create_test_repo() -> Result<PathBuf> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir()
            .join(format!("synthia_git_test_{}_{}", std::process::id(), timestamp));
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir)?;
        }
        fs::create_dir(&temp_dir)?;

        // Initialize git repo
        TokioCommand::new("git")
            .args(&["init"])
            .current_dir(&temp_dir)
            .output()
            .await?;

        // Configure git
        TokioCommand::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&temp_dir)
            .output()
            .await?;

        TokioCommand::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&temp_dir)
            .output()
            .await?;

        Ok(temp_dir)
    }

    async fn cleanup_test_repo(path: &PathBuf) -> Result<()> {
        if path.exists() {
            // Retry cleanup a few times in case of lingering file handles
            for _ in 0..3 {
                match fs::remove_dir_all(path) {
                    Ok(_) => return Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
            // Final attempt
            fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_git_status_not_a_repo() {
        let tool = GitTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "operation": "status",
                "cwd": "/tmp"
            }))
            .await
            .unwrap();

        // Should succeed but indicate not a git repo
        assert!(
            result.content.contains("not a git repository")
                || result.content.contains("Not a git repository")
        );
    }

    #[tokio::test]
    async fn test_git_status_clean() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        let result = tool
            .execute(serde_json::json!({
                "operation": "status",
                "cwd": repo_dir.to_str().unwrap()
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(
            result.content.contains("On branch") || result.content.contains("No commits yet")
        );

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_add_and_commit() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        // Create a test file
        let test_file = repo_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Git add
        let result = tool
            .execute(serde_json::json!({
                "operation": "add",
                "cwd": repo_dir.to_str().unwrap(),
                "files": ["test.txt"]
            }))
            .await
            .unwrap();

        assert!(!result.is_error);

        // Git commit
        let result = tool
            .execute(serde_json::json!({
                "operation": "commit",
                "cwd": repo_dir.to_str().unwrap(),
                "message": "Initial commit"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(
            result.content.contains("Initial commit") || result.content.contains("1 file changed")
        );

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_log() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        // Create and commit a file
        let test_file = repo_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        tool.execute(serde_json::json!({
            "operation": "add",
            "cwd": repo_dir.to_str().unwrap(),
            "files": ["test.txt"]
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "commit",
            "cwd": repo_dir.to_str().unwrap(),
            "message": "Test commit"
        }))
        .await
        .unwrap();

        // Git log
        let result = tool
            .execute(serde_json::json!({
                "operation": "log",
                "cwd": repo_dir.to_str().unwrap(),
                "limit": 5
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Test commit"));

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_diff() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        // Create and commit initial file
        let test_file = repo_dir.join("test.txt");
        fs::write(&test_file, "initial content").unwrap();

        tool.execute(serde_json::json!({
            "operation": "add",
            "cwd": repo_dir.to_str().unwrap(),
            "files": ["test.txt"]
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "commit",
            "cwd": repo_dir.to_str().unwrap(),
            "message": "Initial commit"
        }))
        .await
        .unwrap();

        // Modify file
        fs::write(&test_file, "modified content").unwrap();

        // Git diff (unstaged)
        let result = tool
            .execute(serde_json::json!({
                "operation": "diff",
                "cwd": repo_dir.to_str().unwrap(),
                "staged": false
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("modified") || result.content.contains("diff"));

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_diff_staged() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        // Create and commit initial file
        let test_file = repo_dir.join("test.txt");
        fs::write(&test_file, "initial content").unwrap();

        tool.execute(serde_json::json!({
            "operation": "add",
            "cwd": repo_dir.to_str().unwrap(),
            "files": ["test.txt"]
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "commit",
            "cwd": repo_dir.to_str().unwrap(),
            "message": "Initial commit"
        }))
        .await
        .unwrap();

        // Modify and stage file
        fs::write(&test_file, "modified content").unwrap();

        tool.execute(serde_json::json!({
            "operation": "add",
            "cwd": repo_dir.to_str().unwrap(),
            "files": ["test.txt"]
        }))
        .await
        .unwrap();

        // Git diff --cached (staged)
        let result = tool
            .execute(serde_json::json!({
                "operation": "diff",
                "cwd": repo_dir.to_str().unwrap(),
                "staged": true
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("modified") || result.content.contains("diff"));

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_commit_empty_message() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        let result = tool
            .execute(serde_json::json!({
                "operation": "commit",
                "cwd": repo_dir.to_str().unwrap(),
                "message": ""
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Commit message cannot be empty"));

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_add_no_files() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        let result = tool
            .execute(serde_json::json!({
                "operation": "add",
                "cwd": repo_dir.to_str().unwrap(),
                "files": []
            }))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No files specified"));

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let tool = GitTool::new(5);

        let result = tool
            .execute(serde_json::json!({
                "operation": "invalid_operation"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown git operation"));
    }

    #[tokio::test]
    async fn test_git_log_with_limit() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        // Create multiple commits
        for i in 1..=5 {
            let test_file = repo_dir.join(format!("test{}.txt", i));
            fs::write(&test_file, format!("content {}", i)).unwrap();

            tool.execute(serde_json::json!({
                "operation": "add",
                "cwd": repo_dir.to_str().unwrap(),
                "files": [format!("test{}.txt", i)]
            }))
            .await
            .unwrap();

            tool.execute(serde_json::json!({
                "operation": "commit",
                "cwd": repo_dir.to_str().unwrap(),
                "message": format!("Commit {}", i)
            }))
            .await
            .unwrap();
        }

        // Get log with limit of 3
        let result = tool
            .execute(serde_json::json!({
                "operation": "log",
                "cwd": repo_dir.to_str().unwrap(),
                "limit": 3
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        // Should contain recent commits
        assert!(result.content.contains("Commit"));

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_push_no_remote() {
        let repo_dir = create_test_repo().await.unwrap();
        let tool = GitTool::new(5);

        // Create and commit a file
        let test_file = repo_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        tool.execute(serde_json::json!({
            "operation": "add",
            "cwd": repo_dir.to_str().unwrap(),
            "files": ["test.txt"]
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "commit",
            "cwd": repo_dir.to_str().unwrap(),
            "message": "Test commit"
        }))
        .await
        .unwrap();

        // Try to push (should fail - no remote configured)
        let result = tool
            .execute(serde_json::json!({
                "operation": "push",
                "cwd": repo_dir.to_str().unwrap()
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(
            result.content.contains("No configured push destination")
                || result.content.contains("fatal")
        );

        cleanup_test_repo(&repo_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_git_init() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir()
            .join(format!("synthia_git_init_test_{}_{}", std::process::id(), timestamp));

        // Create directory without initializing git
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir(&temp_dir).unwrap();

        let tool = GitTool::new(5);

        // Initialize git repository
        let result = tool
            .execute(serde_json::json!({
                "operation": "init",
                "cwd": temp_dir.to_str().unwrap()
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Initialized") || result.content.contains("init"));

        // Verify it's a git repository by running git status
        let status_result = tool
            .execute(serde_json::json!({
                "operation": "status",
                "cwd": temp_dir.to_str().unwrap()
            }))
            .await
            .unwrap();

        assert!(!status_result.is_error);
        assert!(status_result.content.contains("On branch") || status_result.content.contains("No commits yet"));

        cleanup_test_repo(&temp_dir).await.unwrap();
    }
}
