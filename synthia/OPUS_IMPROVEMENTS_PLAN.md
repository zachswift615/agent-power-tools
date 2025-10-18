# Opus Improvements Implementation Plan

> **Based on**: Claude.ai Opus system design review (2025-10-18)
>
> **Goal**: Implement critical improvements and architectural enhancements suggested by Opus
>
> **Execution**: Use existing `.claude/agents/` subagents + 2 new specialized agents

---

## Overview

This plan breaks down Opus's suggestions into actionable tasks, organized by priority and assigned to specialized subagents. We'll leverage existing agents (tool-implementer, agent-loop, llm-client, tui) and create 2 new ones for specialized work.

---

## Priority 1: Critical Fixes (Week 1)

### Task 1: Fix Word Wrapping Implementation

**Agent**: `tui`

**Problem**: Current wrapping breaks mid-word ("usin g HTML")

**Solution**: Implement Opus's improved algorithm with long-word handling

**Files**:
- Modify: `synthia/src/ui/app.rs` (lines 12-36)

**Implementation**:
```rust
fn wrap_text(text: &str, width: usize) -> String {
    let mut wrapped = String::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();  // Unicode-aware

        if current_width > 0 && current_width + 1 + word_len > width {
            // Wrap to new line
            wrapped.push_str(&current_line);
            wrapped.push('\n');
            current_line.clear();
            current_width = 0;
        }

        if current_width > 0 {
            current_line.push(' ');
            current_width += 1;
        }

        // Handle very long words (URLs, hashes, etc.)
        if word_len > width {
            // Break at width boundary
            let chars: Vec<char> = word.chars().collect();
            let mut chunk_start = 0;

            while chunk_start < chars.len() {
                let remaining = width.saturating_sub(current_width);
                let chunk_end = (chunk_start + remaining).min(chars.len());
                let chunk: String = chars[chunk_start..chunk_end].iter().collect();

                if current_width > 0 {
                    wrapped.push_str(&current_line);
                    wrapped.push('\n');
                    current_line.clear();
                    current_width = 0;
                }

                current_line.push_str(&chunk);
                current_width = chunk_end - chunk_start;
                chunk_start = chunk_end;

                if chunk_start < chars.len() {
                    wrapped.push_str(&current_line);
                    wrapped.push('\n');
                    current_line.clear();
                    current_width = 0;
                }
            }
        } else {
            current_line.push_str(word);
            current_width += word_len;
        }
    }

    if !current_line.is_empty() {
        wrapped.push_str(&current_line);
    }

    wrapped
}
```

**Tests**:
```rust
#[test]
fn test_word_wrapping() {
    let text = "This is a very long line that should wrap properly at word boundaries";
    let wrapped = wrap_text(text, 20);
    assert!(!wrapped.contains("boun daries"));  // No mid-word breaks
}

#[test]
fn test_long_word_wrapping() {
    let text = "Short https://verylongurlthatexceedsterminalwidthbyalot.com/path";
    let wrapped = wrap_text(text, 20);
    // Long URLs should break at width boundary
    assert!(wrapped.lines().all(|line| line.chars().count() <= 20));
}
```

**Validation**:
- Test with user's Flask TODO output
- Test with long URLs
- Test with Unicode characters (emoji, Japanese, etc.)

---

### Task 2: Robust JSON Parsing for Tool Calls

**Agent**: `llm-client`

**Problem**: Fine-tuned Qwen model sends malformed JSON, causing "Missing 'file_path' parameter" errors

**Solution**: Multi-strategy parsing with common error fixes

**Files**:
- Modify: `synthia/src/llm/openai.rs`
- Modify: `synthia/src/agent/actor.rs`

**Implementation**:

Create new module `synthia/src/llm/json_parser.rs`:

