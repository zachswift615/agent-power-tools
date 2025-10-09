# Agent Power Tools 🛠️

A powerful code indexing and navigation system designed specifically for AI agents like Claude Code. Built on industry-standard protocols (SCIP) and leveraging Tree-sitter for fast AST analysis, with full MCP (Model Context Protocol) integration.

## Quick Start

```bash
# Install via Homebrew
brew tap zachswift615/powertools
brew install powertools

# Run as MCP server
powertools --mcp-server

# Or use directly
powertools index --auto-install      # Index your project
powertools functions --format json   # List all functions
powertools definition src/app.ts:42:10  # Go to definition
```

For Claude Code integration, add a `.mcp.json` file to your project root (see [MCP Server section](#mcp-server-recommended-for-claude-code)).

## Features

### ✅ File Watching and Auto Re-indexing (NEW in v0.2.0)
- **Automatic Re-indexing** - MCP server watches for file changes and re-indexes automatically
- **Smart Debouncing** - 2-second debounce prevents spam during rapid file edits
- **Language-Specific** - Only re-indexes the changed language (5s vs 30s on mixed projects)
- **CLI Watch Mode** - Manual file watching with `powertools watch`
- **MCP Control Tools** - `watcher_start`, `watcher_stop`, `get_watcher_status`
- **Ignore Patterns** - Respects `.git/`, `target/`, `node_modules/`, etc.

### ✅ Semantic Code Navigation (SCIP-based)
- **Go to Definition** - Jump to where symbols are defined
- **Find References** - Find all usages of a symbol across the codebase
- **Multi-language Support** - TypeScript, JavaScript, Python, Rust, and C++
- **Auto-indexing** - Automatically installs and runs language-specific indexers
- **Pagination** - Handle large result sets efficiently (default 100, customizable)

### ✅ AST Pattern Matching (Tree-sitter based)
- **Pattern Search** - Search for code patterns using Tree-sitter queries
- **Function Finder** - List all functions in a project with signatures
- **Class Finder** - Find classes, structs, interfaces across codebases
- **Statistics** - Get project statistics and language breakdown
- **Multiple Output Formats** - JSON, Text, and Markdown output

### ✅ MCP Server Integration
- **Claude Code Native** - All tools available as first-class MCP tools
- **Auto-start Watcher** - File watcher starts automatically when MCP server starts
- **Automatic Discovery** - Tools appear in Claude Code after configuration
- **JSON Responses** - Structured data perfect for AI consumption
- **Project-level Config** - `.mcp.json` can be committed for team collaboration

## Architecture

```
agent-power-tools/
├── powertools-cli/          # Rust CLI implementation
│   ├── src/
│   │   ├── analyzers/       # Tree-sitter based analysis
│   │   ├── commands/        # CLI command implementations
│   │   ├── core/           # Shared types and utilities
│   │   └── indexers/       # SCIP/LSP indexing (WIP)
│   └── Cargo.toml
├── .claude/
│   └── commands/           # Claude Code wrapper scripts
└── scripts/
    └── powertools          # Main CLI wrapper
```

## Technology Stack

- **Tree-sitter** - Fast incremental parsing for pattern matching
- **SCIP** (Sourcegraph Code Intelligence Protocol) - Semantic indexing
- **LSP** (Language Server Protocol) - Language-specific intelligence
- **Rust** - Fast, single-binary distribution

## Installation

### Option 1: Homebrew (Recommended for macOS/Linux)

```bash
brew tap zachswift615/powertools
brew install powertools
```

### Option 2: From Source

1. Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone the repository:
```bash
git clone https://github.com/zachswift615/agent-power-tools.git
cd agent-power-tools
```

3. Build the project:
```bash
cd powertools-cli
cargo build --release
```

The binary will be available at `powertools-cli/target/release/powertools`

## Usage

### MCP Server (Recommended for Claude Code)

The best way to use powertools with Claude Code is through MCP integration:

1. Create a `.mcp.json` file in your project root:

```json
{
  "mcpServers": {
    "powertools": {
      "command": "powertools",
      "args": ["--mcp-server"]
    }
  }
}
```

2. Restart Claude Code - the tools will appear automatically!

**Available MCP Tools:**
- `index_project` - Index your project for semantic navigation
- `goto_definition` - Find where a symbol is defined
- `find_references` - Find all references to a symbol (with pagination)
- `search_ast` - Search using tree-sitter patterns (with pagination)
- `list_functions` - List all functions (with pagination)
- `list_classes` - List all classes/structs (with pagination)
- `project_stats` - Get codebase statistics
- `watcher_start` - Start the file watcher (auto-starts by default)
- `watcher_stop` - Stop the file watcher
- `get_watcher_status` - Get watcher status and project info

All tools support pagination with `limit` (default 100) and `offset` (default 0) parameters.

**Note:** The file watcher starts automatically when the MCP server starts. Use `watcher_stop` to pause auto re-indexing during bulk operations, then `watcher_start` to resume.

### Command Line Interface

```bash
# Index your project (auto-installs language indexers)
powertools index --auto-install

# Watch for file changes and auto re-index (NEW in v0.2.0)
powertools watch                    # Watch current directory
powertools watch --debounce 5       # Custom debounce (seconds)
powertools watch --auto-install     # Auto-install indexers if missing

# Semantic navigation
powertools definition src/file.ts:10:5 --format json
powertools references myFunction --format json

# Search for patterns in AST
powertools search-ast "(function_declaration) @func" --path src/

# Find all functions
powertools functions --include-private --format json

# Find all classes/structs
powertools classes --include-nested --format json

# Get project statistics
powertools stats
```

### Tree-sitter Query Examples

Find all async functions:
```bash
powertools search-ast "(async_function) @func"
```

Find functions starting with "handle":
```bash
powertools search-ast '(function_declaration name: (identifier) @name (#match? @name "^handle"))'
```

Find all class constructors:
```bash
powertools search-ast "(constructor) @ctor"
```

## Language Support

| Language | Tree-sitter | SCIP (Semantic) | Auto-Install |
|----------|------------|-----------------|--------------|
| TypeScript | ✅ | ✅ | ✅ (`@sourcegraph/scip-typescript`) |
| JavaScript | ✅ | ✅ | ✅ (`@sourcegraph/scip-typescript`) |
| Python | ✅ | ✅ | ✅ (`@sourcegraph/scip-python`) |
| Rust | ✅ | ✅ | ✅ (`rust-analyzer`) |
| C++ | ✅ | ✅ | ✅ (`scip-clang`) |
| Go | ✅ | ⏳ | - |
| Java | ✅ | ⏳ | - |

**Legend:**
- ✅ Fully supported
- ⏳ Planned
- Tree-sitter: Pattern matching, function/class listing
- SCIP: Go to definition, find references
- Auto-Install: Automatically installs required indexers

**C++ Requirements:**
- Requires `compile_commands.json` (compilation database)
- Generate with CMake: `cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..`
- Or use Bear for Make projects: `bear -- make`
- scip-clang auto-downloads and installs to `~/.local/bin`

## Performance

- **Tree-sitter queries**: ~1-10ms per file
- **Pattern search**: <1s for 10k files
- **Function/class listing**: <500ms for large projects
- **SCIP indexing**: ~10-30s for medium projects (auto-cached)
- **Pagination**: Default 100 results prevents token limit errors
- **Multi-language**: Indexes all detected languages in parallel

**Tested on:**
- alloy-crm (1,975 files, Python/JavaScript): Successfully indexed and navigated
- agent-powertools (Rust): <5s full index

## Development

### Running Tests

```bash
cd powertools-cli
cargo test
```

### Release Process

We use an automated release script to streamline version bumping and tagging:

**Interactive mode** (prompts for major/minor/patch):
```bash
./scripts/release.sh
```

**Explicit version**:
```bash
./scripts/release.sh 1.2.3
```

The script will:
1. Update version in `Cargo.toml`
2. Commit the version bump
3. Push to main
4. Create and push the git tag
5. Trigger GitHub Actions to build and release binaries

See [scripts/README.md](scripts/README.md) for detailed documentation.

### Adding New Languages

1. Add tree-sitter grammar dependency to `Cargo.toml`
2. Update `Language` enum in `src/core/types.rs`
3. Add language-specific patterns in analyzers
4. Test with sample code

### Extending Commands

Commands are modular - add new ones by:
1. Creating a module in `src/commands/`
2. Adding the command to the CLI enum in `main.rs`
3. Creating a wrapper script in `.claude/commands/`

## Roadmap

### Phase 1: AST Analysis ✅
- ✅ Basic tree-sitter integration
- ✅ Pattern searching
- ✅ Function/class finding
- ✅ Multi-language support

### Phase 2: Semantic Indexing ✅
- ✅ SCIP index generation (TypeScript, JavaScript, Python, Rust)
- ✅ Go to definition
- ✅ Find references
- ✅ Cross-file navigation
- ✅ MCP server integration
- ✅ Pagination for large result sets

### Phase 3: Advanced Features (Current)
- ⏳ Additional language support (Go, Java, C/C++)
- ⏳ Find implementations
- ⏳ Type inference
- ⏳ Call graphs
- ⏳ Dependency graphs

### Phase 4: AI-Specific Features
- ⏳ Context extraction for prompts
- ⏳ Intelligent code summarization
- ⏳ Change impact analysis
- ⏳ Test coverage mapping

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit PRs.

## License

MIT License - See LICENSE file for details

## Acknowledgments

Built on the shoulders of giants:
- [Tree-sitter](https://tree-sitter.github.io/) - Incremental parsing library
- [SCIP](https://github.com/sourcegraph/scip) - Code Intelligence Protocol
- [rust-analyzer](https://rust-analyzer.github.io/) - Rust Language Server