# Synthia Architecture Overview

> **Purpose**: System design review document for external consultation with Claude.ai Opus
>
> **Last Updated**: 2025-10-18
>
> **Current State**: Terminal-native UI refactor complete, addressing word-wrapping issues

---

## Project Vision

**Synthia** is a Claude Code clone for local LLMs. It brings Claude Code's powerful agentic capabilities to local models (via LM Studio, Ollama, etc.) with a clean terminal-native interface and comprehensive tooling.

**Core Goals**:
1. **Local-first**: Run entirely on local LLMs (no cloud dependency)
2. **Rich tooling**: 10+ integrated tools for code navigation, file operations, and more
3. **Native UX**: Terminal-native scrolling, text selection, and interaction (like Claude Code)
4. **Fast & lightweight**: Rust performance with minimal overhead
5. **Session persistence**: Auto-save conversations for continuity

---

## High-Level Architecture

### Actor Model Design

Synthia uses an **actor-based architecture** with tokio channels for message passing between isolated components:

```
┌─────────────────────────────────────────────────────────────┐
│                         Terminal                            │
│  (Raw mode, native scrolling, text selection via OS)       │
└──────────────────▲──────────────┬───────────────────────────┘
                   │              │
                   │ UIUpdate     │ Command
                   │              │
              ┌────┴──────────────▼─────┐
              │                         │
              │      UI Actor (App)     │
              │   - Input handling      │
              │   - Output rendering    │
              │   - Session list UI     │
              │                         │
              └─────────────────────────┘
                   ▲              │
                   │ UIUpdate     │ Command
                   │              │
              ┌────┴──────────────▼─────┐
              │                         │
              │    Agent Actor          │
              │   - LLM orchestration   │
              │   - Tool execution loop │
              │   - Conversation state  │
              │                         │
              └─────────┬───────────────┘
                        │
                        ├─────────► LLM Provider (OpenAI-compatible)
                        │            - HTTP client (reqwest)
                        │            - Streaming support
                        │            - Tool schema generation
                        │
                        └─────────► Tool Registry
                                     - Dynamic tool loading
                                     - Parallel tool execution
                                     - Timeout management
```

**Key Components**:

1. **UI Actor** (`src/ui/app.rs`)
   - Runs in main thread
   - Processes keyboard events in batches (prevents paste spam)
   - Renders output with word-wrapping
   - Manages session list navigation

2. **Agent Actor** (`src/agent/actor.rs`)
   - Spawned as tokio task
   - Orchestrates agentic loop: Think → Act → Observe
   - Maintains conversation history
   - Handles tool execution with timing/error tracking

3. **LLM Provider** (`src/llm/openai.rs`)
   - OpenAI-compatible API client
   - Converts internal message format to OpenAI format
   - Supports tool calling via function schemas
   - Handles streaming responses

4. **Tool Registry** (`src/tools/registry.rs`)
   - Dynamic tool registration
   - Uniform execution interface
   - Schema generation for LLM function calling

---

## Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Language** | Rust 2021 | Performance, safety, async support |
| **Async Runtime** | Tokio | Actor model, async I/O, timers |
| **Terminal** | Crossterm | Raw mode, cursor control, colors |
| **HTTP Client** | Reqwest | LLM API calls, WebFetch tool |
| **Serialization** | serde + serde_json | Message serialization, config |
| **Error Handling** | anyhow | Ergonomic error propagation |
| **Logging** | tracing + tracing-subscriber | Structured logging |

**Removed Dependencies** (as of 2025-10-18):
- ~~Ratatui~~ - Removed in favor of terminal-native rendering
- ~~Markdown parser~~ - Deferred (raw text output for now)

---

## Message Flow

### 1. User Input Flow

```
User types → Event batching → UI Actor → Command::SendMessage
                                              ↓
                                         Agent Actor
                                              ↓
                                    Add to conversation
                                              ↓
                                         LLM Provider
                                              ↓
                                     Parse tool calls
                                              ↓
                                   Execute tools in loop
                                              ↓
                                   UIUpdate::Complete
                                              ↓
                                    Render wrapped output
```

