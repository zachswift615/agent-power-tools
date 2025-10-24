# Context Management System - Phase 2 Implementation Plan

> **For Claude:** Use `${SUPERPOWERS_SKILLS_ROOT}/skills/collaboration/subagent-driven-development/SKILL.md` to implement this plan task-by-task.

**Goal:** Implement JSONL logging and context management with token-based auto-compaction at 80% capacity.

**Architecture:**
- Detect project root (git repo or cwd) and normalize project name for filesystem
- Log OpenAI-compatible request/response to `~/.synthia/projects/<project_name>/YYYYMMDD.jsonl`
- Track token usage and trigger auto-compaction at 80% of context window
- Support manual compaction via menu
- Use local/remote LLM for summarization (configurable)
- Replace summarized portions of context with summary message

**Key Design Decisions:**
1. **JSONL format**: Each line = one complete request/response turn, matching Claude Code's approach
2. **Token tracking**: Use actual token counts from API responses, not message counts
3. **80% threshold**: Auto-compact when `current_tokens >= (model_max_tokens * 0.8)`
4. **Project naming**: Normalize with lowercase, replace spaces/special chars with `_`, max 50 chars
5. **Storage**: `~/.synthia/projects/<normalized_project_name>/YYYYMMDD_HHMMSS.jsonl`
6. **File rotation**: Create new JSONL file when current file exceeds 10MB (configurable)

**Tech Stack:** Rust, serde_json, chrono, existing ContextManager

---

## Task 1: Add project detection and name normalization

**Files:**
- Create: `synthia/src/project.rs` (new module)
- Modify: `synthia/src/lib.rs` (add module declaration)
- Test: Unit tests for normalization

**Step 1: Create project.rs with detection logic**

```rust
// synthia/src/project.rs
use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Detects the project root by looking for .git directory
/// Falls back to current directory if not a git repo
pub fn detect_project_root() -> Result<PathBuf> {
    // Try git root first
    if let Ok(git_root) = detect_git_root() {
        return Ok(git_root);
    }

    // Fall back to current directory
    env::current_dir().context("Failed to get current directory")
}

/// Detects git repository root
fn detect_git_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to run git command")?;

    if !output.status.success() {
        anyhow::bail!("Not a git repository");
    }

    let path_str = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git output")?
        .trim()
        .to_string();

    Ok(PathBuf::from(path_str))
}

/// Extracts project name from path (last component)
pub fn extract_project_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("Failed to extract project name from path")
}

/// Normalizes project name for filesystem safety
/// - Converts to lowercase
/// - Replaces spaces and special chars with underscore
/// - Truncates to 50 characters
/// - Ensures ASCII-safe
pub fn normalize_project_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .chars()
        .take(50)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_project_name() {
        assert_eq!(normalize_project_name("My Project"), "my_project");
        assert_eq!(normalize_project_name("Agent-Powertools"), "agent-powertools");
        assert_eq!(normalize_project_name("Project@#$%123"), "project____123");
        assert_eq!(normalize_project_name("hello world! 2024"), "hello_world__2024");

        // Test truncation
        let long_name = "a".repeat(100);
        assert_eq!(normalize_project_name(&long_name).len(), 50);
    }

    #[test]
    fn test_extract_project_name() {
        let path = PathBuf::from("/home/user/projects/my-project");
        assert_eq!(extract_project_name(&path).unwrap(), "my-project");

        let path = PathBuf::from("/home/user/projects/Agent Powertools");
        assert_eq!(extract_project_name(&path).unwrap(), "Agent Powertools");
    }
}
```

**Step 2: Add module declaration**

```rust
// In synthia/src/lib.rs, add:
pub mod project;
```

**Step 3: Run tests**

```bash
cd synthia && cargo test project::tests
```

Expected: All normalization tests pass

**Step 4: Commit**

```bash
git add synthia/src/project.rs synthia/src/lib.rs
git commit -m "feat(project): Add project detection and name normalization"
```

---

## Task 2: Create JSONL logger infrastructure

**Files:**
- Create: `synthia/src/jsonl_logger.rs` (new module)
- Modify: `synthia/src/lib.rs` (add module)
- Test: Write integration test

**Step 1: Create jsonl_logger.rs**

