# Synthia Implementation Plan

> **For Claude:** Use `${SUPERPOWERS_SKILLS_ROOT}/skills/collaboration/executing-plans/SKILL.md` to implement this plan task-by-task.

**Goal:** Build Synthia, a Claude Code clone for local LLMs with comprehensive tooling, actor-based architecture, and rich TUI.

**Architecture:** Actor model with tokio channels for message passing. Separate actors for UI (ratatui), Agent (LLM orchestration), Tool Executor, and Session management. Tools implement a common trait for uniform execution.

**Tech Stack:** Rust, tokio (async runtime), ratatui + crossterm (TUI), reqwest (HTTP client), serde_json (serialization), anyhow (errors), tracing (logging)

---

## 🎉 Implementation Status

**Last Updated:** 2025-10-17
**Status:** Phase 1 & 2 Complete ✅

### Completed Tasks (Phase 1)

- ✅ **Task 1:** Project Scaffolding (commit: 1765662)
- ✅ **Task 2:** Core Type Definitions (commit: 0c89b0e, fix: 7997dfd)
- ✅ **Task 3:** Tool Trait and Registry (commit: c4b088d, fix: ff4ea4b)
- ✅ **Task 4:** Bash Tool Implementation (commit: f40aecd)
- ✅ **Task 5:** Read, Write, Edit Tools (commit: 8f1ef74, fix: 48ccea9)
- ✅ **Task 6:** LLM Provider Trait and OpenAI Implementation (commit: b91a377, fix: 1dfe449)
- ✅ **Task 7:** Basic Agent Loop (commit: b792312, fix: 56442c4)
- ✅ **Task 8:** Basic TUI (commit: 13bb21c, fix: c5faa34)

### Completed Tasks (Phase 2)

- ✅ **Task 9:** Grep and Glob Tools (commit: 594e0d4)
- ✅ **Task 10:** WebFetch Tool (commit: 9008a5b, fix: 97865f0)
- ✅ **Task 11:** Git Tools (commit: 88bda22)
- ✅ **Task 12:** Powertools Integration (commit: 9b41f30)
- ✅ **Task 13:** Workshop Integration (commit: c4f0738, fix: d65b692)

### Critical Fixes Applied (All Phases)

**Phase 1:**
1. **Task 2 Fix:** Added serialization derives to StopReason and TokenUsage
2. **Task 3 Fix:** Added Default trait and collision handling to ToolRegistry
3. **Task 5 Fix:** Made ReadTool fully async and added overwrite test for WriteTool
4. **Task 6 Fix:** Converted tool definitions to OpenAI format and fixed message conversion for API compatibility
5. **Task 7 Fix:** Prevented duplicate assistant messages in conversation state
6. **Task 8 Fix:** Converted tool definitions to OpenAI format for LM Studio compatibility

**Phase 2:**
7. **Task 10 Fix:** Added proper URL validation, header type checking, and response size limits to WebFetch
8. **Task 13 Fix:** Fixed memory safety issue in Workshop gotcha function and corrected repository URL

### Current State

**Working Features:**
- ✅ Full TUI with conversation display and input
- ✅ Integration with LM Studio (localhost:1234)
- ✅ Agentic loop: Think → Act → Observe
- ✅ **Comprehensive toolset (10 tools):**
  - File operations: Bash, Read, Write, Edit
  - Search: Grep, Glob
  - Network: WebFetch (HTTP/HTTPS with security validation)
  - Version control: Git (status, diff, log, add, commit, push)
  - Code navigation: Powertools (semantic indexing, goto definition, find references)
  - Context management: Workshop (notes, decisions, gotchas, search)
- ✅ Tool timing feedback in UI
- ✅ Error handling and display
- ✅ Graceful shutdown (Ctrl+D)

**Test Coverage:**
- **71 tests passing** (up from 19 in Phase 1)
- Build: Clean (7 non-blocking warnings)
- Binary size: ~5 MB