### 2. Tool Execution Flow

```
LLM returns ToolUse → Agent extracts {id, name, input}
                           ↓
                  UIUpdate::ToolExecutionStarted
                           ↓
                  Tool Registry lookup
                           ↓
                  Execute with timeout
                           ↓
                  Measure duration
                           ↓
                  UIUpdate::ToolResult
                           ↓
            Add ToolResult to conversation
                           ↓
            Continue agentic loop
```

---

## Tool Architecture

### Tool Trait

All tools implement a common trait:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;  // JSON Schema
    async fn execute(&self, params: Value) -> Result<ToolResult>;
}
```

### Available Tools (10 total)

**Core File Operations**:
- `bash` - Execute shell commands with timeout (default: 120s)
- `read` - Read file contents
- `write` - Create/overwrite files
- `edit` - Find-and-replace edits

**Search & Discovery**:
- `grep` - Regex search in files
- `glob` - Pattern-based file finding

**Advanced**:
- `webfetch` - HTTP/HTTPS requests with security validation
- `git` - Git operations (status, diff, log, add, commit, push)
- `powertools` - Semantic code navigation (goto definition, find references)
- `workshop` - Persistent context management (notes, decisions, search)

### Tool Execution Guarantees

- **Timeouts**: All tools have configurable timeouts (prevents hangs)
- **Error isolation**: Tool errors don't crash the agent
- **Structured output**: Tools return `{content: String, is_error: bool}`
- **Timing feedback**: UI shows execution duration for each tool

---

## Terminal-Native UI Design

**Recent Refactor** (2025-10-18): Migrated from Ratatui TUI to terminal-native rendering.

### Why Terminal-Native?

1. **Natural scrolling**: Mouse wheel/trackpad works via terminal, not custom scrolling
2. **Text selection**: OS-level text selection (drag to select, scroll while selecting)
3. **Simpler code**: ~500 fewer lines, no TUI framework complexity
4. **Claude Code parity**: Matches Claude Code's UX (input scrolls out of view)

### Rendering Strategy

**Event Batching** (fixes paste spam):
```rust
// Process ALL pending key events before rendering
while event::poll(Duration::from_millis(0))? {
    if let Event::Key(key) = event::read()? {
        self.handle_input(key).await?;
        had_input = true;
    }
}