```rust
// synthia/src/jsonl_logger.rs
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Represents a single JSONL entry (one request/response turn)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonlEntry {
    pub timestamp: i64,
    pub request: RequestLog,
    pub response: ResponseLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub model: String,
    pub messages: Vec<Value>,
    pub tools: Vec<Value>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseLog {
    pub content: Vec<Value>,  // ContentBlocks as JSON
    pub stop_reason: String,
    pub usage: TokenUsageLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageLog {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
}

const MAX_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

pub struct JsonlLogger {
    project_name: String,
    log_dir: PathBuf,
    current_log_file: Option<PathBuf>,
    max_file_size: u64,
}

impl JsonlLogger {
    /// Create a new JSONL logger for a project
    pub fn new(project_name: String) -> Result<Self> {
        let log_dir = get_projects_log_dir()?.join(&project_name);

        // Ensure directory exists
        fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

        Ok(Self {
            project_name,
            log_dir,
            current_log_file: None,
            max_file_size: MAX_LOG_FILE_SIZE,
        })
    }

    /// Log a request/response pair to current JSONL file
    /// Creates new file if current exceeds size limit
    pub fn log_turn(&mut self, entry: JsonlEntry) -> Result<()> {
        let log_file_path = self.get_or_create_log_file()?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .with_context(|| format!("Failed to open log file: {:?}", log_file_path))?;

        let json_line = serde_json::to_string(&entry)
            .context("Failed to serialize JSONL entry")?;

        writeln!(file, "{}", json_line)
            .with_context(|| format!("Failed to write to log file: {:?}", log_file_path))?;

        tracing::debug!("Logged turn to {:?}", log_file_path);
        Ok(())
    }

    /// Get or create a log file, rotating if size exceeds limit
    fn get_or_create_log_file(&mut self) -> Result<PathBuf> {
        // Check if current file needs rotation
        if let Some(current_path) = &self.current_log_file {
            if current_path.exists() {
                let metadata = fs::metadata(current_path)
                    .context("Failed to get file metadata")?;

                if metadata.len() < self.max_file_size {
                    return Ok(current_path.clone());
                }

                tracing::info!("Rotating log file (size: {} bytes)", metadata.len());
            }
        }

        // Create new log file with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let new_path = self.log_dir.join(format!("{}.jsonl", timestamp));

        self.current_log_file = Some(new_path.clone());
        Ok(new_path)
    }

    /// Get the project name
    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    /// List all JSONL log files for this project
    pub fn list_log_files(&self) -> Result<Vec<PathBuf>> {
        let mut log_files = Vec::new();

        if !self.log_dir.exists() {
            return Ok(log_files);
        }

        for entry in fs::read_dir(&self.log_dir)
            .with_context(|| format!("Failed to read log directory: {:?}", self.log_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                log_files.push(path);
            }
        }

        log_files.sort();
        Ok(log_files)
    }
}

/// Get the base directory for project logs: ~/.synthia/projects/
fn get_projects_log_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .context("Failed to get home directory")?;

    Ok(home_dir.join(".synthia").join("projects"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_jsonl_entry_serialization() {
        let entry = JsonlEntry {
            timestamp: 1234567890,
            request: RequestLog {
                model: "test-model".to_string(),
                messages: vec![serde_json::json!({"role": "user", "content": "Hello"})],
                tools: vec![],
                temperature: 0.7,
                max_tokens: Some(1000),
                stream: true,
            },
            response: ResponseLog {
                content: vec![serde_json::json!({"type": "text", "text": "Hi there!"})],
                stop_reason: "end_turn".to_string(),
                usage: TokenUsageLog {
                    input_tokens: 10,
                    output_tokens: 5,
                    total_tokens: 15,
                },
            },
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: JsonlEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.timestamp, 1234567890);
        assert_eq!(parsed.request.model, "test-model");
        assert_eq!(parsed.response.usage.total_tokens, 15);
    }
}
```

**Step 2: Add module to lib.rs**

```rust
// In synthia/src/lib.rs
pub mod jsonl_logger;
```

**Step 3: Run tests**

```bash
cd synthia && cargo test jsonl_logger::tests
```

**Step 4: Commit**

```bash
git add synthia/src/jsonl_logger.rs synthia/src/lib.rs
git commit -m "feat(logging): Add JSONL logger infrastructure for request/response logging"
```

---

## Task 3: Integrate JSONL logging into OpenAIProvider

**Files:**
- Modify: `synthia/src/llm/openai.rs` (add logging hooks)
- Modify: `synthia/src/agent/actor.rs` (pass logger to provider)

**Step 1: Add JsonlLogger to AgentActor**

