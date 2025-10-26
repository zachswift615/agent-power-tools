# Synthia

> A Claude Code clone for local LLMs

Synthia is a powerful terminal-based AI assistant that brings Claude Code's capabilities to local LLMs. Built with Rust and powered by any OpenAI-compatible API, Synthia provides a rich conversational interface with 10 integrated tools for code navigation, file editing, and more.

## Features

- **🔄 Streaming Responses** - Real-time text streaming with "Thinking..." indicator
- **📝 Rich Markdown Rendering** - Beautiful code blocks with syntax highlighting
- **💾 Session Persistence** - Auto-save conversations and resume later
- **🛠️ 10 Powerful Tools** - Bash, file operations, git, web search, semantic code navigation
- **⚡ Fast & Lightweight** - Native Rust performance
- **🎨 Clean TUI** - Mouse and keyboard scrolling, intuitive controls
- **🔌 OpenAI-Compatible** - Works with LM Studio, Ollama, OpenAI, and more
- **✅ Edit Approval with Diff Preview** - Review all file edits before they execute, with syntax-highlighted diffs
- **Project-Specific Instructions:** Customize Synthia's behavior per-project via `.synthia/.SYNTHIA.md`

## Quick Start

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/yourusername/synthia.git
   cd synthia
   ```

2. **Build from source:**
   ```bash
   cargo build --release
   ```

3. **Configure your LLM provider:**
   ```bash
   mkdir -p ~/.config/synthia
   cp config.toml.example ~/.config/synthia/config.toml
   # Edit config.toml with your API settings
   ```

4. **Run Synthia:**
   ```bash
   ./target/release/synthia
   ```

### Configuration

Create a configuration file at `~/.config/synthia/config.toml`:

```toml
[llm]
api_base = "http://localhost:1234/v1"  # LM Studio
model = "qwen2.5-coder-7b-instruct"
temperature = 0.7
max_tokens = 4096

[timeouts]
bash_timeout = 120
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60

[ui]
syntax_highlighting = true
max_output_lines = 1000
```

For detailed configuration options, see [CONFIGURATION.md](CONFIGURATION.md).

## Tools

Synthia comes with 10 integrated tools:

### Core Tools

- **Bash** - Execute shell commands with configurable timeouts
- **Read** - Read file contents with line range support
- **Write** - Create or overwrite files
- **Edit** - Make precise edits using find-and-replace
- **Grep** - Search file contents with regex patterns
- **Glob** - Find files matching patterns

### Advanced Tools

- **WebFetch** - Fetch and process web content
- **Git** - Execute git operations with timeout controls
- **Powertools** - Semantic code navigation (goto definition, find references, etc.)
- **Workshop** - Persistent context and session management

## Keyboard Shortcuts

### Session Management
- **Ctrl+S** - Save current session
- **Ctrl+N** - Start new session
- **Ctrl+L** - Load previous session
- **Ctrl+D** - Quit application

### Navigation
- **↑/↓ Arrow Keys** - Scroll conversation history
- **Mouse Wheel** - Scroll conversation (3 lines per scroll)
- **Enter** - Send message
- **Backspace** - Delete character

### Session List (when visible)
- **↑/↓** - Navigate sessions
- **Enter** - Load selected session
- **Esc** - Close session list

## Usage Examples

### Basic Conversation

```
User: Hello! Can you help me understand this codebase?
Assistant: Of course! I can analyze your codebase using several tools...
```

### File Operations

```
User: Create a new Python file called hello.py with a hello world function
Assistant: [Uses Write tool to create file]
```

### Code Navigation

```
User: Find all references to the process_data function
Assistant: [Uses Powertools to find semantic references across files]
```

### Git Operations

```
User: Commit these changes with message "Add feature X"
Assistant: [Uses Git tool to create commit]
```

## Architecture

Synthia is built with a clean, modular architecture:

- **Agent** - Orchestrates LLM interactions and tool execution
- **LLM Provider** - Supports any OpenAI-compatible API
- **Tool Registry** - Dynamic tool loading and execution
- **Session Manager** - Persistent conversation storage
- **TUI** - Ratatui-based terminal interface with markdown rendering

### Technology Stack

- **Language**: Rust 2021 edition
- **UI Framework**: [Ratatui](https://github.com/ratatui-org/ratatui)
- **Terminal**: [Crossterm](https://github.com/crossterm-rs/crossterm)
- **Async Runtime**: [Tokio](https://tokio.rs)
- **HTTP Client**: [Reqwest](https://github.com/seanmonstar/reqwest)

## Session Persistence

Synthia automatically saves your conversations to:
```
~/.local/share/synthia/sessions/
```

Each session includes:
- Session ID and metadata
- Full message history
- Timestamp information
- Model configuration

Load previous sessions with `Ctrl+L` and browse by timestamp or message count.

## Troubleshooting

### LLM Connection Issues

**Problem**: Cannot connect to LLM provider

**Solution**:
- Verify API endpoint is correct in `config.toml`
- Check if LM Studio (or other provider) is running
- Test with: `curl http://localhost:1234/v1/models`

### Tool Execution Timeouts

**Problem**: Bash or Git commands timing out

**Solution**: Increase timeout in `config.toml`:
```toml
[timeouts]
bash_timeout = 300  # 5 minutes
git_timeout = 300
```

### Session Loading Errors

**Problem**: Cannot load previous sessions

**Solution**:
- Check session directory exists: `~/.local/share/synthia/sessions/`
- Verify session files are valid JSON
- Check file permissions

### Markdown Not Rendering

**Problem**: Code blocks not showing syntax highlighting

**Solution**: Enable in config:
```toml
[ui]
syntax_highlighting = true
```

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Logs

```bash
RUST_LOG=debug cargo run
```

### Project Structure

```
synthia/
├── src/
│   ├── agent/          # Agent actor and message handling
│   ├── llm/            # LLM provider implementations
│   ├── tools/          # Tool implementations
│   ├── ui/             # TUI and markdown rendering
│   ├── config.rs       # Configuration management
│   ├── session.rs      # Session persistence
│   └── main.rs         # Entry point
├── config.toml.example # Example configuration
└── CONFIGURATION.md    # Configuration documentation
```

## Roadmap

- [ ] MCP (Model Context Protocol) server support
- [ ] Multi-modal support (images, PDFs)
- [ ] Custom tool plugins
- [ ] Session export/import
- [ ] Cloud provider integration
- [ ] Conversation search
- [ ] Tool usage analytics

## Project-Specific Instructions

Synthia supports project-level custom instructions via `.synthia/.SYNTHIA.md`. This allows you to:

- Define project-specific conventions and guidelines
- Provide context about your codebase
- Customize Synthia's behavior for different projects

See [.SYNTHIA.md Guide](docs/SYNTHIA_MD_GUIDE.md) for details.

Quick example:

```bash
# Synthia auto-creates .synthia/.SYNTHIA.md on startup
# Add your custom instructions:
echo "You are helping with a FastAPI project. Follow PEP 8." > .synthia/.SYNTHIA.md

# Restart Synthia to load the instructions
synthia
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## License

[License information to be added]

## Acknowledgments

- Inspired by [Claude Code](https://claude.ai/claude-code)
- Built with [Ratatui](https://github.com/ratatui-org/ratatui)
- Powered by open-source LLMs

---

**Made with ❤️ for the local LLM community**
