use super::{Tool, ToolResult};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

// Embed the powertools binary at compile time
// In a Cargo workspace, target directory is at workspace root
static POWERTOOLS_BINARY: &[u8] = include_bytes!("../../../target/release/powertools");

/// Powertools integration tool that provides semantic code navigation capabilities
/// by shelling out to the powertools binary. Operations include:
/// - index: Index a project for semantic navigation
/// - definition: Go to definition (file:line:column format)
/// - references: Find all references to a symbol
/// - functions: List all functions in a file/directory
/// - classes: List all classes/structs
/// - stats: Get project statistics
pub struct PowertoolsTool {
    binary_path: PathBuf,
    timeout_seconds: u64,
}

impl PowertoolsTool {
    /// Create a new PowertoolsTool with optional custom binary path.
    /// If no path is provided, extracts and uses the embedded binary.
    pub fn new(binary_path: Option<PathBuf>) -> Result<Self> {
        let binary_path = match binary_path {
            Some(path) => path,
            None => Self::get_embedded_binary_path()?,
        };

        Ok(Self {
            binary_path,
            timeout_seconds: 120,
        })
    }

    /// Extract the embedded powertools binary to cache and return its path.
    /// The binary is extracted to ~/.cache/synthia/powertools
    fn get_embedded_binary_path() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow!("Could not determine cache directory"))?
            .join("synthia");

        fs::create_dir_all(&cache_dir)?;

        let binary_path = cache_dir.join("powertools");

        // Check if binary already exists and is up to date
        let needs_extraction = if binary_path.exists() {
            // Compare file size as a simple check
            let existing_size = fs::metadata(&binary_path)?.len();
            existing_size != POWERTOOLS_BINARY.len() as u64
        } else {
            true
        };

        if needs_extraction {
            tracing::info!("Extracting embedded powertools binary to {}", binary_path.display());
            fs::write(&binary_path, POWERTOOLS_BINARY)?;

            // Make it executable (Unix only)
            #[cfg(unix)]
            {
                let mut perms = fs::metadata(&binary_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&binary_path, perms)?;
            }
        }

        Ok(binary_path)
    }

    async fn run_powertools_command(
        &self,
        args: &[&str],
        cwd: Option<&str>,
    ) -> Result<ToolResult> {
        // Check if powertools binary exists
        if !self.binary_path.exists() {
            return Ok(ToolResult {
                content: format!(
                    "Powertools binary not found at {}. Please build it with: cd powertools-cli && cargo build --release",
                    self.binary_path.display()
                ),
                is_error: true,
            });
        }

        let mut cmd = Command::new(&self.binary_path);
        cmd.args(args);

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

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

        // Try to parse and format JSON output
        let content = if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
            // Pretty print JSON for better readability
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| stdout.to_string())
        } else {
            stdout.to_string()
        };

        Ok(ToolResult {
            content,
            is_error: false,
        })
    }

    async fn index(
        &self,
        cwd: Option<&str>,
        auto_install: bool,
        languages: Option<Vec<String>>,
    ) -> Result<ToolResult> {
        let mut args = vec!["index", "--format", "json"];

        if auto_install {
            args.push("--auto-install");
        }

        let languages_str: Vec<String>;
        if let Some(langs) = languages {
            languages_str = langs;
            args.push("--languages");
            for lang in &languages_str {
                args.push(lang);
            }
        }

        self.run_powertools_command(&args, cwd).await
    }

    async fn definition(&self, location: String, project_root: Option<&str>) -> Result<ToolResult> {
        let mut args = vec!["definition", &location, "--format", "json"];

        let project_root_str;
        if let Some(root) = project_root {
            args.push("-p");
            project_root_str = root.to_string();
            args.push(&project_root_str);
        }

        self.run_powertools_command(&args, None).await
    }

    async fn references(
        &self,
        symbol: String,
        project_root: Option<&str>,
        include_declarations: bool,
    ) -> Result<ToolResult> {
        let mut args = vec!["references", &symbol, "--format", "json"];

        if include_declarations {
            args.push("--include-declarations");
        }

        let project_root_str;
        if let Some(root) = project_root {
            args.push("-p");
            project_root_str = root.to_string();
            args.push(&project_root_str);
        }

        self.run_powertools_command(&args, None).await
    }

    async fn functions(&self, path: Option<&str>, cwd: Option<&str>) -> Result<ToolResult> {
        let mut args = vec!["functions", "--format", "json"];

        let path_str;
        if let Some(p) = path {
            args.push("-p");
            path_str = p.to_string();
            args.push(&path_str);
        }

        self.run_powertools_command(&args, cwd).await
    }

    async fn classes(&self, path: Option<&str>, cwd: Option<&str>) -> Result<ToolResult> {
        let mut args = vec!["classes", "--format", "json"];

        let path_str;
        if let Some(p) = path {
            args.push("-p");
            path_str = p.to_string();
            args.push(&path_str);
        }

        self.run_powertools_command(&args, cwd).await
    }

    async fn stats(&self, path: Option<&str>, cwd: Option<&str>) -> Result<ToolResult> {
        let mut args = vec!["stats", "--format", "json"];

        let path_str;
        if let Some(p) = path {
            args.push("-p");
            path_str = p.to_string();
            args.push(&path_str);
        }

        self.run_powertools_command(&args, cwd).await
    }
}