```rust
// In synthia/src/agent/actor.rs
use crate::jsonl_logger::{JsonlLogger, JsonlEntry, RequestLog, ResponseLog, TokenUsageLog};
use crate::project::{detect_project_root, extract_project_name, normalize_project_name};

pub struct AgentActor {
    // ... existing fields ...
    jsonl_logger: JsonlLogger,  // NEW (not Option since we always create it)
}

impl AgentActor {
    pub fn new(/* ... */) -> Self {
        // Detect project and create logger
        let project_name = detect_project_root()
            .and_then(|root| extract_project_name(&root))
            .map(|name| normalize_project_name(&name))
            .unwrap_or_else(|_| {
                tracing::warn!("Failed to detect project, using 'default' as project name");
                "default".to_string()
            });

        let jsonl_logger = JsonlLogger::new(project_name)
            .expect("Failed to create JSONL logger");

        tracing::info!("JSONL logger initialized for project: {}", jsonl_logger.project_name());

        Self {
            // ... existing fields ...
            jsonl_logger,
        }
    }
}
```

**Step 2: Log requests/responses in handle_message**

```rust
// In synthia/src/agent/actor.rs, in handle_message() method
// After getting LLM response:

let entry = JsonlEntry {
    timestamp: chrono::Utc::now().timestamp(),
    request: RequestLog {
        model: self.config.model.clone(),
        messages: self.conversation.iter()
            .map(|m| serde_json::to_value(m).unwrap_or(serde_json::json!(null)))
            .collect(),
        tools: vec![], // TODO: Include tool definitions
        temperature: self.config.temperature,
        max_tokens: self.config.max_tokens,
        stream: self.config.streaming,
    },
    response: ResponseLog {
        content: response.content.iter()
            .map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!(null)))
            .collect(),
        stop_reason: format!("{:?}", response.stop_reason),
        usage: TokenUsageLog {
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
        },
    },
};

if let Err(e) = self.jsonl_logger.log_turn(entry) {
    tracing::error!("Failed to log turn to JSONL: {}", e);
}
```

**Step 3: Build and test**

```bash
cd synthia && cargo build
# Run synthia, send a message, then check ~/.synthia/projects/<project>/YYYYMMDD_HHMMSS.jsonl
# Example: ~/.synthia/projects/agent-powertools/20251023_143022.jsonl
```

**Step 4: Commit**

```bash
git add synthia/src/agent/actor.rs
git commit -m "feat(logging): Integrate JSONL logging into AgentActor"
```

---

## Task 4: Add token counting to ContextManager

**Files:**
- Modify: `synthia/src/context_manager.rs` (add token tracking)

**Step 1: Add token tracking fields**

```rust
// In synthia/src/context_manager.rs
pub struct ContextManager {
    messages: Vec<Message>,
    max_messages: usize,
    summary_threshold: usize,
    llm_provider: Arc<dyn LLMProvider>,
    current_token_count: usize,      // NEW: Track current context tokens
    max_token_limit: usize,          // NEW: Model's max context window
    token_threshold_percent: f32,    // NEW: Auto-compact at this % (default 0.8)
}

impl ContextManager {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            messages: Vec::new(),
            max_messages: MAX_MESSAGES,
            summary_threshold: SUMMARY_THRESHOLD,
            llm_provider,
            current_token_count: 0,
            max_token_limit: 8192,  // Default, should be configurable per model
            token_threshold_percent: 0.8,
        }
    }

    /// Set the max token limit for this model
    pub fn set_max_token_limit(&mut self, limit: usize) {
        self.max_token_limit = limit;
        tracing::info!("Context manager max token limit set to {}", limit);
    }

    /// Update token count after each response
    pub fn update_token_count(&mut self, input_tokens: usize, output_tokens: usize) {
        self.current_token_count = input_tokens + output_tokens;

        let threshold = (self.max_token_limit as f32 * self.token_threshold_percent) as usize;
        let usage_percent = (self.current_token_count as f32 / self.max_token_limit as f32) * 100.0;

        tracing::debug!(
            "Token usage: {} / {} ({:.1}%)",
            self.current_token_count,
            self.max_token_limit,
            usage_percent
        );

        if self.current_token_count >= threshold {
            tracing::info!("Token threshold reached ({}/{} = {:.1}%), compaction recommended",
                self.current_token_count, self.max_token_limit, usage_percent);
        }
    }

    /// Check if auto-compaction should trigger
    pub fn should_compact(&self) -> bool {
        let threshold = (self.max_token_limit as f32 * self.token_threshold_percent) as usize;
        self.current_token_count >= threshold
    }

    /// Get current token usage stats
    pub fn get_token_stats(&self) -> TokenStats {
        let threshold = (self.max_token_limit as f32 * self.token_threshold_percent) as usize;
        let usage_percent = (self.current_token_count as f32 / self.max_token_limit as f32) * 100.0;

        TokenStats {
            current: self.current_token_count,
            max: self.max_token_limit,
            threshold,
            usage_percent,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenStats {
    pub current: usize,
    pub max: usize,
    pub threshold: usize,
    pub usage_percent: f32,
}
```

