# Agent Power Tools 🛠️

A powerful code indexing and navigation system designed specifically for AI agents like Claude Code. Built on industry-standard protocols (SCIP, LSP) and leveraging Tree-sitter for fast AST analysis.

## Features

### ✅ Implemented (Tree-sitter based)
- **Pattern Search** - Search for code patterns using Tree-sitter queries
- **Function Finder** - List all functions in a project with signatures
- **Class Finder** - Find classes, structs, interfaces across codebases
- **Statistics** - Get project statistics and language breakdown
- **Multiple Output Formats** - JSON, Text, and Markdown output

### 🚧 In Progress (SCIP/LSP based)
- **Go to Definition** - Jump to where symbols are defined
- **Find References** - Find all usages of a symbol
- **Find Implementations** - Locate implementations of interfaces/traits
- **Find Callers** - Discover where functions are called
- **Type Information** - Get type info for expressions
- **Dependency Analysis** - Understand module dependencies

## Architecture

```
agent-powertools/
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

### Prerequisites

1. Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone the repository:
```bash
git clone https://github.com/yourusername/agent-powertools.git
cd agent-powertools
```

3. Build the project:
```bash
cd powertools-cli
cargo build --release
```

## Usage

### Command Line Interface

```bash
# Search for patterns in AST
powertools search-ast "function_declaration" --path src/

# Find all functions
powertools functions --include-private

# Find all classes/structs
powertools classes --include-nested

# Get project statistics
powertools stats --detailed

# Build/update index (for semantic features)
powertools index
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

### Claude Code Integration

The `.claude/commands/` directory contains wrapper scripts that Claude can invoke:

```bash
# Claude can run these commands:
.claude/commands/search-ast.sh "pattern"
.claude/commands/find-functions.sh
.claude/commands/find-classes.sh
.claude/commands/go-to-definition.sh "file:line:column"
```

These scripts output JSON that Claude can parse and use for code navigation.

## Language Support

| Language | Tree-sitter | SCIP | LSP |
|----------|------------|------|-----|
| Rust | ✅ | 🚧 | 🚧 |
| TypeScript | ✅ | 🚧 | 🚧 |
| JavaScript | ✅ | 🚧 | 🚧 |
| Python | ✅ | 🚧 | 🚧 |
| Go | ✅ | 🚧 | 🚧 |
| Java | ✅ | 🚧 | 🚧 |
| C/C++ | ✅ | 🚧 | 🚧 |

## Performance

- **Tree-sitter queries**: ~1-10ms per file
- **Pattern search**: <1s for 10k files
- **Function/class listing**: <500ms for large projects

## Development

### Running Tests

```bash
cd powertools-cli
cargo test
```

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
- Basic tree-sitter integration
- Pattern searching
- Function/class finding

### Phase 2: Semantic Indexing (Current)
- SCIP index generation
- Go to definition
- Find references
- Cross-file navigation

### Phase 3: Advanced Features
- Type inference
- Call graphs
- Dependency graphs
- Refactoring support

### Phase 4: AI-Specific Features
- Context extraction for prompts
- Intelligent code summarization
- Change impact analysis
- Test coverage mapping

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit PRs.

## License

MIT License - See LICENSE file for details

## Acknowledgments

Built on the shoulders of giants:
- [Tree-sitter](https://tree-sitter.github.io/) - Incremental parsing library
- [SCIP](https://github.com/sourcegraph/scip) - Code Intelligence Protocol
- [rust-analyzer](https://rust-analyzer.github.io/) - Rust Language Server