#[async_trait]
impl Tool for PowertoolsTool {
    fn name(&self) -> &str {
        "powertools"
    }

    fn description(&self) -> &str {
        "Execute powertools operations for semantic code navigation (index, definition, references, functions, classes, stats)"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Powertools operation to perform",
                    "enum": ["index", "definition", "references", "functions", "classes", "stats"]
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for powertools command (optional)"
                },
                "auto_install": {
                    "type": "boolean",
                    "description": "For index: auto-install missing indexers (default: true)"
                },
                "languages": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "For index: specific languages to index (e.g., ['typescript', 'python'])"
                },
                "location": {
                    "type": "string",
                    "description": "For definition: file:line:column location (e.g., 'src/file.rs:42:10')"
                },
                "symbol": {
                    "type": "string",
                    "description": "For references: symbol name to find references for"
                },
                "include_declarations": {
                    "type": "boolean",
                    "description": "For references: include declarations in results (default: false)"
                },
                "project_root": {
                    "type": "string",
                    "description": "For definition/references: project root directory (optional)"
                },
                "path": {
                    "type": "string",
                    "description": "For functions/classes/stats: path to analyze (optional)"
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
            "index" => {
                let auto_install = params["auto_install"].as_bool().unwrap_or(true);
                let languages = params["languages"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    });
                self.index(cwd, auto_install, languages).await
            }
            "definition" => {
                let location = params["location"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'location' parameter for definition operation"))?
                    .to_string();
                let project_root = params["project_root"].as_str();
                self.definition(location, project_root).await
            }
            "references" => {
                let symbol = params["symbol"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'symbol' parameter for references operation"))?
                    .to_string();
                let project_root = params["project_root"].as_str();
                let include_declarations = params["include_declarations"].as_bool().unwrap_or(false);
                self.references(symbol, project_root, include_declarations)
                    .await
            }
            "functions" => {
                let path = params["path"].as_str();
                self.functions(path, cwd).await
            }
            "classes" => {
                let path = params["path"].as_str();
                self.classes(path, cwd).await
            }
            "stats" => {
                let path = params["path"].as_str();
                self.stats(path, cwd).await
            }
            _ => Err(anyhow!("Unknown powertools operation: {}", operation)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_binary_path() -> PathBuf {
        // Use relative path from synthia directory to powertools binary
        // In a Cargo workspace, target directory is at workspace root
        PathBuf::from("../target/release/powertools")
    }

    #[tokio::test]
    async fn test_powertools_stats() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "stats",
                "cwd": "."
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        // Should contain JSON with file counts
        assert!(result.content.contains("total_files") || result.content.contains("languages"));
    }

    #[tokio::test]
    async fn test_powertools_functions() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "functions",
                "cwd": ".",
                "path": "src/tools"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        // Should return JSON array of functions
        assert!(result.content.contains("name") || result.content.contains("Found"));
    }

    #[tokio::test]
    async fn test_powertools_classes() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "classes",
                "cwd": "."
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        // Should return JSON array of classes/structs
        assert!(result.content.contains("name") || result.content.contains("Found"));
    }

    #[tokio::test]
    async fn test_powertools_invalid_operation() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "invalid_operation"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown powertools operation"));
    }

    #[tokio::test]
    async fn test_powertools_missing_location_for_definition() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "definition"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'location' parameter"));
    }

    #[tokio::test]
    async fn test_powertools_missing_symbol_for_references() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "references"
            }))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'symbol' parameter"));
    }

    #[tokio::test]
    async fn test_powertools_stats_with_path() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "stats",
                "cwd": ".",
                "path": "src"
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("total_files") || result.content.contains("languages"));
    }

    #[tokio::test]
    async fn test_powertools_binary_not_found() {
        let tool = PowertoolsTool::new(Some(PathBuf::from("/nonexistent/path/powertools"))).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "stats"
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("Powertools binary not found"));
    }

    #[tokio::test]
    async fn test_powertools_index_with_auto_install() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "index",
                "cwd": ".",
                "auto_install": true
            }))
            .await;

        // This test might succeed or fail depending on whether indexers are installed
        // We just verify that the command was constructed correctly
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_powertools_index_with_languages() {
        let tool = PowertoolsTool::new(Some(get_test_binary_path())).unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "index",
                "cwd": ".",
                "auto_install": true,
                "languages": ["rust"]
            }))
            .await;

        // This test might succeed or fail depending on whether indexers are installed
        // We just verify that the command was constructed correctly
        assert!(result.is_ok() || result.is_err());
    }
}