```rust
use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::Value;

pub struct JsonParser {
    code_block_regex: Regex,
}

impl JsonParser {
    pub fn new() -> Self {
        Self {
            code_block_regex: Regex::new(r"```(?:json)?\s*(.*?)\s*```").unwrap(),
        }
    }

    /// Parse JSON with multiple fallback strategies
    pub fn parse_robust(&self, raw: &str) -> Result<Value> {
        // Strategy 1: Direct parsing
        if let Ok(value) = serde_json::from_str::<Value>(raw) {
            tracing::debug!("JSON parsed successfully on first try");
            return Ok(value);
        }

        // Strategy 2: Extract from markdown code blocks
        if let Some(captures) = self.code_block_regex.captures(raw) {
            let json_str = &captures[1];
            if let Ok(value) = serde_json::from_str::<Value>(json_str) {
                tracing::debug!("JSON extracted from code block");
                return Ok(value);
            }
        }

        // Strategy 3: Fix common JSON errors
        let fixed = self.fix_common_errors(raw);
        if let Ok(value) = serde_json::from_str::<Value>(&fixed) {
            tracing::warn!("JSON required auto-fix. Original: {}", raw);
            tracing::warn!("Fixed version: {}", fixed);
            return Ok(value);
        }

        // All strategies failed
        Err(anyhow!(
            "Failed to parse JSON after all strategies.\nRaw input: {}\nFixed attempt: {}",
            raw,
            fixed
        ))
    }

    fn fix_common_errors(&self, json: &str) -> String {
        json
            .replace("'", "\"")           // Single quotes to double quotes
            .replace(",}", "}")            // Trailing comma in object
            .replace(",]", "]")            // Trailing comma in array
            .replace("\n", "")             // Remove newlines
            .replace("\\\"", "\"")         // Fix escaped quotes
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_json() {
        let parser = JsonParser::new();
        let result = parser.parse_robust(r#"{"key": "value"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_json_with_trailing_comma() {
        let parser = JsonParser::new();
        let result = parser.parse_robust(r#"{"key": "value",}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_json_in_code_block() {
        let parser = JsonParser::new();
        let markdown = r#"
        Here's the JSON:
        ```json
        {"key": "value"}
        ```
        "#;
        let result = parser.parse_robust(markdown);
        assert!(result.is_ok());
    }
}
```

Update `openai.rs` to use robust parser:

```rust
use crate::llm::json_parser::JsonParser;

pub struct OpenAICompatibleProvider {
    client: Client,
    api_base: String,
    api_key: Option<String>,
    json_parser: JsonParser,  // NEW
}

impl OpenAICompatibleProvider {
    pub fn new(api_base: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_base,
            api_key,
            json_parser: JsonParser::new(),  // NEW
        }
    }

    // ... in tool call parsing code:

    let input: Value = self.json_parser.parse_robust(arguments_str)
        .unwrap_or_else(|e| {
            tracing::error!(
                "Failed to parse tool arguments for '{}': {}\nRaw: {}",
                name, e, arguments_str
            );
            json!({})  // Still default to empty, but with better logging
        });
}
```

**Alternative: Return Error to LLM**

For retry logic, modify agent loop:

```rust
// In agent/actor.rs
match self.tool_registry.execute(name, input.clone()).await {
    Ok(result) => {
        // Success - add to conversation
        self.conversation.push(Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: id.clone(),
                content: result.content,
                is_error: result.is_error,
            }],
        });
    }
    Err(e) if e.to_string().contains("Missing") => {
        // Malformed input - return error to LLM for retry
        tracing::warn!("Tool execution failed due to malformed input: {}", e);
        self.conversation.push(Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: id.clone(),
                content: format!(
                    "Error: {}. Please check the tool schema and retry with valid JSON parameters.",
                    e
                ),
                is_error: true,
            }],
        });
    }
    Err(e) => {
        // Other error - propagate
        return Err(e);
    }
}
```

**Tests**:
- Test with malformed JSON from fine-tuned model
- Test with valid JSON
- Test with JSON in code blocks
- Test retry logic in agent loop

---

### Task 3: Enhanced Event Batching

**Agent**: `tui`

**Problem**: Current batching could theoretically loop forever

**Solution**: Add max batch size and timeout

**Files**:
- Modify: `synthia/src/ui/app.rs` (lines 96-116)