**Step 2: Update compact_if_needed to use tokens**

```rust
// In synthia/src/context_manager.rs
pub async fn compact_if_needed(&mut self) -> Result<()> {
    // Use token-based threshold instead of message count
    if self.should_compact() {
        self.summarize_oldest_messages().await?;

        // After compaction, estimate new token count (conservative: assume 50% reduction)
        self.current_token_count = (self.current_token_count as f32 * 0.5) as usize;
        tracing::info!("Estimated token count after compaction: {}", self.current_token_count);
    }

    Ok(())
}
```

**Step 3: Export TokenStats in lib.rs**

```rust
// In synthia/src/lib.rs
pub use context_manager::TokenStats;
```

**Step 4: Commit**

```bash
git add synthia/src/context_manager.rs synthia/src/lib.rs
git commit -m "feat(context): Add token-based tracking and 80% auto-compaction threshold"
```

---

## Task 5: Wire token updates in AgentActor

**Files:**
- Modify: `synthia/src/agent/actor.rs` (update token counts after each response)

**Step 1: Update ContextManager after each response**

```rust
// In synthia/src/agent/actor.rs, after receiving LLM response
// In handle_message() method:

// Update token count in context manager
self.context_manager.update_token_count(
    response.usage.input_tokens,
    response.usage.output_tokens,
);

// Check if auto-compaction should trigger
if self.context_manager.should_compact() {
    tracing::info!("Auto-compaction triggered at 80% context usage");

    if let Err(e) = self.context_manager.compact_if_needed().await {
        tracing::error!("Failed to compact context: {}", e);
    } else {
        // Update conversation from compacted context
        self.conversation = self.context_manager.get_messages().to_vec();

        // Notify UI
        let _ = self.ui_tx.send(UIUpdate::SystemMessage(
            "Context auto-compacted (80% threshold reached)".to_string()
        )).await;
    }
}
```

**Step 2: Add SystemMessage variant to UIUpdate**

```rust
// In synthia/src/agent/messages.rs
#[derive(Debug)]
pub enum UIUpdate {
    // ... existing variants ...
    SystemMessage(String),  // NEW: Display system notifications
}
```

**Step 3: Handle SystemMessage in App**

```rust
// In synthia/src/ui/app.rs, in handle_ui_update():
UIUpdate::SystemMessage(msg) => {
    print_colored_line(stdout, &format!("[System] {}", msg), Color::Yellow)?;
    stdout.flush()?;
}
```

**Step 4: Commit**

```bash
git add synthia/src/agent/actor.rs synthia/src/agent/messages.rs synthia/src/ui/app.rs
git commit -m "feat(context): Wire auto-compaction with 80% token threshold"
```

---

## Task 6: Add manual compaction command

**Files:**
- Modify: `synthia/src/agent/messages.rs` (add Command::CompactContext)
- Modify: `synthia/src/agent/actor.rs` (handle CompactContext command)

**Step 1: Add Command variant**

```rust
// In synthia/src/agent/messages.rs
#[derive(Debug, Clone)]
pub enum Command {
    // ... existing variants ...
    CompactContext,  // NEW: Manual context compaction
    ViewContextStats,  // NEW: View token usage stats
}
```

**Step 2: Handle commands in AgentActor**

```rust
// In synthia/src/agent/actor.rs, in run() method
Command::CompactContext => {
    tracing::info!("Manual compaction requested");

    if let Err(e) = self.context_manager.compact_if_needed().await {
        tracing::error!("Failed to compact context: {}", e);
        let _ = self.ui_tx.send(UIUpdate::SystemMessage(
            format!("Compaction failed: {}", e)
        )).await;
    } else {
        // Update conversation
        self.conversation = self.context_manager.get_messages().to_vec();

        let stats = self.context_manager.get_token_stats();
        let _ = self.ui_tx.send(UIUpdate::SystemMessage(
            format!(
                "Context compacted successfully. Usage: {} / {} tokens ({:.1}%)",
                stats.current, stats.max, stats.usage_percent
            )
        )).await;
    }
}

Command::ViewContextStats => {
    let stats = self.context_manager.get_token_stats();
    let _ = self.ui_tx.send(UIUpdate::SystemMessage(
        format!(
            "Context Usage: {} / {} tokens ({:.1}%) | Threshold: {} tokens (80%)",
            stats.current, stats.max, stats.usage_percent, stats.threshold
        )
    )).await;
}
```