**Known Limitations:**
- **TUI scrolling bug:** Messages go below visible area when conversation fills the screen - needs auto-scroll to bottom
- No streaming text (appears all at once) - deferred to Task 17 (Phase 3)
- No "Thinking..." indicator while waiting - deferred to Task 17 (Phase 3)
- Cancellation not implemented (Ctrl+C does nothing) - noted as TODO
- No markdown rendering in TUI - deferred to Task 14 (Phase 3)
- No session persistence - deferred to Task 16 (Phase 3)

### Next Steps (Phase 3)

The following tasks are planned for polish and advanced features:
- Task 14: Markdown Rendering in TUI
- Task 15: Configuration System
- Task 16: Session Persistence
- Task 17: Streaming Text Support

---

## Phase 1: Foundation (Sessions 1-2)

### Task 1: Project Scaffolding

**Agent:** architect
**Files:**
- Create: `synthia/Cargo.toml`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Update workspace Cargo.toml**

Edit `Cargo.toml` to add synthia as workspace member:

```toml
[workspace]
members = ["powertools-cli", "synthia"]
resolver = "2"
```

**Step 2: Create synthia directory and Cargo.toml**

```bash
mkdir -p synthia/src
```

Create `synthia/Cargo.toml`:

```toml
[package]
name = "synthia"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
ratatui = "0.28"
crossterm = "0.28"
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
futures = "0.3"

[dev-dependencies]
tokio-test = "0.4"
mockito = "1"
```

**Step 3: Create main.rs with basic structure**

Create `synthia/src/main.rs`:

```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Synthia starting...");
    Ok(())
}
```

**Step 4: Verify it builds**

```bash
cd synthia
cargo build
```

Expected: BUILD SUCCESS

**Step 5: Commit**

```bash
git add Cargo.toml synthia/
git commit -m "feat: initialize synthia workspace member"
```

---

### Task 2: Core Type Definitions

**Agent:** architect
**Files:**
- Create: `synthia/src/types.rs`
- Modify: `synthia/src/main.rs`

**Step 1: Create types module with Message types**