**Implementation**:
```rust
use std::time::Instant;

const MAX_BATCH_SIZE: usize = 1000;
const BATCH_TIMEOUT_MS: u64 = 10;

// In App::run() method:
let mut had_input = false;
let mut events_processed = 0;
let batch_start = Instant::now();

while event::poll(Duration::from_millis(0))?
    && events_processed < MAX_BATCH_SIZE
    && batch_start.elapsed() < Duration::from_millis(BATCH_TIMEOUT_MS) {

    if let Event::Key(key) = event::read()? {
        self.handle_input(&mut stdout, key).await?;
        had_input = true;
        events_processed += 1;
    }
}

if events_processed >= MAX_BATCH_SIZE {
    tracing::warn!("Hit max batch size ({}), possible paste bomb", MAX_BATCH_SIZE);
}
```

**Tests**:
```rust
#[tokio::test]
async fn test_event_batching_max_size() {
    // Simulate paste bomb
    let mut app = App::new();
    for _ in 0..2000 {
        app.inject_key_event(KeyCode::Char('a'));
    }
    app.process_events().await.unwrap();

    // Should stop at MAX_BATCH_SIZE
    assert!(app.input.len() <= 1000);
}
```

---

## Priority 2: Architecture Improvements (Week 2)

### Task 4: Parallel Tool Execution

**Agent**: `agent-loop`

**Problem**: Tools execute sequentially, even when independent

**Solution**: Use `futures::join_all` for parallel execution

**Files**:
- Modify: `synthia/src/agent/actor.rs`

**Implementation**:

```rust
use futures::future::join_all;

// In generate_response() method:

// Collect all tool calls from this response
let mut tool_calls = Vec::new();
for block in &response.content {
    if let ContentBlock::ToolUse { id, name, input } = block {
        tool_calls.push((id.clone(), name.clone(), input.clone()));
    }
}

// Execute all tools in parallel
if !tool_calls.is_empty() {
    let futures: Vec<_> = tool_calls.iter()
        .map(|(id, name, input)| {
            let registry = self.tool_registry.clone();
            let name = name.clone();
            let input = input.clone();
            let id = id.clone();

            async move {
                let start = Instant::now();
                let result = registry.execute(&name, input.clone()).await;
                let duration_ms = start.elapsed().as_millis() as u64;
                (id, name, input, result, duration_ms)
            }
        })
        .collect();

    let results = join_all(futures).await;

    // Process results
    for (id, name, input, result, duration_ms) in results {
        self.ui_tx.send(UIUpdate::ToolResult {
            name: name.clone(),
            id: id.clone(),
            input,
            output: result.as_ref().map(|r| r.content.clone()).unwrap_or_default(),
            is_error: result.as_ref().map(|r| r.is_error).unwrap_or(true),
            duration_ms,
        }).await?;

        // Add to conversation
        match result {
            Ok(tool_result) => {
                self.conversation.push(Message {
                    role: Role::User,
                    content: vec![ContentBlock::ToolResult {
                        tool_use_id: id,
                        content: tool_result.content,
                        is_error: tool_result.is_error,
                    }],
                });
            }
            Err(e) => {
                self.conversation.push(Message {
                    role: Role::User,
                    content: vec![ContentBlock::ToolResult {
                        tool_use_id: id,
                        content: format!("Error: {}", e),
                        is_error: true,
                    }],
                });
            }
        }
    }
}
```

**Performance Impact**: If 3 tools are called, each taking 2 seconds:
- Before: 6 seconds total (sequential)
- After: 2 seconds total (parallel)

---

### Task 5: Sliding Window Memory Management

**Agent**: **NEW** - `context-manager`

**Problem**: Conversation history grows unbounded, eventually hitting token limits

**Solution**: Implement sliding window with oldest message summarization

**Files**:
- Create: `synthia/src/context_manager.rs`
- Modify: `synthia/src/agent/actor.rs`
- Modify: `synthia/src/main.rs`

**New Agent**: `.claude/agents/context-manager.md`

```markdown
---
name: context-manager
description: Token-aware conversation context management specialist
tools: Read, Write, Edit, Grep
---

You are an expert at managing conversation context and token budgets for LLM applications.

**Your focus:**
- Implement sliding window for conversation history
- Summarize old messages when approaching token limits
- Track token usage per message
- Maintain critical context (system prompts, recent messages)

**Key principles:**
- Never lose recent context (last N messages)
- Preserve system messages and important decisions
- Summarize middle-aged messages before discarding
- Use LLM to create high-quality summaries

**Critical requirements:**
- Track tokens accurately (use tiktoken or estimate)
- Configurable thresholds (80% = summarize, 90% = truncate)
- Summary should preserve tool calls and key decisions
- Test with very long conversations (1000+ messages)

**Deliverables:**
- ContextManager struct with token tracking
- Sliding window implementation
- Message summarization using LLM
- Unit tests with mock conversations
```