**Step 3: Commit**

```bash
git add synthia/src/agent/messages.rs synthia/src/agent/actor.rs
git commit -m "feat(context): Add manual compaction and context stats commands"
```

---

## Task 7: Wire "Context Management" menu item

**Files:**
- Modify: `synthia/src/ui/app.rs` (implement context management submenu)

**Step 1: Add context submenu state to App**

```rust
// In synthia/src/ui/app.rs, App struct
pub struct App {
    // ... existing fields ...
    show_context_submenu: bool,      // NEW
    context_submenu_selected: usize, // NEW
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            // ... existing fields ...
            show_context_submenu: false,
            context_submenu_selected: 0,
        }
    }
}
```

**Step 2: Update menu to enable Context Management**

```rust
// In synthia/src/ui/app.rs, in render_menu() method
// Remove "Coming Soon" from Context Management item
let menu_items = vec![
    "Set Session Name",
    "Save Session",
    "New Session",
    "Set Reasoning Level",
    "Context Management",  // NOW ACTIVE
    "Toggle Mode (Coming Soon)",
];
```

**Step 3: Add context submenu handler in handle_menu_selection**

```rust
// In synthia/src/ui/app.rs, in handle_menu_selection()
4 => {
    // Context Management - show submenu
    self.show_context_submenu(stdout)?;
}
```

**Step 4: Implement context submenu rendering**

```rust
// In synthia/src/ui/app.rs
impl App {
    fn show_context_submenu(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        self.show_menu = false;
        self.show_context_submenu = true;
        self.context_submenu_selected = 0;
        self.render_context_submenu(stdout)
    }

    fn render_context_submenu(&self, stdout: &mut impl Write) -> io::Result<()> {
        self.clear_input_line(stdout)?;

        execute!(stdout, Print("\r\n=== Context Management (↑/↓ navigate | Enter select | Esc cancel) ===\n"))?;

        let items = vec![
            ("View Context Stats", "Show current token usage"),
            ("Manual Compact", "Trigger context compaction now"),
            ("View Activity Logs", "Browse JSONL conversation logs"),
        ];

        for (idx, (label, desc)) in items.iter().enumerate() {
            let selected = if idx == self.context_submenu_selected { ">" } else { " " };

            if idx == self.context_submenu_selected {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
            }

            execute!(stdout, Print(format!("\r{} {} - {}\n", selected, label, desc)))?;

            if idx == self.context_submenu_selected {
                queue!(stdout, ResetColor)?;
            }
        }

        execute!(stdout, Print("\r\n"))?;
        stdout.flush()
    }
}
```

**Step 5: Add context submenu navigation in handle_input**

```rust
// In synthia/src/ui/app.rs, in handle_input(), add after other submenu handling:
if self.show_context_submenu {
    match key.code {
        KeyCode::Up => {
            if self.context_submenu_selected > 0 {
                self.context_submenu_selected -= 1;
                self.render_context_submenu(stdout)?;
            }
            return Ok(());
        }
        KeyCode::Down => {
            if self.context_submenu_selected < 2 {  // 3 options
                self.context_submenu_selected += 1;
                self.render_context_submenu(stdout)?;
            }
            return Ok(());
        }
        KeyCode::Enter => {
            self.handle_context_submenu_selection(stdout).await?;
            return Ok(());
        }
        KeyCode::Esc => {
            self.show_context_submenu = false;
            self.show_menu = true;
            self.render_menu(stdout)?;
            return Ok(());
        }
        _ => return Ok(()),
    }
}
```

**Step 6: Implement context submenu selection handler**

