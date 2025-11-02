# Synthia

> **A secure, powerful AI coding assistant with IDE-level semantic navigation and granular permission control.**

Synthia is a terminal-based AI coding agent that combines the intelligence of modern LLMs with the precision of compiler-grade code indexing. Unlike generic chatbots, Synthia understands your codebase at a semantic level—jumping directly to definitions, finding all references, and refactoring with surgical precision.

## Why Synthia?

**🔒 Security First**
- Granular permission system: approve, deny, or auto-allow tool operations
- Interactive diff preview before file edits with per-file approval
- File size limits and duplicate detection prevent runaway modifications
- You stay in control—no surprise file changes or command executions

**🎯 Semantic Code Navigation**
- Jump to definitions instantly (not grep—real compiler-grade precision)
- Find all references across your entire codebase
- Tree-sitter AST pattern matching for structural code search
- Powered by SCIP indexers (same tech as VS Code IntelliSense)

**🛠️ Powerful Tool Integration**
- Read/write files with diff preview
- Execute bash commands safely with approval
- Git operations with permission control
- Semantic code search and refactoring
- Workshop integration for persistent context across sessions

**🚀 Smart & Fast**
- Token-aware context management (up to 400k with GPT-5)
- Streaming responses with real-time token tracking
- Works with local LLMs (LM Studio, Ollama) or cloud (OpenAI, Anthropic)
- Configurable timeouts and safety limits

**📝 Persistent Memory**
- Workshop CLI integration tracks decisions, gotchas, and preferences
- Context survives across sessions
- Automatic session summaries and history
- Never lose track of "why did we do this?"

## Key Features

### 🔐 Permission System (NEW!)

Synthia's permission system gives you granular control over every operation:

**Three Permission Levels:**
- **Deny**: Block operations entirely (e.g., prevent writes to production configs)
- **Allow**: Auto-approve matching patterns (e.g., allow reading all `.md` files)
- **Ask**: Prompt for approval on each operation (default for sensitive tools)

**Smart Pattern Matching:**
```toml
# ~/.config/synthia/settings-local.json
{
  "allow": [
    "read://**/*.md",           # Auto-approve reading any markdown
    "bash:git status",          # Auto-approve safe git commands
    "bash:cargo test:*"         # Auto-approve test runs with any args
  ],
  "deny": [
    "write://**/config.prod.*", # Block production config writes
    "bash:rm:*"                 # Block dangerous deletions
  ]
}
```

**Interactive Approval:**
When Synthia needs permission, you get a clear prompt:
```
┌─ Permission Required ─────────────────────┐
│ Tool: read                                 │
│ File: src/main.rs                         │
│                                            │
│ Do you want to proceed?                   │
│                                            │
│ → 1. Yes                                  │
│   2. Yes, and don't ask again for *.rs    │
│   3. No (esc)                             │
└────────────────────────────────────────────┘
```

Choose option 2 to save the pattern—never be asked again for similar operations.

### ✏️ Edit Approval with Diff Preview

Before any file modification, Synthia shows you exactly what will change:

```diff
┌─ Edit Preview ──────────────────────────────────┐
│ File: src/utils.rs                              │
│                                                  │
│ -     let result = process();                   │
│ +     let result = process_with_validation();   │
│                                                  │
│ Apply these changes?                            │
│ → 1. Yes                                        │
│   2. Yes, and don't ask for this file           │
│   3. No (esc)                                   │
└──────────────────────────────────────────────────┘
```

**Per-File Approval:** Choose option 2 to trust a specific file for the session—useful when iterating on a single file.

### 🧭 Semantic Code Navigation

Synthia integrates with [powertools](../powertools-cli) for IDE-level code intelligence:

**Goto Definition:**
```
You: "Where is the handle_message function defined?"
Synthia: → Uses goto_definition tool
Result: src/agent/handler.rs:142
```