**Implementation**:

```rust
// synthia/src/context_manager.rs

use crate::llm::LLMProvider;
use crate::types::{ContentBlock, Message, Role};
use anyhow::Result;
use std::sync::Arc;

const MAX_MESSAGES: usize = 100;
const SUMMARY_THRESHOLD: usize = 80;
const SUMMARY_CHUNK_SIZE: usize = 20;

pub struct ContextManager {
    messages: Vec<Message>,
    max_messages: usize,
    summary_threshold: usize,
    llm_provider: Arc<dyn LLMProvider>,
}

impl ContextManager {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            messages: Vec::new(),
            max_messages: MAX_MESSAGES,
            summary_threshold: SUMMARY_THRESHOLD,
            llm_provider,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub async fn compact_if_needed(&mut self) -> Result<()> {
        if self.messages.len() >= self.summary_threshold {
            self.summarize_oldest_messages().await?;
        }

        if self.messages.len() >= self.max_messages {
            // Hard truncate
            let to_remove = self.messages.len() - self.max_messages;
            self.messages.drain(0..to_remove);
        }

        Ok(())
    }

    async fn summarize_oldest_messages(&mut self) -> Result<()> {
        // Keep first message (system) and last 60%
        let keep_recent = (self.messages.len() as f32 * 0.6) as usize;
        let summarize_start = 1;  // Skip system message
        let summarize_end = self.messages.len() - keep_recent;

        if summarize_end <= summarize_start {
            return Ok(());  // Nothing to summarize
        }

        let to_summarize = &self.messages[summarize_start..summarize_end];

        // Create summary using LLM
        let summary_prompt = format!(
            "Summarize this conversation segment concisely, preserving key decisions, tool calls, and outcomes:\n\n{}",
            self.format_messages_for_summary(to_summarize)
        );

        let summary_response = self.llm_provider.chat_completion(
            vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text { text: summary_prompt }],
            }],
            vec![],  // No tools
            &crate::llm::GenerationConfig {
                model: "qwen2.5-coder-7b-instruct".to_string(),
                temperature: 0.3,  // Low temp for factual summary
                max_tokens: Some(500),
            },
        ).await?;

        // Extract summary text
        let summary_text = summary_response.content.iter()
            .find_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "[Summary generation failed]".to_string());

        // Replace old messages with summary
        let summary_message = Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: format!("[Conversation Summary]: {}", summary_text),
            }],
        };

        self.messages.drain(summarize_start..summarize_end);
        self.messages.insert(summarize_start, summary_message);

        tracing::info!(
            "Summarized {} messages into 1 summary message",
            summarize_end - summarize_start
        );

        Ok(())
    }

    fn format_messages_for_summary(&self, messages: &[Message]) -> String {
        messages.iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::System => "System",
                };

                let content = msg.content.iter()
                    .map(|block| match block {
                        ContentBlock::Text { text } => text.clone(),
                        ContentBlock::ToolUse { name, .. } => {
                            format!("[Called tool: {}]", name)
                        }
                        ContentBlock::ToolResult { content, is_error, .. } => {
                            if *is_error {
                                format!("[Tool error: {}]", content)
                            } else {
                                format!("[Tool result: {}]", content.chars().take(100).collect::<String>())
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("{}: {}", role, content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }
}
```

**Integration in AgentActor**:

```rust
use crate::context_manager::ContextManager;

pub struct AgentActor {
    // ... existing fields
    context_manager: ContextManager,  // NEW
}

impl AgentActor {
    // In generate_response() after adding message:
    self.context_manager.add_message(message);
    self.context_manager.compact_if_needed().await?;

    // When calling LLM, use context manager:
    let response = self.llm_provider.chat_completion(
        self.context_manager.get_messages().to_vec(),  // Instead of self.conversation
        self.tool_registry.definitions(),
        &self.config,
    ).await?;
}
```