// Only render AFTER all events processed
if self.input_needs_render {
    self.render_input_line()?;
}
```

**Word Wrapping**:
```rust
fn wrap_text(text: &str, width: usize) -> String {
    // Split by whitespace, wrap at word boundaries
    // Prevents mid-word breaks like "usin g HTML"
}
```

**Streaming Output**:
- Shows "Synthia: Thinking..." during generation
- Accumulates streaming deltas in buffer
- On completion, wraps accumulated text and displays once
- **Trade-off**: No real-time token-by-token streaming (intentional, matches Claude Code)

---

## Session Persistence

**Storage**: `~/.local/share/synthia/sessions/`

**Format**: JSON with structure:
```json
{
  "id": "session_uuid",
  "messages": [...],
  "created_at": 1234567890,
  "last_modified": 1234567890,
  "model": "qwen2.5-coder-7b-instruct"
}
```

**Features**:
- Auto-save on session end
- Load previous sessions with `Ctrl+L`
- Browse sessions with arrow keys
- Session list shows timestamp, message count, truncated ID

---

## Current Issues & Pain Points

### 1. **Word Wrapping Inconsistencies**

**Status**: Active investigation (2025-10-18)

**Problem**: Text wrapping still breaks mid-word in some cases:
```
"To build a web UI for your Flask TODO app, we'll need to create a simple frontend usin
g HTML, CSS, and JavaScript..."
```

**Attempted Fix**:
- Added `wrap_text()` function to wrap at word boundaries
- Accumulate streaming deltas in buffer
- Re-print wrapped version on completion

**Current Theory**: Terminal may still be wrapping the "Thinking..." line or there's an edge case in the wrapping logic.

**Impact**: Moderate (output is readable but ugly)

---

### 2. **Fine-Tuned Model Tool Calling Issues**

**Model**: `zachswift615/qwen2.5-coder-synthia-tool-use/model-q4_k_m.gguf`
- Fine-tuned Qwen 2.5 Coder for tool use
- Location: `/Users/zachswift/.lmstudio/models/zachswift615/qwen2.5-coder-synthia-tool-use/model-q4_k_m.gguf`

**Problem**: Model sometimes generates malformed tool calls:

**Example Error**:
```
Error: Agent error: Missing 'file_path' parameter
```

**Root Cause Analysis**:
1. **Malformed JSON**: LLM sends incomplete or invalid JSON for tool arguments
2. **Silent failure**: Code was using `.unwrap_or_else(|_| json!({}))` without logging
3. **Misleading error**: Tools received empty `{}` and reported missing required parameters

**Fix Applied** (BUGFIX_SUMMARY.md):
- Added error logging to show raw JSON from LLM
- Now logs: `"Failed to parse tool arguments for tool 'write': <error>. Raw arguments: <json>"`
- Helps diagnose whether issue is LLM or parsing

**Potential Solutions**:
1. **More training data**: Fine-tune with more examples of correct tool call format
2. **Retry logic**: On malformed JSON, return error to LLM and let it retry
3. **Schema validation**: Validate JSON against tool schema before execution
4. **Stricter error handling**: Return error instead of defaulting to `{}`

**Impact**: High (breaks agentic loop when tools fail)

---

### 3. **LM Studio Freezing on Fine-Tuned Model**

**Hardware**: MacBook Pro M-series, 16GB RAM, 10 cores

**Problem**: Running fine-tuned Qwen model caused MacBook to freeze

**Root Cause**: Evaluation Batch Size too high (512) for available RAM

**Solution Applied**:
- GPU Offload: 28 (max, for Metal acceleration)
- Evaluation Batch Size: 128 (reduced from 512)
- Context Length: 4096
- CPU Thread Pool: 7 (leaves cores free)

**Status**: Not yet tested by user

**Impact**: Critical (can't use fine-tuned model)

---

### 4. **Session Bleeding** (FIXED)

**Problem**: Starting new session with `Ctrl+N` didn't clear conversation state, so old session messages appeared in new session.

**Fix**: Clear conversation vector when creating new session.

**Status**: ✅ Resolved

---

### 5. **Ctrl+C Not Canceling** (FIXED)

**Problem**: `Ctrl+C` didn't stop LLM generation.

**Fix**: Implemented cancellation token in agent loop.

**Status**: ✅ Resolved

---

### 6. **Paste Spam** (FIXED)

**Problem**: Pasting text caused dozens of duplicate input line renders (one per character).

**Fix**: Batch all pending key events with `poll(0ms)` before rendering.

**Status**: ✅ Resolved

---

## Configuration

**Location**: `~/.config/synthia/config.toml`

**Example**:
```toml
[llm]
api_base = "http://localhost:1234/v1"
model = "qwen2.5-coder-7b-instruct"
temperature = 0.7
max_tokens = 4096

[timeouts]
bash_timeout = 120     # seconds
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60