Create `synthia/src/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

**Step 2: Add module to main.rs**

Modify `synthia/src/main.rs`:

```rust
mod types;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Synthia starting...");
    Ok(())
}
```

**Step 3: Verify it compiles**

```bash
cargo build
```

Expected: BUILD SUCCESS

**Step 4: Write basic test for Message types**

Add to bottom of `synthia/src/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content.len(), 1);
    }
}
```

**Step 5: Run tests**

```bash
cargo test
```

Expected: 1 test passed

**Step 6: Commit**

```bash
git add synthia/src/
git commit -m "feat: add core message types"
```

---

### Task 3: Tool Trait and Registry

**Agent:** tool-implementer
**Files:**
- Create: `synthia/src/tools/mod.rs`
- Create: `synthia/src/tools/registry.rs`
- Modify: `synthia/src/main.rs`

**Step 1: Write test for Tool trait**

Create `synthia/src/tools/mod.rs`:

```rust
pub mod registry;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, params: Value) -> Result<ToolResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock"
        }
        fn description(&self) -> &str {
            "A mock tool"
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({})
        }
        async fn execute(&self, _params: Value) -> Result<ToolResult> {
            Ok(ToolResult {
                content: "mock result".to_string(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_tool_trait() {
        let tool = MockTool;
        assert_eq!(tool.name(), "mock");
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(!result.is_error);
    }
}
```

**Step 2: Run test**

```bash
cargo test test_tool_trait
```

Expected: PASS

**Step 3: Implement ToolRegistry**

Create `synthia/src/tools/registry.rs`:

```rust
use super::{Tool, ToolResult};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub async fn execute(&self, name: &str, params: Value) -> Result<ToolResult> {
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;
        tool.execute(params).await
    }

    pub fn definitions(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.parameters_schema(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test"
        }
        fn description(&self) -> &str {
            "Test tool"
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({})
        }
        async fn execute(&self, _params: Value) -> Result<ToolResult> {
            Ok(ToolResult {
                content: "executed".to_string(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool));

        let result = registry
            .execute("test", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result.content, "executed");
    }

    #[tokio::test]
    async fn test_registry_missing_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("missing", serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
```

**Step 4: Add module to main.rs**

Modify `synthia/src/main.rs`:

```rust
mod tools;
mod types;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Synthia starting...");
    Ok(())
}
```

**Step 5: Run tests**

```bash
cargo test
```

Expected: All tests pass

**Step 6: Commit**

```bash
git add synthia/src/
git commit -m "feat: add Tool trait and registry"
```

---

### Task 4: Bash Tool Implementation

**Agent:** tool-implementer
**Files:**
- Create: `synthia/src/tools/bash.rs`
- Modify: `synthia/src/tools/mod.rs`

**Step 1: Write test for BashTool**

Create `synthia/src/tools/bash.rs`:

```rust
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
        "Execute a bash command and return stdout/stderr"
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
        let command = params["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        let result = timeout(
            Duration::from_secs(self.timeout_seconds),
            Command::new("bash").arg("-c").arg(command).output(),
        )
        .await??;

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
```

**Step 2: Add module to tools/mod.rs**

Modify `synthia/src/tools/mod.rs`:

```rust
pub mod bash;
pub mod registry;

// ... rest of file
```

**Step 3: Run tests**

```bash
cargo test bash
```

Expected: 3 tests pass

**Step 4: Commit**

```bash
git add synthia/src/tools/
git commit -m "feat: implement Bash tool with timeout"
```

---

### Task 5: Read, Write, Edit Tools

**Agent:** tool-implementer
**Files:**
- Create: `synthia/src/tools/read.rs`
- Create: `synthia/src/tools/write.rs`
- Create: `synthia/src/tools/edit.rs`
- Modify: `synthia/src/tools/mod.rs`

**Step 1: Implement ReadTool**

Create `synthia/src/tools/read.rs`:

```rust
use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use tokio::fs;

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
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

        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult {
                content: format!("File not found: {}", file_path),
                is_error: true,
            });
        }

        let content = fs::read_to_string(path).await?;

        Ok(ToolResult {
            content,
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

        let tool = ReadTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": temp_path
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content, "test content");

        // Cleanup
        fs::remove_file(temp_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_read_missing_file() {
        let tool = ReadTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": "/tmp/nonexistent_file.txt"
            }))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(result.content.contains("not found"));
    }
}
```

**Step 2: Implement WriteTool**

Create `synthia/src/tools/write.rs`:

```rust
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

        let path = Path::new(file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, content).await?;

        Ok(ToolResult {
            content: format!("Successfully wrote to {}", file_path),
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
}
```

**Step 3: Implement EditTool**

Create `synthia/src/tools/edit.rs`:

```rust
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

        let content = fs::read_to_string(file_path).await?;

        if !content.contains(old_string) {
            return Ok(ToolResult {
                content: format!("String '{}' not found in file", old_string),
                is_error: true,
            });
        }

        let new_content = content.replace(old_string, new_string);
        fs::write(file_path, new_content).await?;

        Ok(ToolResult {
            content: format!("Successfully edited {}", file_path),
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
}
```

**Step 4: Add modules to tools/mod.rs**

Modify `synthia/src/tools/mod.rs`:

```rust
pub mod bash;
pub mod edit;
pub mod read;
pub mod registry;
pub mod write;

// ... rest
```

**Step 5: Run tests**

```bash
cargo test tools
```

Expected: All tool tests pass

**Step 6: Commit**

```bash
git add synthia/src/tools/
git commit -m "feat: implement Read, Write, Edit tools"
```

---

### Task 6: LLM Provider Trait and OpenAI Implementation

**Agent:** llm-client
**Files:**
- Create: `synthia/src/llm/mod.rs`
- Create: `synthia/src/llm/provider.rs`
- Create: `synthia/src/llm/openai.rs`
- Modify: `synthia/src/main.rs`

**Step 1: Define LLMProvider trait**

Create `synthia/src/llm/mod.rs`:

```rust
pub mod openai;
pub mod provider;

pub use provider::{GenerationConfig, LLMProvider, LLMResponse};
```

Create `synthia/src/llm/provider.rs`:

```rust
use crate::types::{ContentBlock, Message, StopReason, TokenUsage};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Vec<Value>,
        config: &GenerationConfig,
    ) -> Result<LLMResponse>;
}
```

**Step 2: Implement OpenAI-compatible provider skeleton**

Create `synthia/src/llm/openai.rs`:

```rust
use super::provider::{GenerationConfig, LLMProvider, LLMResponse};
use crate::types::{ContentBlock, Message, Role, StopReason, TokenUsage};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

pub struct OpenAICompatibleProvider {
    client: Client,
    api_base: String,
    api_key: Option<String>,
}

impl OpenAICompatibleProvider {
    pub fn new(api_base: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_base,
            api_key,
        }
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<Value> {
        messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                };

                let content: Vec<Value> = msg
                    .content
                    .into_iter()
                    .map(|block| match block {
                        ContentBlock::Text { text } => {
                            json!({ "type": "text", "text": text })
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            json!({
                                "type": "tool_use",
                                "id": id,
                                "name": name,
                                "input": input
                            })
                        }
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => {
                            json!({
                                "type": "tool_result",
                                "tool_use_id": tool_use_id,
                                "content": content,
                                "is_error": is_error
                            })
                        }
                    })
                    .collect();

                json!({ "role": role, "content": content })
            })
            .collect()
    }
}