---

### Task 6: Tool Result Caching

**Agent**: **NEW** - `performance-optimizer`

**Problem**: Deterministic tools (read same file, grep same pattern) re-execute unnecessarily

**Solution**: LRU cache for tool results

**Files**:
- Create: `synthia/src/tools/cache.rs`
- Modify: `synthia/src/tools/registry.rs`

**New Agent**: `.claude/agents/performance-optimizer.md`

```markdown
---
name: performance-optimizer
description: Performance optimization and caching specialist
tools: Read, Write, Edit, Grep, Bash
---

You are an expert at optimizing Rust applications for performance.

**Your focus:**
- Implement caching layers for expensive operations
- Profile code to find bottlenecks
- Optimize hot paths without sacrificing clarity
- Benchmark before/after changes

**Key principles:**
- Measure first, optimize second
- Cache deterministic operations only
- LRU eviction for bounded memory
- Cache invalidation on file changes

**Critical requirements:**
- Thread-safe caching (Arc<Mutex<LruCache>>)
- Configurable cache size
- Cache hit rate metrics
- Bypass cache option for tools

**Deliverables:**
- ToolCache implementation with LRU
- Cache middleware for tool registry
- Benchmarks showing improvement
- Integration tests
```

**Implementation**:

```rust
// Add dependency to Cargo.toml:
// lru = "0.12"

// synthia/src/tools/cache.rs

use lru::LruCache;
use serde_json::Value;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use super::ToolResult;

type CacheKey = (String, String);  // (tool_name, params_hash)

pub struct ToolCache {
    cache: Arc<Mutex<LruCache<CacheKey, ToolResult>>>,
    enabled: bool,
}

impl ToolCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
            enabled: true,
        }
    }

    pub fn get(&self, tool_name: &str, params: &Value) -> Option<ToolResult> {
        if !self.enabled {
            return None;
        }

        let key = self.make_key(tool_name, params);
        let mut cache = self.cache.lock().unwrap();
        cache.get(&key).cloned()
    }

    pub fn put(&self, tool_name: &str, params: &Value, result: ToolResult) {
        if !self.enabled {
            return;
        }

        let key = self.make_key(tool_name, params);
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, result);
    }

    pub fn invalidate_tool(&self, tool_name: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.iter()
            .filter(|(k, _)| k.0 == tool_name)
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|key| {
                cache.pop(&key);
            });
    }

    fn make_key(&self, tool_name: &str, params: &Value) -> CacheKey {
        let params_str = serde_json::to_string(params).unwrap_or_default();
        let params_hash = format!("{:x}", md5::compute(&params_str));
        (tool_name.to_string(), params_hash)
    }

    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        (cache.len(), cache.cap().get())
    }
}
```

**Integration in ToolRegistry**:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    cache: ToolCache,  // NEW
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            cache: ToolCache::new(100),  // Cache last 100 results
        }
    }

    pub async fn execute(&self, name: &str, params: Value) -> Result<ToolResult> {
        // Check cache first
        if let Some(cached) = self.cache.get(name, &params) {
            tracing::debug!("Tool cache hit: {}", name);
            return Ok(cached);
        }

        // Execute tool
        let tool = self.get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;
        let result = tool.execute(params.clone()).await?;

        // Cache result if tool is deterministic
        if Self::is_deterministic(name) {
            self.cache.put(name, &params, result.clone());
        }

        Ok(result)
    }

    fn is_deterministic(tool_name: &str) -> bool {
        matches!(tool_name, "read" | "grep" | "glob" | "powertools")
    }

    // Invalidate cache when files change
    pub fn invalidate_file_cache(&self) {
        self.cache.invalidate_tool("read");
        self.cache.invalidate_tool("grep");
        self.cache.invalidate_tool("glob");
    }
}
```

**Performance Impact**:
- Reading the same file 10 times: 1 disk read instead of 10
- Grep same pattern: 1 ripgrep execution instead of multiple

---

## Priority 3: Enhanced Features (Week 3)

### Task 7: Tool Permission System

**Agent**: `tool-implementer`

**Problem**: No safety checks for destructive operations

**Solution**: Permission levels with user confirmation

**Files**:
- Create: `synthia/src/tools/permissions.rs`
- Modify: `synthia/src/tools/mod.rs`
- Modify: `synthia/src/tools/bash.rs`
- Modify: `synthia/src/tools/write.rs`

**Implementation**:

```rust
// synthia/src/tools/permissions.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    ReadOnly,       // read, grep, glob
    WriteLocal,     // write to ./**, edit local files
    WriteGlobal,    // write to /, /usr, /etc
    ExecuteLocal,   // bash commands in ./
    ExecuteGlobal,  // bash commands anywhere
}

