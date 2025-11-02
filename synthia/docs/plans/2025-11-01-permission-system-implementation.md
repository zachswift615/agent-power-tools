# Permission System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add persistent permission system to Synthia that allows users to approve tool executions once and save decisions to avoid future prompts.

**Architecture:** Middleware layer (PermissionManager) sits between ToolRegistry and tools, checking permissions before execution. Two approval flows: file edits show diffs with per-file approval, non-file operations show command details with pattern-based approval. Permissions stored in `.synthia/settings-local.json`.

**Tech Stack:** Rust, serde_json, glob crate, tokio channels for UI communication

---

## Task 1: Create Permission Configuration Module

**Files:**
- Create: `synthia/src/permission_config.rs`
- Modify: `synthia/src/lib.rs` (add module declaration)

**Step 1: Add permission_config module to lib.rs**

In `synthia/src/lib.rs`, add after the other module declarations:

```rust
pub mod permission_config;
```

**Step 2: Create permission_config.rs with data structures**

Create `synthia/src/permission_config.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub permissions: Permissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub ask: Vec<String>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            permissions: Permissions {
                allow: Vec::new(),
                deny: Vec::new(),
                ask: Vec::new(),
            },
        }
    }
}

impl PermissionConfig {
    /// Load permission config from file, or return default if file doesn't exist
    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(config_path)?;
        let config: PermissionConfig = serde_json::from_str(&contents)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse permission config: {}, using defaults", e);
                Self::default()
            });

        Ok(config)
    }

    /// Save permission config to file atomically
    pub fn save(&self, config_path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;

        // Write atomically: temp file + rename
        let temp_path = config_path.with_extension("tmp");
        fs::write(&temp_path, json)?;
        fs::rename(temp_path, config_path)?;

        Ok(())
    }

    /// Add a permission pattern to the allow list
    pub fn add_permission(&mut self, pattern: String) -> Result<()> {
        // Avoid duplicates
        if !self.permissions.allow.contains(&pattern) {
            self.permissions.allow.push(pattern);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = PermissionConfig::default();
        assert!(config.permissions.allow.is_empty());
        assert!(config.permissions.deny.is_empty());
        assert!(config.permissions.ask.is_empty());
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = PathBuf::from("/nonexistent/path/settings.json");
        let config = PermissionConfig::load(&path).unwrap();
        assert!(config.permissions.allow.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = env::temp_dir();
        let config_path = temp_dir.join("test_permissions.json");

        // Clean up from previous runs
        let _ = fs::remove_file(&config_path);

        let mut config = PermissionConfig::default();
        config.add_permission("Bash(cargo:*)".to_string()).unwrap();
        config.add_permission("Read(//Users/test/**)".to_string()).unwrap();

        config.save(&config_path).unwrap();

        let loaded = PermissionConfig::load(&config_path).unwrap();
        assert_eq!(loaded.permissions.allow.len(), 2);
        assert!(loaded.permissions.allow.contains(&"Bash(cargo:*)".to_string()));

        // Clean up
        fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_add_permission_avoids_duplicates() {
        let mut config = PermissionConfig::default();
        config.add_permission("Bash(cargo:*)".to_string()).unwrap();
        config.add_permission("Bash(cargo:*)".to_string()).unwrap();
        assert_eq!(config.permissions.allow.len(), 1);
    }

    #[test]
    fn test_load_corrupted_json_returns_default() {
        let temp_dir = env::temp_dir();
        let config_path = temp_dir.join("corrupted_permissions.json");

        // Write invalid JSON
        fs::write(&config_path, "{ invalid json }").unwrap();

        let config = PermissionConfig::load(&config_path).unwrap();
        assert!(config.permissions.allow.is_empty());

        // Clean up
        fs::remove_file(&config_path).unwrap();
    }
}
```

**Step 3: Run tests to verify config module**

Run: `cargo test permission_config --lib`
Expected: All 5 tests pass

**Step 4: Commit**

```bash
git add synthia/src/lib.rs synthia/src/permission_config.rs
git commit -m "feat(permissions): Add permission config data structures and file I/O

- PermissionConfig with allow/deny/ask lists
- Atomic save with temp file + rename
- Load with fallback to defaults on missing/corrupted file
- Duplicate prevention in add_permission"
```

---

## Task 2: Create Permission Manager with Pattern Matching

**Files:**
- Create: `synthia/src/permission_manager.rs`
- Modify: `synthia/src/lib.rs` (add module)
- Modify: `synthia/Cargo.toml` (add glob dependency)

**Step 1: Add glob dependency**

In `synthia/Cargo.toml`, add to `[dependencies]`:

```toml
glob = "0.3"
```

**Step 2: Add permission_manager module to lib.rs**

In `synthia/src/lib.rs`:

```rust
pub mod permission_manager;
```

**Step 3: Write tests for permission manager**

Create `synthia/src/permission_manager.rs`:

```rust
use crate::permission_config::PermissionConfig;
use anyhow::Result;
use glob::Pattern;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

pub struct PermissionManager {
    config: PermissionConfig,
    config_path: PathBuf,
    project_root: PathBuf,
}

impl PermissionManager {
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let config_path = project_root.join(".synthia/settings-local.json");
        let config = PermissionConfig::load(&config_path)?;

        Ok(Self {
            config,
            config_path,
            project_root,
        })
    }

    /// Check if an operation is permitted
    pub fn check_permission(&self, tool: &str, params: &Value) -> PermissionDecision {
        let pattern = self.build_pattern(tool, params);

        // Check deny list first (highest priority)
        if self.matches_any(&pattern, &self.config.permissions.deny) {
            return PermissionDecision::Deny;
        }

        // Check allow list
        if self.matches_any(&pattern, &self.config.permissions.allow) {
            return PermissionDecision::Allow;
        }

        // Default to ask
        PermissionDecision::Ask
    }

    /// Add a permission pattern and save config
    pub fn add_permission(&mut self, pattern: String) -> Result<()> {
        self.config.add_permission(pattern)?;
        self.config.save(&self.config_path)?;
        Ok(())
    }

    /// Build a permission pattern from tool and params
    fn build_pattern(&self, tool: &str, params: &Value) -> String {
        match tool {
            "bash" => {
                if let Some(command) = params["command"].as_str() {
                    let cmd_name = command.split_whitespace().next().unwrap_or(command);
                    format!("Bash({}:*)", cmd_name)
                } else {
                    "Bash(unknown:*)".to_string()
                }
            }
            "read" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    let abs_path = self.normalize_path(file_path);
                    let dir = Path::new(&abs_path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| abs_path.clone());
                    format!("Read(//{}/)**", dir)
                } else {
                    "Read(unknown)".to_string()
                }
            }
            "write" | "edit" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    let abs_path = self.normalize_path(file_path);
                    format!("{}(//{})", tool.to_string().replace("bash", "Bash").replace("write", "Write").replace("edit", "Edit"), abs_path)
                } else {
                    format!("{}(unknown)", tool)
                }
            }
            "git" => {
                // Extract git subcommand from params
                if let Some(command) = params["command"].as_str() {
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    if parts.len() > 1 && parts[0] == "git" {
                        format!("Git({}:*)", parts[1])
                    } else {
                        "Git(unknown:*)".to_string()
                    }
                } else {
                    "Git(unknown:*)".to_string()
                }
            }
            "webfetch" => {
                if let Some(url) = params["url"].as_str() {
                    if let Ok(parsed_url) = url::Url::parse(url) {
                        if let Some(domain) = parsed_url.host_str() {
                            return format!("WebFetch(domain:{})", domain);
                        }
                    }
                }
                "WebFetch(unknown)".to_string()
            }
            other => {
                // MCP tools or other tools
                other.to_string()
            }
        }
    }

    /// Check if operation pattern matches any permission pattern
    fn matches_any(&self, operation: &str, patterns: &[String]) -> bool {
        patterns.iter().any(|p| self.matches(operation, p))
    }

    /// Check if operation matches a permission pattern
    fn matches(&self, operation: &str, pattern: &str) -> bool {
        // Exact match
        if operation == pattern {
            return true;
        }

        // Extract tool and pattern from permission string
        if let Some((perm_tool, perm_pattern)) = self.parse_permission(pattern) {
            if let Some((op_tool, op_value)) = self.parse_permission(operation) {
                if perm_tool != op_tool {
                    return false;
                }

                // Check pattern match
                if perm_pattern == "*" || perm_pattern.ends_with(":*") {
                    return true;
                }

                // Glob pattern matching for paths
                if perm_pattern.contains('*') {
                    if let Ok(glob_pattern) = Pattern::new(perm_pattern) {
                        return glob_pattern.matches(&op_value);
                    }
                }

                // Prefix match for exact patterns
                return op_value.starts_with(perm_pattern);
            }
        }

        false
    }

    /// Parse permission string into (tool, pattern)
    fn parse_permission(&self, perm: &str) -> Option<(String, String)> {
        if let Some(idx) = perm.find('(') {
            let tool = perm[..idx].to_string();
            let pattern = perm[idx + 1..]
                .trim_end_matches(')')
                .to_string();
            Some((tool, pattern))
        } else {
            // MCP tools or simple patterns
            Some((perm.to_string(), String::new()))
        }
    }

    /// Normalize path to absolute
    fn normalize_path(&self, path: &str) -> String {
        let path_buf = PathBuf::from(path);

        if path_buf.is_absolute() {
            path.to_string()
        } else {
            self.project_root
                .join(path)
                .to_string_lossy()
                .to_string()
        }
    }

    /// Generate suggested pattern for "don't ask again" based on tool and params
    pub fn suggest_pattern(&self, tool: &str, params: &Value) -> String {
        match tool {
            "bash" => {
                if let Some(command) = params["command"].as_str() {
                    let cmd_name = command.split_whitespace().next().unwrap_or(command);
                    format!("don't ask again for '{}' commands", cmd_name)
                } else {
                    "don't ask again for this command".to_string()
                }
            }
            "read" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    let abs_path = self.normalize_path(file_path);
                    let dir = Path::new(&abs_path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| abs_path);
                    format!("don't ask again for reads in {}/**", dir)
                } else {
                    "don't ask again for reads".to_string()
                }
            }
            "write" | "edit" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    format!("don't ask again for edits to {}", file_path)
                } else {
                    "don't ask again for edits".to_string()
                }
            }
            _ => "don't ask again for this operation".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn create_test_manager() -> PermissionManager {
        let temp_dir = env::temp_dir();
        let project_root = temp_dir.join("test_project");
        fs::create_dir_all(&project_root).unwrap();

        PermissionManager::new(project_root).unwrap()
    }

    #[test]
    fn test_new_manager_loads_empty_config() {
        let manager = create_test_manager();
        assert!(manager.config.permissions.allow.is_empty());
    }

    #[test]
    fn test_bash_pattern_matching() {
        let mut manager = create_test_manager();
        manager.add_permission("Bash(cargo:*)".to_string()).unwrap();

        let params = serde_json::json!({
            "command": "cargo test"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_bash_different_command_not_matched() {
        let mut manager = create_test_manager();
        manager.add_permission("Bash(cargo:*)".to_string()).unwrap();

        let params = serde_json::json!({
            "command": "npm install"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn test_read_glob_pattern_matching() {
        let mut manager = create_test_manager();
        manager.add_permission("Read(//Users/test/**)".to_string()).unwrap();

        let params = serde_json::json!({
            "file_path": "/Users/test/foo/bar.txt"
        });

        assert_eq!(
            manager.check_permission("read", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_write_exact_file_matching() {
        let mut manager = create_test_manager();
        manager.add_permission("Write(//Users/test/file.rs)".to_string()).unwrap();

        let params = serde_json::json!({
            "file_path": "/Users/test/file.rs"
        });

        assert_eq!(
            manager.check_permission("write", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_deny_overrides_allow() {
        let mut manager = create_test_manager();
        manager.config.permissions.allow.push("Bash(cargo:*)".to_string());
        manager.config.permissions.deny.push("Bash(cargo:*)".to_string());

        let params = serde_json::json!({
            "command": "cargo test"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Deny
        );
    }

    #[test]
    fn test_default_is_ask() {
        let manager = create_test_manager();

        let params = serde_json::json!({
            "command": "cargo test"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn test_mcp_tool_exact_match() {
        let mut manager = create_test_manager();
        manager.add_permission("mcp__powertools__index_project".to_string()).unwrap();

        let params = serde_json::json!({});

        assert_eq!(
            manager.check_permission("mcp__powertools__index_project", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_suggest_pattern_bash() {
        let manager = create_test_manager();
        let params = serde_json::json!({
            "command": "cargo test --all"
        });

        let suggestion = manager.suggest_pattern("bash", &params);
        assert!(suggestion.contains("cargo"));
    }

    #[test]
    fn test_add_permission_saves_config() {
        let temp_dir = env::temp_dir();
        let project_root = temp_dir.join("test_add_permission");
        fs::create_dir_all(&project_root).unwrap();

        let mut manager = PermissionManager::new(project_root.clone()).unwrap();
        manager.add_permission("Bash(cargo:*)".to_string()).unwrap();

        // Load fresh manager and verify persistence
        let manager2 = PermissionManager::new(project_root.clone()).unwrap();
        assert_eq!(manager2.config.permissions.allow.len(), 1);

        // Clean up
        fs::remove_dir_all(&project_root).unwrap();
    }
}
```