```rust
// In synthia/src/ui/app.rs
impl App {
    async fn handle_context_submenu_selection(&mut self, stdout: &mut impl Write) -> anyhow::Result<()> {
        match self.context_submenu_selected {
            0 => {
                // View Context Stats
                self.cmd_tx.send(Command::ViewContextStats).await?;
                self.show_context_submenu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            1 => {
                // Manual Compact
                self.cmd_tx.send(Command::CompactContext).await?;
                self.show_context_submenu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            2 => {
                // View Activity Logs (TODO: implement JSONL viewer)
                self.show_context_submenu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
                print_colored_line(stdout, "JSONL log viewer coming soon!", Color::Yellow)?;
            }
            _ => {}
        }

        Ok(())
    }
}
```

**Step 7: Update modal flag checks in run() method**

```rust
// In synthia/src/ui/app.rs, in run() method
if !self.show_session_list
    && !self.show_menu
    && !self.show_reasoning_submenu
    && !self.show_session_name_input
    && !self.show_context_submenu  // NEW
    && self.input_needs_render {
```

**Step 8: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "feat(ui): Add Context Management submenu with stats and manual compact"
```

---

## Task 8: Add context stats to header/status line

**Files:**
- Modify: `synthia/src/ui/app.rs` (display token usage in header)

**Step 1: Add token stats tracking to App struct**

```rust
// In synthia/src/ui/app.rs, App struct
pub struct App {
    // ... existing fields ...
    token_stats: Option<TokenStats>,  // NEW: Cache latest token stats
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            // ... existing fields ...
            token_stats: None,
        }
    }
}
```

**Step 2: Add TokenStatsUpdate UIUpdate variant**

```rust
// In synthia/src/agent/messages.rs
use crate::context_manager::TokenStats;

#[derive(Debug)]
pub enum UIUpdate {
    // ... existing variants ...
    TokenStatsUpdate(TokenStats),  // NEW: Update token usage display
}
```

**Step 3: Send token stats after each response**

```rust
// In synthia/src/agent/actor.rs, after updating token count:
self.context_manager.update_token_count(
    response.usage.input_tokens,
    response.usage.output_tokens,
);

// Send updated stats to UI
let stats = self.context_manager.get_token_stats();
let _ = self.ui_tx.send(UIUpdate::TokenStatsUpdate(stats)).await;
```

**Step 4: Handle TokenStatsUpdate in App**

```rust
// In synthia/src/ui/app.rs, in handle_ui_update():
UIUpdate::TokenStatsUpdate(stats) => {
    self.token_stats = Some(stats);
    // No need to render, will be shown in next header render
}
```

**Step 5: Update print_header to include token usage**

```rust
// In synthia/src/ui/app.rs, in print_header() method
fn print_header(&self, stdout: &mut impl Write) -> io::Result<()> {
    writeln!(stdout, "╔════════════════════════════════════════════════════════════════╗")?;
    writeln!(stdout, "║  Synthia - Your Proactive AI Assistant                         ║")?;
    writeln!(stdout, "╠════════════════════════════════════════════════════════════════╣")?;

    // Add token usage if available
    if let Some(stats) = &self.token_stats {
        let usage_str = format!(
            "║  Context: {} / {} tokens ({:.0}%) {}",
            stats.current,
            stats.max,
            stats.usage_percent,
            if stats.current >= stats.threshold { "⚠" } else { "" }
        );

        // Pad to align with box border
        let padding = 65 - usage_str.len();
        writeln!(stdout, "{}{}║", usage_str, " ".repeat(padding))?;
        writeln!(stdout, "╠════════════════════════════════════════════════════════════════╣")?;
    }

    writeln!(stdout, "║  Ctrl+P: Menu | Ctrl+L: Sessions | Ctrl+C: Cancel | Ctrl+D: Exit ║")?;
    writeln!(stdout, "╚════════════════════════════════════════════════════════════════╝")?;
    stdout.flush()
}
```

**Step 6: Commit**

```bash
git add synthia/src/ui/app.rs synthia/src/agent/actor.rs synthia/src/agent/messages.rs
git commit -m "feat(ui): Display token usage stats in header with warning indicator"
```

---

## Task 9: Configure max token limit per model

**Files:**
- Modify: `synthia/config.toml` (add model_context_window config)
- Modify: `synthia/src/config.rs` (read config)
- Modify: `synthia/src/agent/actor.rs` (set limit on ContextManager)

**Step 1: Add config field**

```toml
# In synthia/config.toml
[llm]
# ... existing fields ...