#[async_trait]
impl LLMProvider for OpenAICompatibleProvider {
    async fn chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Vec<Value>,
        config: &GenerationConfig,
    ) -> Result<LLMResponse> {
        let url = format!("{}/chat/completions", self.api_base);

        let mut request_body = json!({
            "model": config.model,
            "messages": self.convert_messages(messages),
            "temperature": config.temperature,
        });

        if let Some(max_tokens) = config.max_tokens {
            request_body["max_tokens"] = json!(max_tokens);
        }

        if !tools.is_empty() {
            request_body["tools"] = json!(tools);
        }

        let mut req = self.client.post(&url).json(&request_body);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send().await?;
        let response_json: Value = response.json().await?;

        // Parse response (simplified for now)
        let choice = &response_json["choices"][0];
        let message = &choice["message"];

        let content = if let Some(text) = message["content"].as_str() {
            vec![ContentBlock::Text {
                text: text.to_string(),
            }]
        } else {
            vec![]
        };

        let stop_reason = match choice["finish_reason"].as_str() {
            Some("stop") => StopReason::EndTurn,
            Some("length") => StopReason::MaxTokens,
            _ => StopReason::EndTurn,
        };

        let usage = TokenUsage {
            input_tokens: response_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: response_json["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            stop_reason,
            usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;

    #[test]
    fn test_convert_messages() {
        let provider = OpenAICompatibleProvider::new(
            "http://localhost:1234/v1".to_string(),
            None,
        );

        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        }];

        let converted = provider.convert_messages(messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["role"], "user");
    }
}
```

**Step 3: Add module to main.rs**

Modify `synthia/src/main.rs`:

```rust
mod llm;
mod tools;
mod types;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Synthia starting...");
    Ok(())
}
```

**Step 4: Build and test**

```bash
cargo test llm
```

Expected: Tests pass

**Step 5: Commit**

```bash
git add synthia/src/
git commit -m "feat: add LLM provider trait and OpenAI implementation"
```

---

### Task 7: Basic Agent Loop

**Agent:** agent-loop
**Files:**
- Create: `synthia/src/agent/mod.rs`
- Create: `synthia/src/agent/actor.rs`
- Create: `synthia/src/agent/messages.rs`
- Modify: `synthia/src/main.rs`

**Step 1: Define agent messages**

Create `synthia/src/agent/mod.rs`:

```rust
pub mod actor;
pub mod messages;

