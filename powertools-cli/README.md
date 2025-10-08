# Agent Power Tools

Powerful code indexing and navigation tools optimized for AI agents. Provides semantic code navigation, pattern searching, and code analysis via SCIP (Source Code Intelligence Protocol) and tree-sitter.

## Features

- 🔍 **Semantic Navigation**: Go to definition, find references, powered by SCIP
- 🌲 **AST Search**: Pattern matching with tree-sitter queries
- 🚀 **Multi-language**: TypeScript, JavaScript, Python, Rust
- 🤖 **AI-Optimized**: JSON output, MCP server integration
- ⚡ **Fast**: Rust implementation with parallel processing

## Installation

### Option 1: Install Script (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/zachswift615/agent-powertools/main/install.sh | sh
```

This automatically detects your platform and installs the latest version.

### Option 2: Homebrew (macOS/Linux)

```bash
# Add the tap
brew tap zachswift615/powertools https://github.com/zachswift615/agent-powertools

# Install
brew install powertools
```

### Option 3: Download Binary

Download the latest release for your platform from [GitHub Releases](https://github.com/zachswift615/agent-powertools/releases):

- **macOS ARM64** (M1/M2/M3): `powertools-macos-arm64.tar.gz`
- **macOS x86_64** (Intel): `powertools-macos-x86_64.tar.gz`
- **Linux x86_64**: `powertools-linux-x86_64.tar.gz`

```bash
# Extract and install
tar xzf powertools-*.tar.gz
sudo mv powertools /usr/local/bin/
```

### Option 4: Build from Source

Requires [Rust](https://rustup.rs/) to be installed.

```bash
git clone https://github.com/zachswift615/agent-powertools.git
cd agent-powertools/powertools-cli
cargo build --release

# Binary will be at: target/release/powertools
```

## Quick Start

### 1. Index a Project

```bash
# Auto-detect and index all languages
powertools index --auto-install

# Index specific languages
powertools index --languages typescript python
```

### 2. Navigate Code

```bash
# Go to definition
powertools definition src/main.ts:10:5

# Find references
powertools references myFunction

# List all functions
powertools functions --path src/
```

### 3. Search with AST

```bash
# Find all async functions in TypeScript
powertools search-ast "(async_function) @func" --path src/

# Find classes
powertools classes
```

## MCP Integration

Powertools can run as an [MCP (Model Context Protocol)](https://modelcontextprotocol.io) server, making all commands available as first-class tools in Claude Code and other MCP clients.

### Setup for Claude Code

1. **Create config file** at `~/.config/claude/mcp_settings.json`:

\`\`\`json
{
  "mcpServers": {
    "powertools": {
      "command": "/usr/local/bin/powertools",
      "args": ["--mcp-server"],
      "description": "Semantic code navigation and analysis tools"
    }
  }
}
\`\`\`

> **Note**: Adjust the \`command\` path based on where powertools is installed:
> - Homebrew: \`/opt/homebrew/bin/powertools\` (M1/M2/M3) or \`/usr/local/bin/powertools\` (Intel)
> - Install script: \`~/.local/bin/powertools\`
> - Find it with: \`which powertools\`

2. **Restart Claude Code**

3. **Verify**: The following tools should be available in Claude Code:
   - \`index_project\` - Index a project for semantic navigation
   - \`goto_definition\` - Find where a symbol is defined
   - \`find_references\` - Find all references to a symbol
   - \`search_ast\` - Search for code patterns
   - \`list_functions\` - List all functions
   - \`list_classes\` - List all classes/structs
   - \`project_stats\` - Get codebase statistics

## Supported Languages

| Language | SCIP Indexing | Tree-sitter Search |
|----------|---------------|-------------------|
| TypeScript | ✅ | ✅ |
| JavaScript | ✅ | ✅ |
| Python | ✅ | ✅ |
| Rust | ✅ | ✅ |
| Go | ❌ | ✅ |
| Java | ❌ | ✅ |
| C++ | ❌ | ✅ |

## Commands

\`\`\`bash
powertools --help

# Indexing
powertools index [OPTIONS] [PATH]

# Navigation
powertools definition <file:line:column>
powertools references <symbol>
powertools implementations <name>
powertools callers <function>

# Search & Analysis
powertools search-ast <pattern> [OPTIONS]
powertools functions [OPTIONS]
powertools classes [OPTIONS]
powertools stats [OPTIONS]
powertools symbols <query>

# MCP Server
powertools --mcp-server
\`\`\`

## Output Formats

All commands support \`--format\` flag:
- \`text\` - Human-readable (default)
- \`json\` - Structured JSON for AI agents
- \`markdown\` - Formatted markdown

Example:
\`\`\`bash
powertools functions --format json | jq
\`\`\`

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for TypeScript/JavaScript indexing)
- Python 3.8+ (for Python indexing)

### Build

\`\`\`bash
cd powertools-cli
cargo build --release
\`\`\`

### Test

\`\`\`bash
cargo test
\`\`\`

### Run Locally

\`\`\`bash
cargo run -- index --help
\`\`\`

## Contributing

Contributions welcome! Please open an issue or PR.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- [SCIP](https://github.com/sourcegraph/scip) - Source Code Intelligence Protocol
- [tree-sitter](https://tree-sitter.github.io/) - Parser generator and incremental parsing library
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - Rust MCP SDK