**Find All References:**
```
You: "Find everywhere we call parse_config"
Synthia: → Uses find_references tool
Found 12 references:
  src/main.rs:45
  src/config.rs:89
  tests/config_test.rs:23
  ...
```

**AST Pattern Search:**
```
You: "Find all async functions in the codebase"
Synthia: → Uses search_ast tool with pattern: (async_function) @func
Found 47 async functions across 12 files
```

**Supported Languages:**
- TypeScript/JavaScript (via scip-typescript)
- Python (via scip-python)
- Rust (via rust-analyzer)
- C++ (via scip-clang)
- Swift (tree-sitter mode, limited to single-file refactoring)

### 🔧 Tool Arsenal

Synthia has direct access to:

| Tool | Purpose | Permission Required |
|------|---------|-------------------|
| **read** | Read files with syntax highlighting | Yes (default: Ask) |
| **write** | Create new files with preview | Yes (default: Ask) |
| **edit** | Modify existing files with diff preview | Yes (always shows diff) |
| **bash** | Execute shell commands | Yes (default: Ask) |
| **git** | Run git operations | Yes (default: Ask) |
| **glob** | Find files by pattern | Optional |
| **grep** | Search file contents by regex | Optional |
| **goto_definition** | Jump to symbol definition (semantic) | Optional |
| **find_references** | Find all symbol usages (semantic) | Optional |
| **search_ast** | Search code structure with tree-sitter | Optional |
| **list_functions** | Extract all function signatures | Optional |
| **list_classes** | Extract all class/struct definitions | Optional |
| **workshop** | Query/update persistent context | Optional |

All tools respect the permission system—you decide what Synthia can do.

### 🧠 Workshop Integration

Synthia integrates with Workshop CLI for persistent context:

**Automatic Context Loading:**
- Session summaries load automatically on startup
- Recent decisions, gotchas, and preferences are always available
- Ask "why did we implement X this way?" and get instant answers

**Workshop Commands (via bash tool):**
```bash
workshop decision "Use async/await for all I/O" -r "Better error handling"
workshop gotcha "Database migrations must run before app start"
workshop preference "Use 4-space indentation" --category code_style
workshop why "async/await"  # Recall why you made past decisions
```

**Cross-Session Memory:**
Workshop maintains context across sessions, so Synthia remembers your project's architecture, coding standards, and past decisions—even weeks later.

## Quick Start

### Option 1: Local LLM (LM Studio)

**Best for:** Privacy, unlimited usage, testing

