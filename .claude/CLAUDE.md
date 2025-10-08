# Agent Power Tools

This project has a powerful code indexing system available via the `powertools` binary.

## Power Tools Commands

The powertools binary is located at: `./powertools-cli/target/release/powertools`

### Available Commands:

#### Semantic Navigation (SCIP-based)
```bash
# Index a project (auto-detects TypeScript, JavaScript, Python, Rust)
./powertools-cli/target/release/powertools index --auto-install

# Index only specific languages
./powertools-cli/target/release/powertools index --languages typescript python

# Go to definition (returns JSON with file path, line, column)
./powertools-cli/target/release/powertools definition src/file.ts:10:5 --format json -p /path/to/project

# Find all references to a symbol
./powertools-cli/target/release/powertools references myFunction --format json -p /path/to/project

# Include declarations in references
./powertools-cli/target/release/powertools references myFunction --include-declarations --format json
```

**When to use:**
- Use `index` when starting work on a new project or when files have changed significantly
- Use `definition` when you need to find where a function/variable is defined
- Use `references` when you need to find all usages of a symbol

**Output:** All commands support `--format json` which returns structured data perfect for parsing.

#### Tree-sitter Pattern Matching
```bash
# Search for AST patterns using tree-sitter queries
./powertools-cli/target/release/powertools search-ast "(function_item) @func" -p src/

# Find all functions in a project
./powertools-cli/target/release/powertools functions --format json

# Find all classes/structs
./powertools-cli/target/release/powertools classes --format json

# Get project statistics
./powertools-cli/target/release/powertools stats

# Get help
./powertools-cli/target/release/powertools --help
```

### Example Tree-sitter Patterns:
- Rust functions: `(function_item) @func`
- TypeScript functions: `(function_declaration) @func`
- Python functions: `(function_definition) @func`
- Find async functions: `(async_function) @func`
- Find classes: `(class_declaration) @class`

### Supported Languages:
- **TypeScript**: Full semantic navigation via scip-typescript
- **JavaScript**: Full semantic navigation via scip-typescript (requires tsconfig.json with `allowJs: true`)
- **Python**: Full semantic navigation via scip-python
- **Rust**: Full semantic navigation via rust-analyzer

**Multi-language projects:** Powertools automatically detects and indexes all languages in a project. For example, a project with both TypeScript and Python will generate both `index.typescript.scip` and `index.python.scip`, and queries will search across both.

### Output Formats:
Use `--format json` for structured data that's easy to parse.

## Workshop CLI Integration

This project uses Workshop, a persistent context tool. At the start of each session, Workshop context is automatically loaded. At the end of each session, a summary is automatically saved.

## Workshop Commands

**Use Workshop liberally throughout the session to:**
- Record decisions: `workshop decision "<text>" -r "<reasoning>"`
- Document gotchas: `workshop gotcha "<text>" -t tag1 -t tag2`
- Add notes: `workshop note "<text>"`
- Track preferences: `workshop preference "<text>" --category code_style`
- Manage state: `workshop goal add "<text>"` and `workshop next "<text>"`

**Query context (use these frequently!):**
- `workshop why "<topic>"` - THE KILLER FEATURE! Answers "why did we do X?" - prioritizes decisions with reasoning
- `workshop context` - View session summary
- `workshop search "<query>"` - Find relevant entries
- `workshop recent` - Recent activity
- `workshop summary` - Activity overview
- `workshop sessions` - View past session history
- `workshop session last` - View details of the most recent session

**Important:** Workshop helps maintain continuity across sessions. Document architectural decisions, failed approaches, user preferences, and gotchas as you discover them.

**Best Practice:** When you wonder "why did we choose X?" or "why is this implemented this way?", run `workshop why "X"` first before asking the user!

## Importing Past Sessions

Workshop can import context from past Claude Code sessions stored in JSONL transcript files:

- **When to suggest:** If the user mentions wanting context from previous sessions, or asks "why" questions that might be answered by historical context, suggest running `workshop import --execute`
- **First-time import:** Always ask the user before running import for the first time - it can extract hundreds of entries from historical sessions
- **What it does:** Analyzes JSONL transcripts and automatically extracts decisions, gotchas, and preferences from past conversations
- **Command:** `workshop import --execute` (without --execute it's just a preview)
- **Location:** By default, imports from the current project's JSONL files in `~/.claude/projects/`

**Important:** You have permission to run `workshop import --execute`, but always ask the user first, especially if import has never been run in this project. Let them decide if they want to import historical context.