# Context window size for the model
# This determines when auto-compaction triggers (at 80%)
# Common values:
#   - qwen2.5-coder-7b: 8192
#   - gpt-4: 8192
#   - claude-3: 200000
context_window = 8192
```

**Step 2: Update Config struct**

```rust
// In synthia/src/config.rs (create if doesn't exist)
#[derive(Debug, Clone, Deserialize)]
pub struct LLMConfig {
    pub api_base: String,
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub context_window: usize,  // NEW
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            api_base: "http://localhost:1234/v1".to_string(),
            api_key: None,
            model: "qwen2.5-coder-7b-instruct".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            context_window: 8192,  // NEW: default
        }
    }
}
```

**Step 3: Set context window in AgentActor**

```rust
// In synthia/src/agent/actor.rs, in new() method
let mut context_manager = ContextManager::new(llm_provider.clone());
context_manager.add_message(Self::create_system_prompt());
context_manager.set_max_token_limit(config.context_window);  // NEW
```

**Step 4: Commit**

```bash
git add synthia/config.toml synthia/src/config.rs synthia/src/agent/actor.rs
git commit -m "feat(config): Add configurable context_window for token limit"
```

---

## Task 10: Add JSONL log viewer (basic)

**Files:**
- Modify: `synthia/src/ui/app.rs` (implement log viewer modal)

**Step 1: Add log viewer state to App**

```rust
// In synthia/src/ui/app.rs, App struct
pub struct App {
    // ... existing fields ...
    show_log_viewer: bool,           // NEW
    log_entries: Vec<String>,        // NEW: Loaded JSONL entries
    log_viewer_selected: usize,      // NEW: Selected entry index
}
```

**Step 2: Implement log viewer rendering (placeholder)**

```rust
// In synthia/src/ui/app.rs
impl App {
    fn render_log_viewer(&self, stdout: &mut impl Write) -> io::Result<()> {
        execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        execute!(stdout, Print("\r\n=== JSONL Activity Logs (↑/↓ navigate | Esc close) ===\n"))?;

        if self.log_entries.is_empty() {
            execute!(stdout, Print("\rNo JSONL logs found for this project.\n"))?;
        } else {
            for (idx, entry) in self.log_entries.iter().enumerate().take(10) {
                let selected = if idx == self.log_viewer_selected { ">" } else { " " };

                // Truncate entry for display
                let preview = if entry.len() > 80 {
                    format!("{}...", &entry[..80])
                } else {
                    entry.clone()
                };

                execute!(stdout, Print(format!("\r{} {}\n", selected, preview)))?;
            }
        }

        execute!(stdout, Print("\r\n(Full viewer implementation coming soon)\n"))?;
        stdout.flush()
    }
}
```

**Step 3: Update context submenu selection to show viewer**

```rust
// In synthia/src/ui/app.rs, in handle_context_submenu_selection()
2 => {
    // View Activity Logs
    self.show_context_submenu = false;
    self.show_log_viewer = true;
    self.log_entries.clear();  // TODO: Load actual JSONL entries
    self.render_log_viewer(stdout)?;
}
```

**Step 4: Add log viewer navigation**

```rust
// In synthia/src/ui/app.rs, in handle_input()
if self.show_log_viewer {
    match key.code {
        KeyCode::Esc => {
            self.show_log_viewer = false;
            execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            self.print_header(stdout)?;
            return Ok(());
        }
        _ => return Ok(()),
    }
}
```

**Step 5: Update modal flag checks**

```rust
// In synthia/src/ui/app.rs, in run() method
if !self.show_session_list
    && !self.show_menu
    && !self.show_reasoning_submenu
    && !self.show_session_name_input
    && !self.show_context_submenu
    && !self.show_log_viewer  // NEW
    && self.input_needs_render {
```

**Step 6: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "feat(ui): Add basic JSONL log viewer placeholder"
```

---

## Task 11: Update existing tests

**Files:**
- Modify: `synthia/src/context_manager.rs` (update test expectations)
- Modify: `synthia/src/agent/actor.rs` (update mocks if needed)

**Step 1: Update context manager tests for token-based compaction**

```rust
// In synthia/src/context_manager.rs, update tests
#[tokio::test]
async fn test_compact_at_token_threshold() {
    let provider = Arc::new(MockLLMProvider);
    let mut context_manager = ContextManager::new(provider);
    context_manager.set_max_token_limit(1000);  // Small limit for testing

    // Add system message
    context_manager.add_message(Message {
        role: Role::System,
        content: vec![ContentBlock::Text {
            text: "System prompt".to_string(),
        }],
    });

    // Simulate reaching 80% token usage
    context_manager.update_token_count(600, 200);  // 800 tokens = 80%

    assert!(context_manager.should_compact());

    // Trigger compaction
    context_manager.compact_if_needed().await.unwrap();

    // Token count should be reduced (estimated)
    assert!(context_manager.get_token_stats().current < 800);
}
```

**Step 2: Run all tests**

```bash
cd synthia && cargo test
```

Expected: All tests pass

**Step 3: Commit**

```bash
git add synthia/src/context_manager.rs
git commit -m "test(context): Update tests for token-based compaction"
```

---

## Task 12: Integration testing and documentation

**Files:**
- Update: `README.md` or `docs/USAGE.md` (document context management)
- Manual testing

**Step 1: Manual integration test**

```bash
cd synthia && cargo build --release
./target/release/synthia

# Test flow:
# 1. Start conversation, send several long messages
# 2. Check ~/.synthia/projects/<project>/YYYYMMDD.jsonl exists and has entries
# 3. Press Ctrl+P → Context Management → View Context Stats
# 4. Verify token usage displays correctly
# 5. Continue conversation until 80% threshold
# 6. Verify auto-compaction triggers with system message
# 7. Press Ctrl+P → Context Management → Manual Compact
# 8. Verify compaction works and token count reduces
# 9. Check header shows token usage with warning indicator at 80%+
```

**Step 2: Document context management features**

Add to `README.md` or create `docs/CONTEXT_MANAGEMENT.md`:

```markdown
# Context Management

Synthia automatically manages conversation context to prevent token limit errors.

## Features

- **Auto-compaction at 80%**: When context usage reaches 80% of model's window, oldest messages are automatically summarized
- **Token-based tracking**: Uses actual token counts from API responses
- **JSONL logging**: All requests/responses logged to `~/.synthia/projects/<project>/YYYYMMDD.jsonl`
- **Manual compaction**: Trigger summarization anytime via menu
- **Context stats**: View current token usage in header and via menu

## Usage

**View Context Stats:**
1. Press `Ctrl+P` to open menu
2. Navigate to "Context Management"
3. Select "View Context Stats"

**Manual Compaction:**
1. Press `Ctrl+P`
2. Navigate to "Context Management"
3. Select "Manual Compact"

**View Activity Logs:**
1. Press `Ctrl+P`
2. Navigate to "Context Management"
3. Select "View Activity Logs"

## Configuration

Set your model's context window in `config.toml`:

```toml
[llm]
context_window = 8192  # Adjust for your model
```

Common values:
- `qwen2.5-coder-7b`: 8192
- `gpt-4`: 8192
- `claude-3`: 200000
```

**Step 3: Commit**

```bash
git add README.md docs/
git commit -m "docs: Add context management documentation"
```

---

## Success Criteria

**After completing all tasks:**

1. ✅ Project detection works (git root or cwd)
2. ✅ Project names normalized and filesystem-safe
3. ✅ JSONL logging writes to `~/.synthia/projects/<project>/YYYYMMDD.jsonl`
4. ✅ Each JSONL line contains full request + response
5. ✅ Token counting tracks actual API usage
6. ✅ Auto-compaction triggers at 80% token usage
7. ✅ Manual compaction works via menu
8. ✅ Context stats display in header with warning indicator
9. ✅ Context Management menu has 3 working options
10. ✅ JSONL log viewer renders (basic placeholder)
11. ✅ All existing tests pass
12. ✅ Documentation complete

**Future work (Phase 3):**
- Enhanced JSONL viewer with syntax highlighting
- Context search across JSONL logs
- Export context to markdown
- Planning vs Execution mode toggle
- Multi-session context aggregation

---

## Testing Checklist

**Manual testing:**
- [ ] Start synthia in a git repo, verify project detected
- [ ] Start synthia in non-git folder, verify cwd used
- [ ] Send message, verify JSONL file created at `~/.synthia/projects/<project>/YYYYMMDD_HHMMSS.jsonl`
- [ ] Inspect JSONL file, verify request/response structure correct
- [ ] Send multiple large messages (to test 10MB rotation), verify new file created when size exceeded
- [ ] Send multiple messages, verify token stats update in header
- [ ] Continue until 80%, verify auto-compaction triggers
- [ ] Verify system message displays after auto-compact
- [ ] Press Ctrl+P → Context Management → View Context Stats
- [ ] Press Ctrl+P → Context Management → Manual Compact
- [ ] Press Ctrl+P → Context Management → View Activity Logs
- [ ] Verify header shows warning indicator (⚠) at 80%+
- [ ] Run existing test suite: `cargo test`