pub use actor::AgentActor;
pub use messages::{Command, UIUpdate};
```

Create `synthia/src/agent/messages.rs`:

```rust
use crate::types::Message;

#[derive(Debug, Clone)]
pub enum Command {
    SendMessage(String),
    Cancel,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum UIUpdate {
    AssistantText(String),
    ToolExecutionStarted { name: String, id: String },
    ToolExecutionCompleted { name: String, id: String, duration_ms: u64 },
    Error(String),
    Complete,
}
```

**Step 2: Implement basic AgentActor**

Create `synthia/src/agent/actor.rs`:

```rust
use super::messages::{Command, UIUpdate};
use crate::llm::{GenerationConfig, LLMProvider, LLMResponse};
use crate::tools::registry::ToolRegistry;
use crate::types::{ContentBlock, Message, Role, StopReason};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct AgentActor {
    llm_provider: Arc<dyn LLMProvider>,
    tool_registry: Arc<ToolRegistry>,
    conversation: Vec<Message>,
    config: GenerationConfig,
    ui_tx: Sender<UIUpdate>,
    cmd_rx: Receiver<Command>,
}

impl AgentActor {
    pub fn new(
        llm_provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: GenerationConfig,
        ui_tx: Sender<UIUpdate>,
        cmd_rx: Receiver<Command>,
    ) -> Self {
        Self {
            llm_provider,
            tool_registry,
            conversation: Vec::new(),
            config,
            ui_tx,
            cmd_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Agent actor starting");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::SendMessage(text) => {
                    self.conversation.push(Message {
                        role: Role::User,
                        content: vec![ContentBlock::Text { text }],
                    });
                    if let Err(e) = self.generate_response().await {
                        self.ui_tx
                            .send(UIUpdate::Error(format!("Agent error: {}", e)))
                            .await?;
                    }
                }
                Command::Cancel => {
                    tracing::info!("Cancellation requested");
                    // TODO: Implement cancellation
                }
                Command::Shutdown => {
                    tracing::info!("Shutdown requested");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn generate_response(&mut self) -> Result<()> {
        loop {
            let response = self
                .llm_provider
                .chat_completion(
                    self.conversation.clone(),
                    self.tool_registry.definitions(),
                    &self.config,
                )
                .await?;

            // Process response content
            for block in &response.content {
                match block {
                    ContentBlock::Text { text } => {
                        self.ui_tx.send(UIUpdate::AssistantText(text.clone())).await?;
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        self.ui_tx
                            .send(UIUpdate::ToolExecutionStarted {
                                name: name.clone(),
                                id: id.clone(),
                            })
                            .await?;

                        let start = std::time::Instant::now();
                        let result = self.tool_registry.execute(name, input.clone()).await?;
                        let duration_ms = start.elapsed().as_millis() as u64;

                        self.ui_tx
                            .send(UIUpdate::ToolExecutionCompleted {
                                name: name.clone(),
                                id: id.clone(),
                                duration_ms,
                            })
                            .await?;

                        // Add tool result to conversation
                        self.conversation.push(Message {
                            role: Role::Assistant,
                            content: vec![block.clone()],
                        });
                        self.conversation.push(Message {
                            role: Role::User,
                            content: vec![ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: result.content,
                                is_error: result.is_error,
                            }],
                        });
                    }
                    _ => {}
                }
            }

            // Add assistant message to conversation
            if !response.content.is_empty() {
                let text_blocks: Vec<_> = response
                    .content
                    .iter()
                    .filter(|b| matches!(b, ContentBlock::Text { .. }))
                    .cloned()
                    .collect();
                if !text_blocks.is_empty() {
                    self.conversation.push(Message {
                        role: Role::Assistant,
                        content: text_blocks,
                    });
                }
            }

            // Check stop reason
            if matches!(response.stop_reason, StopReason::EndTurn) {
                self.ui_tx.send(UIUpdate::Complete).await?;
                break;
            }
        }

        Ok(())
    }
}
```

**Step 3: Add module to main.rs**

Modify `synthia/src/main.rs`:

```rust
mod agent;
mod llm;
mod tools;
mod types;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Synthia starting...");
    Ok(())
}
```

**Step 4: Build**

```bash
cargo build
```

Expected: BUILD SUCCESS

**Step 5: Commit**

```bash
git add synthia/src/
git commit -m "feat: implement basic agent loop actor"
```

---

### Task 8: Basic TUI

**Agent:** tui
**Files:**
- Create: `synthia/src/ui/mod.rs`
- Create: `synthia/src/ui/app.rs`
- Modify: `synthia/src/main.rs`

**Step 1: Create basic TUI app structure**

Create `synthia/src/ui/mod.rs`:

```rust
pub mod app;

pub use app::App;
```

Create `synthia/src/ui/app.rs`:

```rust
use crate::agent::messages::{Command, UIUpdate};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct App {
    conversation: Vec<String>,
    input: String,
    cmd_tx: Sender<Command>,
    ui_rx: Receiver<UIUpdate>,
    should_quit: bool,
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            conversation: Vec::new(),
            input: String::new(),
            cmd_tx,
            ui_rx,
            should_quit: false,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            // Handle UI updates from agent
            while let Ok(update) = self.ui_rx.try_recv() {
                self.handle_ui_update(update);
            }

            // Render
            terminal.draw(|f| self.render(f))?;

            // Handle input
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(key).await?;
                }
            }
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn handle_ui_update(&mut self, update: UIUpdate) {
        match update {
            UIUpdate::AssistantText(text) => {
                self.conversation.push(format!("Assistant: {}", text));
            }
            UIUpdate::ToolExecutionStarted { name, id } => {
                self.conversation
                    .push(format!("[Tool: {}] ⏳ Running...", name));
            }
            UIUpdate::ToolExecutionCompleted {
                name,
                id,
                duration_ms,
            } => {
                if let Some(last) = self.conversation.last_mut() {
                    *last = format!("[Tool: {}] ✓ {}ms", name, duration_ms);
                }
            }
            UIUpdate::Error(err) => {
                self.conversation.push(format!("Error: {}", err));
            }
            UIUpdate::Complete => {
                // Generation complete
            }
        }
    }

    async fn handle_input(
        &mut self,
        key: event::KeyEvent,
    ) -> anyhow::Result<()> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::Cancel).await?;
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.cmd_tx.send(Command::Shutdown).await?;
                self.should_quit = true;
            }
            (KeyCode::Enter, _) => {
                if !self.input.is_empty() {
                    let msg = self.input.clone();
                    self.conversation.push(format!("User: {}", msg));
                    self.cmd_tx.send(Command::SendMessage(msg)).await?;
                    self.input.clear();
                }
            }
            (KeyCode::Char(c), _) => {
                self.input.push(c);
            }
            (KeyCode::Backspace, _) => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn render(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Status bar
        let status = Paragraph::new("Synthia v0.1.0")
            .style(Style::default().bg(Color::Blue).fg(Color::White))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, chunks[0]);

        // Conversation
        let conversation_text: Vec<Line> = self
            .conversation
            .iter()
            .map(|msg| Line::from(msg.as_str()))
            .collect();
        let conversation = Paragraph::new(conversation_text)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: false });
        f.render_widget(conversation, chunks[1]);

        // Input
        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[2]);
    }
}
```

**Step 2: Wire up main.rs**

Modify `synthia/src/main.rs`:

```rust
mod agent;
mod llm;
mod tools;
mod types;
mod ui;