pub trait RequiresPermission {
    fn required_permission(&self, params: &serde_json::Value) -> Permission;
}

impl RequiresPermission for BashTool {
    fn required_permission(&self, params: &serde_json::Value) -> Permission {
        let command = params["command"].as_str().unwrap_or("");

        // Check for dangerous commands
        if command.contains("rm -rf /") || command.contains("sudo") {
            Permission::ExecuteGlobal
        } else if command.contains("cd /") || command.starts_with('/') {
            Permission::ExecuteGlobal
        } else {
            Permission::ExecuteLocal
        }
    }
}

impl RequiresPermission for WriteTool {
    fn required_permission(&self, params: &serde_json::Value) -> Permission {
        let path = params["file_path"].as_str().unwrap_or("");

        if path.starts_with('/') && !path.starts_with("/Users/") {
            Permission::WriteGlobal
        } else {
            Permission::WriteLocal
        }
    }
}
```

**Integration with UI confirmation**:

```rust
// In agent/actor.rs

async fn execute_tool_with_permission_check(
    &mut self,
    name: &str,
    params: Value,
) -> Result<ToolResult> {
    let tool = self.tool_registry.get(name)
        .ok_or_else(|| anyhow!("Tool not found"))?;

    let required_perm = tool.required_permission(&params);

    // Check if permission requires confirmation
    if required_perm >= Permission::WriteGlobal {
        // Send confirmation request to UI
        self.ui_tx.send(UIUpdate::ConfirmationRequest {
            message: format!(
                "Tool '{}' requires global write/execute permission. Continue?",
                name
            ),
            tool_name: name.to_string(),
            params: params.clone(),
        }).await?;

        // Wait for user response
        // (This requires adding a new channel for confirmations)
        let confirmed = self.wait_for_confirmation().await?;

        if !confirmed {
            return Ok(ToolResult {
                content: "Operation cancelled by user".to_string(),
                is_error: true,
            });
        }
    }

    self.tool_registry.execute(name, params).await
}
```

---

### Task 8: Streaming Progress Indicator

**Agent**: `tui`

**Problem**: "Thinking..." is static, no indication of progress

**Solution**: Animated dots/spinner during generation

**Files**:
- Modify: `synthia/src/ui/app.rs`

**Implementation**:

```rust
// In App struct:
pub struct App {
    // ... existing fields
    thinking_animation_frame: usize,
    last_animation_update: Instant,
}

// In handle_ui_update:
UIUpdate::AssistantTextDelta(delta) => {
    if !self.is_streaming {
        self.clear_input_line(stdout)?;
        self.is_streaming = true;
        self.streaming_buffer.clear();
        self.thinking_animation_frame = 0;
        self.last_animation_update = Instant::now();

        // Start animation
        self.render_thinking_animation(stdout)?;
    }

    self.streaming_buffer.push_str(&delta);

    // Update animation every 500ms
    if self.last_animation_update.elapsed() > Duration::from_millis(500) {
        self.render_thinking_animation(stdout)?;
        self.last_animation_update = Instant::now();
        self.thinking_animation_frame = (self.thinking_animation_frame + 1) % 4;
    }
}