1. Install [LM Studio](https://lmstudio.ai/)
2. Download a coding model (recommended: Qwen 2.5 Coder 7B or 14B)
3. Start the local server (default: `http://localhost:1234`)
4. Run Synthia:

```bash
cargo run --release
```

**Recommended Models:**
- `qwen2.5-coder-7b-instruct` - Fast, accurate, great for coding
- `qwen2.5-coder-14b-instruct` - More capable, needs 16GB+ RAM
- `deepseek-coder-33b-instruct` - Extremely powerful, needs 32GB+ RAM

**Configuration for LM Studio:**
Create `~/.config/synthia/config.toml`:
```toml
[llm]
api_base = "http://localhost:1234/v1"
api_key = "not-needed"  # LM Studio doesn't require API key
model = "qwen2.5-coder-7b-instruct"
temperature = 0.7
max_tokens = 4000         # Adjust based on model context window
streaming = true
context_window = 32000    # Qwen 2.5 Coder 7B supports 32k

[timeouts]
bash_timeout = 300
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60

[ui]
syntax_highlighting = true
max_output_lines = 1000
edit_approval = true      # Show diff before edits (recommended!)
```

### Option 2: OpenAI GPT-5

**Best for:** Maximum capability, massive context windows, production use

**Compatible Models:**
- ✅ `gpt-5-chat-latest` - 400k context, streaming, fast (RECOMMENDED)
- ✅ `gpt-4o` - 128k context, great performance
- ✅ `gpt-4o-mini` - 128k context, cost-effective
- ❌ `gpt-5-codex` - Uses Responses API (not compatible)

**Configuration:**
Create `~/.config/synthia/config.toml`:
```toml
[llm]
api_base = "https://api.openai.com/v1"
api_key = "sk-proj-YOUR_API_KEY_HERE"
model = "gpt-5-chat-latest"
temperature = 0.7
max_tokens = 16000
streaming = true
context_window = 400000  # GPT-5 has 400k context window

[timeouts]
bash_timeout = 300
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60

[ui]
syntax_highlighting = true
max_output_lines = 1000
edit_approval = true
```

**GPT-5 Performance:**
- Context window: 400,000 tokens (~1.2 million characters)
- Max output: Up to 128,000 tokens
- Speed: Extremely fast (can generate full applications in seconds)
- Cost: Check [OpenAI pricing](https://openai.com/api/pricing/)

### Option 3: Anthropic Claude

**Best for:** Reasoning-heavy tasks, long-context analysis

```toml
[llm]
api_base = "https://api.anthropic.com/v1"
api_key = "sk-ant-YOUR_API_KEY_HERE"
model = "claude-3-5-sonnet-20241022"
temperature = 0.7
max_tokens = 8000
streaming = true
context_window = 200000  # Claude 3.5 Sonnet supports 200k
```

## Building

```bash
# Development build
cargo build

# Release build (optimized, recommended)
cargo build --release

# Run
./target/release/synthia

# Or run directly
cargo run --release
```

## Configuration

Synthia uses a **hierarchical configuration system**:

1. **Project config** (`./synthia.toml`) - Highest priority, per-project overrides
2. **Global config** (`~/.config/synthia/config.toml`) - User-level defaults
3. **Hardcoded defaults** - Fallback values

**Permission config** is stored separately:
- `~/.config/synthia/settings-local.json` - Global permissions (allow/deny/ask patterns)

See [CONFIG.md](CONFIG.md) for complete configuration documentation, including:
- All available configuration fields and defaults
- Remote LM Studio setup over network
- Project-level overrides for team settings
- Troubleshooting common config issues

## Permission Patterns

The permission system uses glob-style patterns for flexible matching:

### Pattern Syntax

**Basic patterns:**
```json
{
  "allow": [
    "read:/path/to/file.rs",          // Exact file path
    "read:/path/to/*.rs",              // All .rs files in directory
    "read:/path/to/**/*.rs",           // All .rs files recursively
    "bash:git status",                 // Exact bash command
    "bash:cargo test:*"                // Command with any arguments
  ]
}
```

**Wildcards:**
- `*` - Matches any characters except `/` (single directory level)
- `**` - Matches any characters including `/` (recursive)
- `:*` - Matches any arguments after command (bash tool only)

**Examples:**

```json
{
  "allow": [
    "read://**/*.md",                  // All markdown files anywhere
    "read://Users/you/projects/**",    // Entire projects directory
    "bash:git status",                 // Safe git command
    "bash:git log:*",                  // Git log with any flags
    "bash:cargo test:*",               // Cargo test with any args
    "bash:npm run build",              // Specific npm script
    "grep:**/*.rs"                     // Grep all Rust files
  ],
  "deny": [
    "write://**/config.prod.*",        // Production configs
    "write://**/secrets.*",            // Secret files
    "bash:rm:*",                       // Dangerous deletions
    "bash:sudo:*",                     // Privileged operations
    "git:push:*"                       // Prevent accidental pushes
  ]
}
```

### Building Permission Patterns

When you choose "Yes, and don't ask again" from a permission prompt, Synthia intelligently builds patterns:

**File operations:**
- Extracts file extension → `read://**/*.rs`
- Preserves directory structure for precision when appropriate

**Bash commands:**
- Exact match for simple commands → `bash:git status`
- Wildcard args for commands with flags → `bash:cargo test:*`

**Manual pattern editing:**
Edit `~/.config/synthia/settings-local.json` directly for complex patterns:

```json
{
  "allow": [
    "read://home/user/project/src/**/*.rs",
    "bash:cargo:*",
    "git:status",
    "git:diff:*"
  ],
  "deny": [
    "write://**/Cargo.lock",
    "bash:cargo publish:*"
  ]
}
```

## Architecture

**Agent Loop:**
- `src/agent/mod.rs` - Core agent orchestration
- `src/agent/messages.rs` - Message types for agent ↔ UI communication

**Tool System:**
- `src/tools/registry.rs` - Tool execution with permission middleware
- `src/tools/bash.rs` - Shell command execution
- `src/tools/files.rs` - File read/write/edit operations
- `src/tools/git.rs` - Git operations
- `src/tools/powertools.rs` - Semantic code navigation

**Permission System:**
- `src/permission_config.rs` - Config data structures and file I/O
- `src/permission_manager.rs` - Permission checking and pattern matching

**UI:**
- `src/ui/app.rs` - Terminal UI with crossterm, permission prompts, diff rendering
- `src/ui/token_tracker.rs` - Real-time token usage tracking

**LLM Providers:**
- `src/llm/openai.rs` - OpenAI Chat Completions API client
- `src/llm/anthropic.rs` - Anthropic Messages API client
- `src/config.rs` - Hierarchical configuration system

## Use Cases

**Code Exploration:**
```
You: "Find all places where we parse JSON in this project"
Synthia: [Uses search_ast and find_references]
→ Found 23 JSON parsing locations across 8 files
```

**Refactoring:**
```
You: "Rename handle_request to handle_api_request everywhere"
Synthia: [Shows preview of all changes with diffs]
→ Found 15 occurrences across 7 files
→ Shows each diff for approval
```

**Bug Investigation:**
```
You: "Why does authenticate() fail when the token is expired?"
Synthia: [Reads authenticate function, traces through token validation]
→ The issue is in src/auth.rs:89 - we check expiry but don't refresh
```

**Documentation:**
```
You: "Document the database module with examples"
Synthia: [Reads module code, writes comprehensive docs]
→ Added module-level docs with usage examples
→ Shows diff before applying
```

**Testing:**
```
You: "Write integration tests for the API endpoints"
Synthia: [Examines endpoint implementations, generates tests]
→ Created tests/api_integration_test.rs with 12 test cases
→ Shows test file diff for approval
```

## Safety Features

**File Protection:**
- 100KB file size limit (prevents loading massive files)
- Per-file write limit (max 2 writes per file per turn)
- Duplicate code detection (prevents infinite loops)
- Diff preview before every edit

**Command Safety:**
- Permission required for all bash commands
- Configurable timeouts (default: 5 minutes)
- Git operations require explicit approval
- Dangerous commands (rm, sudo) easily blocked via deny patterns

**Token Management:**
- Real-time token usage tracking in UI
- Configurable context windows per model
- Automatic truncation when approaching limits
- Warning when context is near capacity

## Troubleshooting

**Problem: Synthia appears frozen when trying to use a tool**
- **Cause:** Permission prompt is active but hidden behind input line
- **Fix:** This was fixed in v0.2.0—upgrade to latest version
- **Workaround:** Hit Ctrl+C and manually approve via settings

**Problem: Local model generates infinitely without stopping**
- **Cause:** Some models struggle with tool-calling workflows
- **Fix:** Use a model fine-tuned for function calling (Qwen 2.5 Coder, Synthia-Coder)
- **Workaround:** Lower `max_tokens` in config to prevent runaway generation

**Problem: "Request too large" errors with OpenAI**
- **Cause:** Tool output exceeds token limits, or `max_tokens` too high
- **Fix:** Reduce `max_tokens` to 8000-16000 in config
- **Workaround:** Ask simpler questions that require less context

**Problem: Permission prompt doesn't appear**
- **Cause:** May be testing old version without permission system
- **Fix:** Ensure you're on latest version (v0.2.0+) with `git pull && cargo build --release`

**Problem: Can't connect to LM Studio**
- **Cause:** Server not running, or wrong port
- **Fix:** Check LM Studio is running and server is started (default: localhost:1234)
- **Fix:** Verify `api_base` in config matches LM Studio's server address

See [CONFIG.md](CONFIG.md) for more troubleshooting tips.

## Recent Changes

### v0.2.0 (2025-01-15) - Permission System

**Major Features:**
- ✨ **Permission System**: Granular control over tool operations (allow/deny/ask)
- ✨ **Smart Pattern Matching**: Glob-style patterns with wildcard support
- ✨ **Interactive Approval**: Arrow-key navigation for permission prompts
- ✨ **Per-File Edit Approval**: "Don't ask again for this file" option
- ✨ **Informational Diffs**: See changes even when auto-approving edits

**Permission System Details:**
- Allow/deny/ask permission levels with pattern matching
- Saved patterns in `~/.config/synthia/settings-local.json`
- UI prompts for bash, read, write, edit, git operations
- Intelligent pattern suggestions based on file extensions and command structure
- Per-session per-file edit approval for rapid iteration

**UI Improvements:**
- Fixed: Permission prompts now always visible (no longer hidden by input line)
- Fixed: Input rendering no longer duplicates "You:" prefix on each character
- Improved: Box-drawing borders for permission/edit approval prompts
- Added: Arrow key navigation for permission options

**Bug Fixes:**
- Fixed: Input line rendering over modal prompts
- Fixed: Input character duplication in terminal
- Fixed: Cursor position calculation during input rendering
- Fixed: Powertools binary path for Cargo workspace structure

### v0.1.1 (2024-10-26)

**UI Improvements:**
- Fixed: Input messages no longer echoed back to user
- Fixed: Token usage stats now update in real-time in header
- Added: Token usage display shows `Context: X / Y tokens (Z%)`

**Configuration:**
- Added: Comprehensive config hierarchy (project → global → defaults)
- Added: `CONFIG.md` documentation with examples
- Fixed: Config location now properly uses `~/.config/synthia/config.toml`

**Safety Safeguards:**
- Added: File size limit (100KB max) to prevent massive files
- Added: Duplicate code detection to prevent loop generation
- Added: Per-file write limit (max 2 writes per file per turn)

**OpenAI Integration:**
- Verified: GPT-5-chat-latest compatibility (400k context, streaming)
- Documented: Chat Completions API vs Responses API differences
- Note: GPT-5-Codex requires Responses API (not yet supported)

## Roadmap

**v0.3.0 (Planned):**
- [ ] Multi-model support (run local + cloud models simultaneously)
- [ ] Plugin system for custom tools
- [ ] Web UI option (keep terminal as primary)
- [ ] Session branching (explore multiple solution paths)

**v0.4.0 (Planned):**
- [ ] Code review mode (static analysis integration)
- [ ] Automated testing suggestions
- [ ] Performance profiling integration
- [ ] Collaborative features (shared sessions)

## Contributing

Contributions welcome! Synthia is part of the [agent-power-tools](../) monorepo.

**Development setup:**
```bash
git clone https://github.com/yourusername/agent-power-tools.git
cd agent-power-tools/synthia
cargo build
cargo test
```

**Areas for contribution:**
- New LLM provider integrations (Ollama, Together.ai, etc.)
- Additional semantic indexers (Go, Java, C#)
- Improved permission pattern matching
- UI enhancements (syntax themes, layout options)
- Documentation and examples

## License

MIT

---

**Made with ❤️ by developers who believe AI assistants should be powerful, transparent, and trustworthy.**
