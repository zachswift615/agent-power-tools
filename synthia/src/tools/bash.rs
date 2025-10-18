use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub struct BashTool {
    timeout_seconds: u64,
}

impl BashTool {
    pub fn new(timeout_seconds: u64) -> Self {
        Self { timeout_seconds }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a bash command and return stdout/stderr. \
        For long-running processes (servers, watchers), append '&' to run in background. \
        Default timeout: 5 minutes (configurable in synthia.toml)."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        // Log the received params for debugging
        tracing::debug!("BashTool received params: {:?}", params);

        let command = params["command"]
            .as_str()
            .ok_or_else(|| {
                tracing::error!("BashTool: Missing or invalid 'command' parameter. Received params: {:?}", params);
                anyhow::anyhow!("Missing 'command' parameter. Received params: {}", params)
            })?;

        let result = timeout(
            Duration::from_secs(self.timeout_seconds),
            Command::new("bash").arg("-c").arg(command).output(),
        )
        .await;

        // Handle timeout with a helpful error message
        let result = match result {
            Ok(r) => r?,
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Command timed out after {} seconds.\n\n\
                    For long-running processes (servers, watchers, etc.), consider:\n\
                    1. Running the command in the background by appending '&' (e.g., 'python app.py &')\n\
                    2. Using 'nohup' for processes that should continue after disconnect\n\
                    3. Increasing the timeout in synthia.toml under [timeouts] bash_timeout\n\n\
                    Command: {}",
                    self.timeout_seconds,
                    command
                ))
            }
        };

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        let content = if !stderr.is_empty() {
            format!("stdout:\n{}\nstderr:\n{}", stdout, stderr)
        } else {
            stdout.to_string()
        };

        Ok(ToolResult {
            content,
            is_error: !result.status.success(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "command": "echo 'hello world'"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("hello world"));
    }

    #[tokio::test]
    async fn test_bash_error() {
        let tool = BashTool::new(5);
        let result = tool
            .execute(serde_json::json!({
                "command": "exit 1"
            }))
            .await
            .unwrap();

        assert!(result.is_error);
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let tool = BashTool::new(1);
        let result = tool
            .execute(serde_json::json!({
                "command": "sleep 10"
            }))
            .await;

        assert!(result.is_err());
    }
}
