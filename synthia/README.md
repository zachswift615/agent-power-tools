# Synthia

An AI coding assistant with powerful tool integration and semantic code navigation.

## Features

- **Tool Calling**: Read/write files, execute bash commands, search code, git operations
- **Semantic Navigation**: Integrated with powertools for goto-definition, find-references
- **Context Management**: Token-aware conversation with configurable context windows
- **Workshop Integration**: Persistent context across sessions with decisions, gotchas, preferences
- **Streaming Responses**: Real-time LLM output with token usage tracking
- **Safety Safeguards**: File size limits, duplicate detection, per-file write limits

## Quick Start

### Local LLM (LM Studio)

1. Install and run [LM Studio](https://lmstudio.ai/)
2. Load a model (e.g., Qwen 2.5 Coder 7B)
3. Start the local server (default: `http://localhost:1234`)
4. Run Synthia:

```bash
cargo run --release
```

Synthia will use the default config pointing to `http://localhost:1234/v1`.

### OpenAI GPT-5

Synthia supports OpenAI's GPT-5 models via the Chat Completions API.

**Compatible Models:**
- ✅ `gpt-5-chat-latest` - Works with Chat Completions API (RECOMMENDED)
- ✅ `gpt-4o` - Works with Chat Completions API
- ✅ `gpt-4o-mini` - Works with Chat Completions API
- ❌ `gpt-5-codex` - Requires Responses API (NOT COMPATIBLE)

**Important:** GPT-5-Codex uses the [Responses API](https://platform.openai.com/docs/api-reference/responses), which is a different endpoint than Chat Completions. Synthia currently only supports the Chat Completions API, so use `gpt-5-chat-latest` instead.

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
- Context window: 400,000 tokens
- Max output: Up to 128,000 tokens
- Speed: Extremely fast (can generate full apps in seconds)
- Cost: Check [OpenAI pricing](https://openai.com/api/pricing/)

## Configuration

Synthia uses a hierarchical configuration system:

1. **Project config** (`./synthia.toml`) - Highest priority
2. **Global config** (`~/.config/synthia/config.toml`) - User settings
3. **Hardcoded defaults** - Fallback

See [CONFIG.md](CONFIG.md) for complete configuration documentation, including:
- All configuration fields and defaults
- Remote LM Studio setup
- Project-level overrides
- Troubleshooting

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run
./target/release/synthia
```

## Recent Changes

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

## License

MIT