[ui]
syntax_highlighting = false  # Deferred feature
max_output_lines = 1000
```

---

## Keyboard Shortcuts

### Session Management
- **Ctrl+S** - Save current session
- **Ctrl+N** - Start new session
- **Ctrl+L** - Load previous session
- **Ctrl+D** - Quit application

### Input Editing
- **←/→** - Move cursor left/right
- **Home/End** - Jump to start/end of input
- **Ctrl+A/E** - Jump to start/end (Emacs-style)
- **Backspace** - Delete character before cursor
- **Delete** - Delete character after cursor
- **Enter** - Send message

### Session List (when visible)
- **↑/↓** - Navigate sessions
- **Enter** - Load selected session
- **Esc** - Close session list

---

## Code Organization

```
synthia/
├── src/
│   ├── agent/
│   │   ├── actor.rs          # Agent orchestration loop
│   │   ├── messages.rs       # Command/UIUpdate enums
│   │   └── mod.rs
│   ├── llm/
│   │   ├── openai.rs         # OpenAI-compatible provider
│   │   ├── provider.rs       # LLMProvider trait
│   │   └── mod.rs
│   ├── tools/
│   │   ├── bash.rs           # Bash tool
│   │   ├── read.rs           # Read tool
│   │   ├── write.rs          # Write tool
│   │   ├── edit.rs           # Edit tool
│   │   ├── grep.rs           # Grep tool
│   │   ├── glob.rs           # Glob tool
│   │   ├── webfetch.rs       # WebFetch tool
│   │   ├── git.rs            # Git tool
│   │   ├── powertools.rs     # Powertools integration
│   │   ├── workshop.rs       # Workshop integration
│   │   ├── registry.rs       # Tool registry
│   │   └── mod.rs
│   ├── ui/
│   │   ├── app.rs            # Terminal-native UI
│   │   ├── markdown.rs       # (Unused, deferred feature)
│   │   └── mod.rs
│   ├── config.rs             # Configuration loading
│   ├── session.rs            # Session persistence
│   ├── types.rs              # Core types (Message, ContentBlock, etc.)
│   └── main.rs               # Entry point
├── config.toml.example
├── README.md
├── CONFIGURATION.md
├── ARCHITECTURE.md           # This file
├── BUGFIX_SUMMARY.md         # Tool calling error fix
└── CURSOR_FIX.md             # Outdated (pre-refactor)
```

---

## Build & Test

**Build**:
```bash
cd synthia
cargo build --release
```

**Run**:
```bash
./target/release/synthia
```

**Tests**:
```bash
cargo test
```

**Current Test Status**:
- 71 tests passing
- Core tools have unit tests
- Integration tests pending

---

## Performance Characteristics

**Binary Size**: ~5 MB (release build)

**Memory Usage**: ~20-30 MB baseline (mostly tokio runtime)

**Startup Time**: <100ms

**Tool Execution**: Depends on tool (bash can be slow, read is instant)

**LLM Latency**: Depends on model and hardware (local inference)

---

## Design Decisions & Trade-offs

### 1. Terminal-Native vs. TUI Framework

**Decision**: Use terminal-native rendering instead of Ratatui

**Rationale**:
- Simpler code (~500 fewer lines)
- Natural OS-level scrolling and text selection
- Matches Claude Code UX
- Easier to maintain

**Trade-off**:
- Less control over rendering (can't easily do split panes, syntax highlighting, etc.)
- Relies on terminal capabilities

---

### 2. No Real-Time Streaming Display

**Decision**: Accumulate streaming deltas, display wrapped text on completion

**Rationale**:
- Matches Claude Code behavior
- Enables proper word-wrapping (can't wrap mid-stream)
- Cleaner output (no unwrapped text flashing by)

**Trade-off**:
- No real-time token-by-token feedback during generation
- User sees "Thinking..." instead of partial output

---

### 3. Actor Model Architecture

**Decision**: Use tokio channels and isolated actors instead of shared state

**Rationale**:
- Clear separation of concerns
- No mutex/lock contention
- Easy to reason about message flow
- Testable components

**Trade-off**:
- Slightly more boilerplate (message passing)
- Harder to share state between actors

---

### 4. OpenAI-Compatible API Only

**Decision**: Only support OpenAI-compatible APIs (not Anthropic, Gemini, etc.)

**Rationale**:
- Most local LLM providers support OpenAI format (LM Studio, Ollama, etc.)
- Simpler code (one provider implementation)
- Easy to swap models

**Trade-off**:
- Can't use native Anthropic API
- Must rely on local proxy/adapter for other formats

---

### 5. Tool Timeout Defaults

**Decision**: Default bash/git timeouts to 120 seconds

**Rationale**:
- Long enough for most operations
- Short enough to prevent infinite hangs
- User-configurable via config

**Trade-off**:
- Some long-running operations may timeout
- Users must configure for slow environments

---

## Roadmap & Future Work

### Near-Term (Next 1-2 Weeks)

1. **Fix word wrapping** - Resolve mid-word breaks
2. **Test fine-tuned model** - Validate LM Studio settings fix
3. **Improve tool call error handling** - Return errors to LLM for retry
4. **Update README** - Document terminal-native refactor

### Medium-Term (Next 1-2 Months)

1. **Markdown rendering** - Re-add with terminal-native approach (inline code, headings)
2. **Streaming UX improvements** - Show progress indicator (e.g., spinning cursor)
3. **Multi-turn tool execution** - Allow LLM to call multiple tools in parallel
4. **Tool call validation** - JSON schema validation before execution
5. **Session search** - Search across past conversations

### Long-Term (Future)

1. **MCP (Model Context Protocol) support** - Integrate with Claude Code MCP servers
2. **Multi-modal support** - Images, PDFs (via base64 encoding)
3. **Custom tool plugins** - User-defined tools via config
4. **Cloud provider integration** - Optional fallback to OpenAI/Anthropic for tough queries
5. **Conversation branching** - Fork conversations at any point
6. **Tool usage analytics** - Track which tools are most/least used

---

## Questions for System Design Review

We're seeking feedback from Claude.ai Opus on the following:

### Architecture

1. **Is the actor model the right choice?** Should we consider alternative patterns (e.g., shared state with mutexes, event sourcing)?

2. **Tool execution strategy**: Currently sequential (one tool at a time). Should we support parallel tool execution for independent operations?

3. **Error recovery**: How should we handle partial failures in multi-tool sequences? Currently, we continue the loop even if tools fail.

4. **Memory management**: Conversation history grows unbounded in-memory. When should we truncate or summarize?

### Fine-Tuned Model Issues

5. **Tool calling reliability**: Our fine-tuned Qwen model struggles with proper JSON formatting for tool calls. What strategies can improve reliability?
   - Should we use few-shot examples in the system prompt?
   - Should we implement retry logic with error messages?
   - Should we pre-validate tool schemas?

6. **Error handling philosophy**: Should we:
   - Return errors to the LLM and let it retry?
   - Default to `{}` and let tools fail gracefully?
   - Halt execution and show error to user?

### UX & Terminal Design

7. **Word wrapping strategy**: Is buffering and wrapping on completion the right approach? Are there better alternatives?

8. **Streaming UX**: We intentionally don't show real-time token output. Is there a middle ground (e.g., show chunks every N tokens)?

9. **Session management**: Should sessions auto-save more frequently (e.g., after each LLM response) or only on exit?

### Performance & Scalability

10. **Tool timeout configuration**: Are our defaults (120s for bash/git) reasonable? Should timeouts be per-operation instead of per-tool?

11. **LLM provider abstraction**: Should we support multiple LLM providers simultaneously (e.g., local + cloud fallback)?

12. **Caching strategy**: Should we cache LLM responses for identical prompts? Tool results for identical inputs?

### Security

13. **Tool execution safety**: Should we implement a sandboxing layer for bash/git commands? A confirmation prompt for destructive operations?

14. **File access controls**: Should tools be restricted to certain directories? How to prevent accidental `/` wipes?

---

## Appendix: Example Tool Schemas

### Bash Tool Schema (OpenAI Format)

```json
{
  "type": "function",
  "function": {
    "name": "bash",
    "description": "Execute a bash command and return stdout/stderr. For long-running processes (servers, watchers), append '&' to run in background. Default timeout: 5 minutes (configurable in synthia.toml).",
    "parameters": {
      "type": "object",
      "properties": {
        "command": {
          "type": "string",
          "description": "The bash command to execute"
        }
      },
      "required": ["command"]
    }
  }
}
```

### Read Tool Schema

```json
{
  "type": "function",
  "function": {
    "name": "read",
    "description": "Read a file from the filesystem",
    "parameters": {
      "type": "object",
      "properties": {
        "file_path": {
          "type": "string",
          "description": "Path to the file to read"
        }
      },
      "required": ["file_path"]
    }
  }
}
```

---

**End of Architecture Overview**

**For Review By**: Claude.ai Opus
**Prepared By**: Zach Swift + Claude Code
**Date**: 2025-10-18