**Step 4: Add url dependency for domain parsing**

In `synthia/Cargo.toml`, add to `[dependencies]`:

```toml
url = "2.5"
```

**Step 5: Run tests to verify permission manager**

Run: `cargo test permission_manager --lib`
Expected: All 11 tests pass

**Step 6: Commit**

```bash
git add synthia/Cargo.toml synthia/src/lib.rs synthia/src/permission_manager.rs
git commit -m "feat(permissions): Add permission manager with pattern matching

- PermissionManager checks allow/deny/ask patterns
- Pattern construction for bash, read, write, edit, git, webfetch, MCP
- Glob pattern matching for file paths
- Command prefix matching for bash/git
- Deny overrides allow for security
- Suggested patterns for UI display"
```

---

## Task 3: Integrate PermissionManager into ToolRegistry

**Files:**
- Modify: `synthia/src/tools/registry.rs`
- Modify: `synthia/src/main.rs`

**Step 1: Add PermissionManager field to ToolRegistry**

In `synthia/src/tools/registry.rs`, import at top:

```rust
use crate::permission_manager::{PermissionManager, PermissionDecision};
use std::sync::{Arc, Mutex};
```

Add field to `ToolRegistry` struct (around line 20):

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    ui_sender: Option<UnboundedSender<UIUpdate>>,
    permission_manager: Arc<Mutex<PermissionManager>>,  // NEW
}
```

**Step 2: Update ToolRegistry::new() to accept PermissionManager**

Modify the `new()` method (around line 24):

```rust
impl ToolRegistry {
    pub fn new(permission_manager: Arc<Mutex<PermissionManager>>) -> Self {
        Self {
            tools: HashMap::new(),
            ui_sender: None,
            permission_manager,
        }
    }
```

**Step 3: Add permission check at start of execute() method**

In the `execute()` method (around line 46), add permission check before the existing edit/write approval logic:

```rust
pub async fn execute(&self, name: &str, params: Value) -> Result<ToolResult> {
    // 1. Check permission first
    let decision = self.permission_manager.lock().unwrap()
        .check_permission(name, &params);

    match decision {
        PermissionDecision::Deny => {
            return Ok(ToolResult {
                content: "Operation denied by permissions".to_string(),
                is_error: true,
            });
        }
        PermissionDecision::Allow => {
            // For edit/write: show informational diff if in allow list
            // For others: execute directly
        }
        PermissionDecision::Ask => {
            // Proceed to existing approval flow
        }
    }

    // Existing code continues here...
```

**Step 4: Update main.rs to create PermissionManager**

In `synthia/src/main.rs`, import at top:

```rust
use synthia::permission_manager::PermissionManager;
use std::sync::{Arc, Mutex};
```

After loading project context (around line 65), create permission manager:

```rust
let permission_manager = Arc::new(Mutex::new(
    PermissionManager::new(project_context.project_root.clone())?
));
```

Update ToolRegistry creation (around line 72):

```rust
let mut tool_registry = ToolRegistry::new(permission_manager);
```

**Step 5: Test compilation**

Run: `cargo build --lib`
Expected: Builds successfully

**Step 6: Commit**

```bash
git add synthia/src/tools/registry.rs synthia/src/main.rs
git commit -m "feat(permissions): Integrate PermissionManager into ToolRegistry

- Add PermissionManager field to ToolRegistry
- Check permissions before tool execution
- Deny returns error, Ask continues to approval flow
- Allow flow (informational diff) to be implemented next"
```

---

## Task 4: Add UI Updates for Permission Prompts

**Files:**
- Modify: `synthia/src/ui/mod.rs` (add UIUpdate variants)
- Modify: `synthia/src/tools/registry.rs` (send new UI updates)

**Step 1: Add new UIUpdate variants**

In `synthia/src/ui/mod.rs`, add to the `UIUpdate` enum (around line 20):

```rust
pub enum UIUpdate {
    // ... existing variants ...

    PermissionPrompt {
        tool_name: String,
        operation_details: String,
        suggested_pattern: String,
        response_tx: tokio::sync::oneshot::Sender<PermissionResponse>,
    },

    InformationalDiff {
        tool_name: String,
        file_path: String,
        diff: String,
    },
}
```

Add PermissionResponse enum:

```rust
#[derive(Debug)]
pub enum PermissionResponse {
    Yes,
    YesAndDontAsk(String),  // Contains pattern to add
    No,
}
```

**Step 2: Implement permission prompt sending in registry**

In `synthia/src/tools/registry.rs`, in the `execute()` method where we handle `PermissionDecision::Ask`, add permission prompt logic for non-edit/write tools:

```rust
PermissionDecision::Ask => {
    // Check if this is edit/write (existing approval flow)
    if name == "edit" || name == "write" {
        // Use existing edit approval flow
        // (keep existing code)
    } else {
        // New permission prompt for other tools
        if let Some(ui_sender) = &self.ui_sender {
            let (response_tx, response_rx) = tokio::sync::oneshot::channel();

            let operation_details = match name {
                "bash" => {
                    format!("Command: {}\nDirectory: {}",
                        params["command"].as_str().unwrap_or("unknown"),
                        std::env::current_dir()
                            .unwrap_or_default()
                            .to_string_lossy())
                }
                "read" => {
                    format!("Read file: {}",
                        params["file_path"].as_str().unwrap_or("unknown"))
                }
                "git" => {
                    format!("Git command: {}",
                        params["command"].as_str().unwrap_or("unknown"))
                }
                _ => format!("Operation: {} with params", name),
            };

            let suggested_pattern = self.permission_manager.lock().unwrap()
                .suggest_pattern(name, &params);

            ui_sender.send(UIUpdate::PermissionPrompt {
                tool_name: name.to_string(),
                operation_details,
                suggested_pattern,
                response_tx,
            })?;

            match response_rx.await? {
                PermissionResponse::Yes => {
                    // Execute once
                }
                PermissionResponse::YesAndDontAsk(pattern) => {
                    // Add to permissions and execute
                    self.permission_manager.lock().unwrap()
                        .add_permission(pattern)?;
                    // Execute
                }
                PermissionResponse::No => {
                    return Ok(ToolResult {
                        content: "Operation cancelled by user".to_string(),
                        is_error: false,
                    });
                }
            }
        }
    }
}
```

**Step 3: Implement informational diff for auto-approved edits**

In the `PermissionDecision::Allow` branch for edit/write tools:

```rust
PermissionDecision::Allow => {
    if name == "edit" || name == "write" {
        // Compute diff for informational display
        let diff = if name == "edit" {
            self.compute_edit_diff(&params)?
        } else {
            self.compute_write_diff(&params)?
        };

        if let Some(ui_sender) = &self.ui_sender {
            ui_sender.send(UIUpdate::InformationalDiff {
                tool_name: name.to_string(),
                file_path: params["file_path"].as_str().unwrap_or("unknown").to_string(),
                diff,
            })?;
        }
    }
    // Continue to execute directly
}
```

**Step 4: Test compilation**

Run: `cargo build --lib`
Expected: Builds successfully

**Step 5: Commit**

```bash
git add synthia/src/ui/mod.rs synthia/src/tools/registry.rs
git commit -m "feat(permissions): Add UI updates for permission prompts

- Add PermissionPrompt UIUpdate variant with response channel
- Add InformationalDiff for auto-approved edits
- Send permission prompts for non-edit/write tools
- Send informational diffs for edit/write in allow list"
```

---

## Task 5: Implement UI Rendering for Permission Prompts

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Add permission approval state to App**

In `synthia/src/ui/app.rs`, add field to `App` struct (around line 50):

```rust
pub struct App {
    // ... existing fields ...
    pending_permission_approval: Option<PermissionApprovalState>,
}
```

Add state struct before `impl App`:

```rust
struct PermissionApprovalState {
    tool_name: String,
    operation_details: String,
    suggested_pattern: String,
    response_tx: tokio::sync::oneshot::Sender<PermissionResponse>,
    selected_option: usize,  // 0, 1, or 2
}
```

Initialize in `App::new()` (around line 100):

```rust
pending_permission_approval: None,
```

**Step 2: Handle PermissionPrompt UIUpdate**

In the `run()` method's UIUpdate match (around line 700), add:

```rust
UIUpdate::PermissionPrompt {
    tool_name,
    operation_details,
    suggested_pattern,
    response_tx,
} => {
    self.pending_permission_approval = Some(PermissionApprovalState {
        tool_name,
        operation_details,
        suggested_pattern,
        response_tx,
        selected_option: 0,  // Default to first option
    });
}
```

**Step 3: Handle InformationalDiff UIUpdate**

Add case in same match:

```rust
UIUpdate::InformationalDiff {
    tool_name,
    file_path,
    diff,
} => {
    // Add to message history as informational text
    self.message_history.push(ConversationMessage {
        role: "assistant".to_string(),
        content: format!("Auto-approved {} for {}:\n\n{}", tool_name, file_path, diff),
    });
}
```

**Step 4: Render permission prompt**

Add rendering function before the impl block:

```rust
fn render_permission_prompt(
    &self,
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &PermissionApprovalState,
) {
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::style::{Color, Style};

    let prompt_text = format!(
        "Tool: {}\n{}\n\nDo you want to proceed?\n  {} 1. Yes\n  {} 2. Yes, and {}\n  {} 3. No (esc)",
        state.tool_name,
        state.operation_details,
        if state.selected_option == 0 { "→" } else { " " },
        if state.selected_option == 1 { "→" } else { " " },
        state.suggested_pattern,
        if state.selected_option == 2 { "→" } else { " " },
    );

    let block = Block::default()
        .title("Permission Required")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(prompt_text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
```

Call this in the main render method when permission approval is pending.

**Step 5: Handle keyboard input for permission prompt**

In `handle_input()` method (around line 1250), add at the beginning:

```rust
// Check if we have pending permission approval
if let Some(mut approval_state) = self.pending_permission_approval.take() {
    match event {
        Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            approval_state.selected_option = approval_state.selected_option.saturating_sub(1);
            self.pending_permission_approval = Some(approval_state);
            return Ok(());
        }
        Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            approval_state.selected_option = (approval_state.selected_option + 1).min(2);
            self.pending_permission_approval = Some(approval_state);
            return Ok(());
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            let _ = approval_state.response_tx.send(PermissionResponse::Yes);
            return Ok(());
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('2'),
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            let pattern = self.build_permission_pattern(&approval_state);
            let _ = approval_state.response_tx.send(PermissionResponse::YesAndDontAsk(pattern));
            return Ok(());
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('3') | KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            let _ = approval_state.response_tx.send(PermissionResponse::No);
            return Ok(());
        }
        Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            let response = match approval_state.selected_option {
                0 => PermissionResponse::Yes,
                1 => {
                    let pattern = self.build_permission_pattern(&approval_state);
                    PermissionResponse::YesAndDontAsk(pattern)
                }
                2 => PermissionResponse::No,
                _ => PermissionResponse::No,
            };
            let _ = approval_state.response_tx.send(response);
            return Ok(());
        }
        _ => {
            // Put state back and continue processing
            self.pending_permission_approval = Some(approval_state);
        }
    }
}
```

**Step 6: Add helper to build permission pattern**

Add method to App:

```rust
fn build_permission_pattern(&self, state: &PermissionApprovalState) -> String {
    // This will be constructed from the actual operation params
    // For now, return a placeholder that registry will compute
    state.suggested_pattern.clone()
}
```

**Step 7: Test compilation**

Run: `cargo build`
Expected: Builds successfully (may have warnings)

**Step 8: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "feat(permissions): Implement UI rendering for permission prompts

- Add PermissionApprovalState to App
- Render permission prompt with three options
- Handle keyboard input (arrows, 1-3, enter, esc)
- Display informational diffs in message history
- Arrow keys and number keys for option selection"
```

---

## Task 6: Enhance Edit/Write Approval with "Don't Ask for This File"

**Files:**
- Modify: `synthia/src/ui/app.rs`
- Modify: `synthia/src/tools/registry.rs`

**Step 1: Update EditApprovalState with new option**

In `synthia/src/ui/app.rs`, modify the `EditApprovalState` struct to include file path:

```rust
struct EditApprovalState {
    file_path: String,  // NEW: store file path for pattern building
    diff: String,
    response_tx: tokio::sync::oneshot::Sender<ApprovalResponse>,
}
```

**Step 2: Add new ApprovalResponse variant**

Modify `ApprovalResponse` enum:

```rust
pub enum ApprovalResponse {
    Approve,
    ApproveDontAsk(String),  // NEW: contains permission pattern
    Reject,
}
```

**Step 3: Update edit approval prompt rendering**

In the `render_edit_approval_prompt()` function, change the prompt text:

```rust
let prompt = Span::styled(
    "[A]ccept  [D]on't ask for this file  [R]eject",
    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
);
```

**Step 4: Handle 'D' key in edit approval input**

In the edit approval input handling (around line 1240), add case for 'D' key:

```rust
if let Some(approval_state) = self.pending_edit_approval.take() {
    match event {
        Event::Key(KeyEvent {
            code: KeyCode::Char('a') | KeyCode::Char('A'),
            ..
        }) => {
            let _ = approval_state.response_tx.send(ApprovalResponse::Approve);
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('d') | KeyCode::Char('D'),
            ..
        }) => {
            // Build pattern for this specific file
            let pattern = format!("Edit(//{})", approval_state.file_path);
            let _ = approval_state.response_tx.send(ApprovalResponse::ApproveDontAsk(pattern));
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Esc,
            ..
        }) => {
            let _ = approval_state.response_tx.send(ApprovalResponse::Reject);
        }
        _ => {
            self.pending_edit_approval = Some(approval_state);
        }
    }
}
```

**Step 5: Update registry to handle ApproveDontAsk**

In `synthia/src/tools/registry.rs`, update the edit approval response handling:

```rust
match response_rx.await? {
    ApprovalResponse::Approve => {
        // Execute
        tool.execute(params.clone()).await
    }
    ApprovalResponse::ApproveDontAsk(pattern) => {
        // Add permission and execute
        self.permission_manager.lock().unwrap()
            .add_permission(pattern)?;
        tool.execute(params.clone()).await
    }
    ApprovalResponse::Reject => {
        Ok(ToolResult {
            content: "Edit cancelled by user".to_string(),
            is_error: false,
        })
    }
}
```

**Step 6: Pass file_path in EditPreview UIUpdate**

Update where EditPreview is sent to include file_path:

```rust
ui_sender.send(UIUpdate::EditPreview {
    file_path: params["file_path"].as_str().unwrap_or("unknown").to_string(),  // NEW
    diff: diff_output,
    response_tx,
})?;
```

**Step 7: Test compilation**

Run: `cargo build`
Expected: Builds successfully

**Step 8: Commit**

```bash
git add synthia/src/ui/app.rs synthia/src/tools/registry.rs
git commit -m "feat(permissions): Add 'don't ask for this file' to edit approval

- Add ApproveDontAsk variant to ApprovalResponse
- Handle 'D' key in edit approval prompt
- Build per-file permission pattern (Edit(//path))
- Save permission when user selects don't ask
- Update EditApprovalState to include file_path"
```

---

## Task 7: Handle Permission Patterns from Registry

**Files:**
- Modify: `synthia/src/tools/registry.rs`

**Step 1: Fix permission pattern building in registry**

The UI currently uses suggested pattern text, but registry needs to build the actual permission string. Update the permission prompt handling to build the correct pattern:

```rust
PermissionResponse::YesAndDontAsk(_) => {
    // Build actual permission pattern from tool and params
    let pattern = self.permission_manager.lock().unwrap()
        .build_pattern(name, &params);

    self.permission_manager.lock().unwrap()
        .add_permission(pattern)?;

    // Execute
}
```

**Step 2: Make build_pattern public in PermissionManager**

In `synthia/src/permission_manager.rs`, change visibility:

```rust
pub fn build_pattern(&self, tool: &str, params: &Value) -> String {
    // existing implementation
}
```

**Step 3: Test compilation**

Run: `cargo build`
Expected: Builds successfully

**Step 4: Commit**

```bash
git add synthia/src/permission_manager.rs synthia/src/tools/registry.rs
git commit -m "fix(permissions): Use actual permission patterns from manager

- Make build_pattern public in PermissionManager
- Registry builds correct pattern instead of using suggested text
- Ensures saved patterns match checking logic"
```

---

## Task 8: Integration Testing and Bug Fixes

**Files:**
- Create: `synthia/tests/permission_integration_test.rs`

**Step 1: Create integration test**

Create `synthia/tests/permission_integration_test.rs`:

```rust
use std::env;
use std::fs;
use std::path::PathBuf;
use synthia::permission_manager::{PermissionManager, PermissionDecision};

#[test]
fn test_permission_workflow() {
    let temp_dir = env::temp_dir();
    let project_root = temp_dir.join("permission_integration_test");
    fs::create_dir_all(&project_root).unwrap();

    // Create manager
    let mut manager = PermissionManager::new(project_root.clone()).unwrap();

    // Initially should ask
    let params = serde_json::json!({
        "command": "cargo test"
    });
    assert_eq!(
        manager.check_permission("bash", &params),
        PermissionDecision::Ask
    );

    // Add permission
    manager.add_permission("Bash(cargo:*)".to_string()).unwrap();

    // Now should allow
    assert_eq!(
        manager.check_permission("bash", &params),
        PermissionDecision::Allow
    );

    // Verify persistence
    let manager2 = PermissionManager::new(project_root.clone()).unwrap();
    assert_eq!(
        manager2.check_permission("bash", &params),
        PermissionDecision::Allow
    );

    // Clean up
    fs::remove_dir_all(&project_root).unwrap();
}
```

**Step 2: Run integration test**

Run: `cargo test permission_integration_test`
Expected: Test passes

**Step 3: Manual testing checklist**

Test the following scenarios manually:

1. **Bash command approval**:
   - Run Synthia
   - Trigger bash command (e.g., ask it to run "cargo --version")
   - Verify permission prompt appears
   - Select option 2 (don't ask again)
   - Verify command runs
   - Trigger same command again
   - Verify it runs without prompt

2. **Edit approval with don't ask**:
   - Ask Synthia to edit a file
   - Verify diff preview appears
   - Press 'D' for don't ask
   - Verify edit executes
   - Ask to edit same file again
   - Verify diff shows but auto-executes

3. **Read file outside project**:
   - Ask Synthia to read a file outside project
   - Verify permission prompt
   - Approve with pattern
   - Verify subsequent reads in that directory don't prompt

4. **Config file persistence**:
   - Check `.synthia/settings-local.json` exists
   - Verify it contains added permissions
   - Restart Synthia
   - Verify permissions still apply

**Step 4: Document any bugs found and fix them**

Create follow-up commits for any issues discovered during testing.

**Step 5: Commit integration test**

```bash
git add synthia/tests/permission_integration_test.rs
git commit -m "test(permissions): Add integration test for permission workflow

- Test ask -> add permission -> allow flow
- Verify persistence across manager instances
- Validate full permission lifecycle"
```

---

## Task 9: Documentation and Final Polish

**Files:**
- Modify: `synthia/README.md` (if exists)
- Create: `synthia/docs/PERMISSIONS.md`
- Modify: `synthia/src/permission_config.rs` (add doc comments)
- Modify: `synthia/src/permission_manager.rs` (add doc comments)

**Step 1: Create permissions documentation**

Create `synthia/docs/PERMISSIONS.md`:

```markdown
# Permission System

Synthia includes a persistent permission system that allows you to approve tool executions once and save your decision to avoid future prompts.

## How It Works

When Synthia attempts to execute a tool (bash command, file read/write, git operation, etc.), it checks the permission configuration. If the operation isn't explicitly allowed, you'll be prompted to approve it.

## Permission Prompts

### File Edits (Edit/Write)

Shows a diff preview with three options:
- **[A]ccept** - Execute this edit once
- **[D]on't ask for this file** - Auto-approve all future edits to this specific file (still shows diffs)
- **[R]eject** - Cancel the operation

### Command Execution (Bash/Git/Read/etc)

Shows operation details with three options:
- **1. Yes** - Execute once
- **2. Yes, and don't ask again** - Auto-approve all similar operations (e.g., all cargo commands)
- **3. No** - Cancel the operation

Use arrow keys or numbers to select, Enter to confirm, Esc to cancel.

## Permission Patterns

Permissions are stored in `.synthia/settings-local.json` with patterns like:

- `Bash(cargo:*)` - All cargo commands
- `Read(//Users/username/**)` - All reads in directory tree
- `Write(//path/to/file.rs)` - Specific file writes
- `Git(commit:*)` - All git commit operations
- `WebFetch(domain:github.com)` - Fetches from github.com
- `mcp__powertools__index_project` - Specific MCP tool

## Configuration File

Location: `.synthia/settings-local.json` in your project root

Example:
```json
{
  "permissions": {
    "allow": [
      "Bash(cargo:*)",
      "Read(//Users/username/projects/**)",
      "Write(//Users/username/projects/myproject/src/main.rs)"
    ],
    "deny": [],
    "ask": []
  }
}
```

**Note:** This file is typically git-ignored (user-specific permissions). You can commit it to share team permissions if desired.

## Glob Patterns

- `**` - Match any number of directories
- `*` - Match any characters in a path segment
- Paths with `//` prefix are absolute
- Paths with `/` prefix are relative to project root

## Security

- Deny list takes precedence over allow list
- All paths are normalized to prevent traversal attacks
- Corrupted config files fall back to safe defaults (ask for everything)
- Atomic writes prevent config corruption
```

**Step 2: Add comprehensive doc comments to permission_config.rs**

Add module-level documentation at the top of `synthia/src/permission_config.rs`:

```rust
//! Permission configuration data structures and file I/O.
//!
//! This module provides the data structures for storing permission configurations
//! and handles loading/saving them from disk with atomic writes for safety.
```

Add doc comments to public methods.

**Step 3: Add comprehensive doc comments to permission_manager.rs**

Add module-level documentation:

```rust
//! Permission manager with pattern matching logic.
//!
//! This module provides the core permission checking logic, including:
//! - Building permission patterns from tool invocations
//! - Matching operations against allow/deny lists
//! - Glob pattern matching for file paths
//! - Command prefix matching for bash/git operations
```

**Step 4: Update main README if it exists**

If `synthia/README.md` exists, add section:

```markdown
## Permissions

Synthia includes a permission system to control tool execution. See [docs/PERMISSIONS.md](docs/PERMISSIONS.md) for details.
```

**Step 5: Commit documentation**

```bash
git add synthia/docs/PERMISSIONS.md synthia/src/permission_config.rs synthia/src/permission_manager.rs
git commit -m "docs(permissions): Add comprehensive permission system documentation

- User guide in docs/PERMISSIONS.md
- Module-level documentation in source files
- Examples of permission patterns and usage
- Security considerations and glob pattern syntax"
```

---

## Task 10: Final Testing and Cleanup

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All permission-related tests pass (pre-existing failures acceptable)

**Step 2: Run clippy for code quality**

Run: `cargo clippy --all-targets`
Fix any warnings that appear in permission-related code.

**Step 3: Check formatting**

Run: `cargo fmt --check`
If needed: `cargo fmt`

**Step 4: Build release binary**

Run: `cargo build --release`
Expected: Builds successfully

**Step 5: Manual smoke test**

1. Run Synthia from release build
2. Trigger bash command → approve with pattern → verify persistence
3. Trigger edit → approve for file → verify diff shows on next edit
4. Check `.synthia/settings-local.json` has correct entries
5. Restart Synthia → verify permissions still work

**Step 6: Commit any final fixes**

```bash
git add .
git commit -m "chore(permissions): Final cleanup and linting fixes

- Fix clippy warnings
- Format code with rustfmt
- Verify all tests pass"
```

**Step 7: Create summary commit**

```bash
git log --oneline feature/permission-system --not main
```

Review all commits and ensure they tell a clear story.

---

## Completion Checklist

- [ ] PermissionConfig module with tests
- [ ] PermissionManager with pattern matching
- [ ] ToolRegistry integration
- [ ] UI rendering for permission prompts
- [ ] Edit/write approval enhancement
- [ ] Permission pattern building
- [ ] Integration testing
- [ ] Documentation
- [ ] Code quality (clippy, fmt)
- [ ] Manual testing passed

## Next Steps

After completing this plan:

1. **Merge to main**: Create PR from `feature/permission-system` branch
2. **User testing**: Get feedback on permission UX
3. **Iteration**: Based on feedback, consider:
   - Permission management UI (list/edit/remove patterns)
   - Risk level indicators for operations
   - Time-based permissions (expire after session)
   - Team-level permission profiles

## References

- Design document: `docs/plans/2025-11-01-permission-system-design.md`
- Existing approval flow: `synthia/src/tools/registry.rs:78-213`
- Example settings: `/Users/zachswift/projects/substansive_synth/.claude/settings.local.json`