use agent::{messages::Command, messages::UIUpdate, AgentActor};
use anyhow::Result;
use llm::{openai::OpenAICompatibleProvider, GenerationConfig};
use std::sync::Arc;
use tools::{bash::BashTool, read::ReadTool, registry::ToolRegistry, write::WriteTool};
use tokio::sync::mpsc;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Create LLM provider
    let llm_provider = Arc::new(OpenAICompatibleProvider::new(
        "http://localhost:1234/v1".to_string(),
        None,
    ));

    // Create tool registry
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(BashTool::new(120)));
    tool_registry.register(Arc::new(ReadTool::new()));
    tool_registry.register(Arc::new(WriteTool::new()));
    let tool_registry = Arc::new(tool_registry);

    // Create channels
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(100);
    let (ui_tx, ui_rx) = mpsc::channel::<UIUpdate>(100);

    // Create agent actor
    let config = GenerationConfig {
        model: "qwen2.5-coder-7b-instruct".to_string(),
        temperature: 0.7,
        max_tokens: Some(4096),
    };
    let mut agent = AgentActor::new(
        llm_provider,
        tool_registry,
        config,
        ui_tx,
        cmd_rx,
    );

    // Spawn agent actor
    tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            tracing::error!("Agent error: {}", e);
        }
    });

    // Run TUI
    let mut app = App::new(cmd_tx, ui_rx);
    app.run().await?;

    Ok(())
}
```

**Step 3: Build and run (with LM Studio running)**

```bash
cargo build --release
```

Expected: BUILD SUCCESS

**Step 4: Test manually**

```bash
./target/release/synthia
```

Expected: TUI appears, can type messages (if LM Studio is running)

**Step 5: Commit**

```bash
git add synthia/src/
git commit -m "feat: implement basic TUI and wire up complete system"
```

---

## Phase 2: Complete Toolset (Session 3)

### Task 9: Grep and Glob Tools

**Agent:** tool-implementer
**Files:**
- Create: `synthia/src/tools/grep.rs`
- Create: `synthia/src/tools/glob.rs`
- Modify: `synthia/src/tools/mod.rs`
- Modify: `synthia/src/main.rs`

**Implementation details:** Similar to previous tools, using ripgrep and glob crates...

### Task 10: WebFetch Tool

**Agent:** tool-implementer
**Implementation:** Use reqwest to fetch URLs, convert to text...

### Task 11: Git Tools

**Agent:** tool-implementer
**Implementation:** Shell out to git commands or use git2 crate...

### Task 12: Powertools Integration

**Agent:** integration
**Implementation:** Shell out to powertools binary, parse JSON output...

### Task 13: Workshop Integration

**Agent:** integration
**Implementation:** Shell out to workshop CLI, parse output...

---

## Phase 3: Polish & Advanced Features (Session 4)

### Task 14: Markdown Rendering in TUI

**Agent:** tui
**Implementation:** Add ratatui-markdown or custom parser...

### Task 15: Configuration System

**Agent:** integration
**Implementation:** TOML config loading with serde...

### Task 16: Session Persistence

**Agent:** integration
**Implementation:** Save/load conversations to JSON...

### Task 17: Streaming Text Support

**Agent:** llm-client + tui
**Implementation:** Add streaming endpoint support...

---

## Phase 4: Testing & Release (Distributed)

### Task 18: Comprehensive Test Suite

**Agent:** test
**Implementation:** Unit tests for all tools, integration tests for agent loop...

### Task 19: CI Pipeline

**Agent:** test
**Implementation:** GitHub Actions workflow...

### Task 20: Documentation

**Agent:** integration
**Implementation:** README, architecture docs...

---

## Subagent Coordination Notes

- **Architect** runs first to set up structure
- **LLM Client, Tool Implementer, Agent Loop** can run in parallel after architect
- **TUI** needs agent loop messages defined first
- **Test** runs continuously throughout
- **Integration** runs after core tools are implemented

## Success Criteria

- ✅ Can send message to local LLM via LM Studio
- ✅ LLM can call tools (bash, read, write, edit)
- ✅ Tools execute and return results to LLM
- ✅ Conversation displays in TUI
- ✅ Can cancel generation and exit cleanly
- ✅ 80%+ test coverage on critical paths
- ✅ CI passing on all commits