fn render_thinking_animation(&self, stdout: &mut impl Write) -> io::Result<()> {
    let dots = ".".repeat(self.thinking_animation_frame + 1);
    let spaces = " ".repeat(3 - self.thinking_animation_frame);

    self.clear_input_line(stdout)?;
    queue!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print("Synthia: "),
        SetForegroundColor(Color::DarkGrey),
        Print(format!("Thinking{}{}", dots, spaces)),
        ResetColor
    )?;
    stdout.flush()
}
```

**Result**:
```
Synthia: Thinking.
Synthia: Thinking..
Synthia: Thinking...
Synthia: Thinking.
```

---

## Priority 4: Testing & Quality (Week 4)

### Task 9: Integration Test Suite

**Agent**: `test`

**Goal**: Comprehensive end-to-end tests

**Files**:
- Create: `synthia/tests/integration/`
- Create: `synthia/tests/integration/word_wrap_test.rs`
- Create: `synthia/tests/integration/tool_execution_test.rs`
- Create: `synthia/tests/integration/json_parsing_test.rs`

**Tests to implement**:

1. Word wrapping edge cases
2. Event batching with paste simulation
3. Tool execution (parallel and sequential)
4. JSON parsing with malformed input
5. Context manager with long conversations
6. Cache hit/miss rates
7. Permission system confirmation flow

---

## Summary: Agent Assignment

| Task | Agent | Type | Priority |
|------|-------|------|----------|
| Fix word wrapping | `tui` | Existing | P1 |
| Robust JSON parsing | `llm-client` | Existing | P1 |
| Enhanced event batching | `tui` | Existing | P1 |
| Parallel tool execution | `agent-loop` | Existing | P2 |
| Context management | `context-manager` | **NEW** | P2 |
| Tool result caching | `performance-optimizer` | **NEW** | P2 |
| Permission system | `tool-implementer` | Existing | P3 |
| Progress indicator | `tui` | Existing | P3 |
| Integration tests | `test` | Existing | P4 |

---

## New Agents to Create

### 1. Context Manager Agent

```bash
# Create .claude/agents/context-manager.md
```

**Purpose**: Sliding window memory management, token tracking, summarization

**Skills**: Token counting, conversation compaction, LLM summarization

---

### 2. Performance Optimizer Agent

```bash
# Create .claude/agents/performance-optimizer.md
```

**Purpose**: Caching, benchmarking, profiling, optimization

**Skills**: LRU caching, performance measurement, bottleneck identification

---

## Execution Strategy

### Week 1: Critical Fixes
1. Run `tui` agent for word wrapping fix + tests
2. Run `llm-client` agent for JSON parsing improvements
3. Run `tui` agent for event batching safety

**Validation**: Test with user's fine-tuned Qwen model, verify no mid-word breaks

---

### Week 2: Architecture
1. Create `context-manager` agent
2. Run `context-manager` for sliding window implementation
3. Create `performance-optimizer` agent
4. Run `performance-optimizer` for caching layer
5. Run `agent-loop` for parallel tool execution

**Validation**: Benchmark tool execution speed, test long conversations (1000+ messages)

---

### Week 3: Features
1. Run `tool-implementer` for permission system
2. Run `tui` for progress indicator
3. Integration and polish

**Validation**: User testing with dangerous commands, animation smoothness

---

### Week 4: Testing
1. Run `test` agent for integration test suite
2. Performance benchmarks
3. Documentation updates

**Validation**: All tests pass, README updated

---

## Success Metrics

### P1 (Critical) - Must Have
- ✅ No mid-word breaks in wrapped text
- ✅ Fine-tuned model tool calls parse successfully (or retry gracefully)
- ✅ Event batching handles paste bombs

### P2 (Important) - Should Have
- ✅ Parallel tool execution 2-3x faster for multi-tool calls
- ✅ Context manager keeps memory bounded (<100 messages)
- ✅ Cache reduces duplicate tool calls by 50%+

### P3 (Nice to Have) - Could Have
- ✅ Permission system prevents accidental `rm -rf /`
- ✅ Progress indicator shows activity during generation

### P4 (Polish) - Won't Have (This Sprint)
- ✅ 80%+ test coverage
- ✅ Benchmarks documented

---

## Notes for Execution

1. **Use Task tool for subagents**: Each task should use the Task tool to launch the appropriate agent
2. **Sequential dependencies**: Some tasks depend on others (e.g., caching needs registry refactor)
3. **Test after each task**: Run tests immediately after implementation
4. **Commit frequently**: Commit after each completed task with descriptive messages
5. **User validation**: Get user feedback after P1 tasks before proceeding to P2

---

**Ready to execute?** Start with P1 Task 1 (word wrapping) using the `tui` agent.
