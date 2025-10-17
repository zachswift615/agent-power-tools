# Agent Power Tools

This project has a powerful code indexing system available via the `powertools` binary.

## Power Tools Commands

The powertools binary is located at: `./powertools-cli/target/release/powertools`

### MCP Server Mode

Powertools can run as an MCP (Model Context Protocol) server, making all commands available as first-class tools in Claude Code:

```bash
# Run as MCP server (communicates via stdio)
./powertools-cli/target/release/powertools --mcp-server
```

**Claude Code Integration:**
To enable MCP integration, create a `.mcp.json` file at your project root:

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

**Important:** The file must be named `.mcp.json` (not `mcp_settings.json`) and placed at the project root. This file can be committed to git for team collaboration.

After creating the file and restarting Claude Code, the following tools will be available:

**Core Navigation Tools:**
- `index_project` - Index a project for semantic navigation (auto-installs indexers)
- `goto_definition` - Find where a symbol is defined
- `find_references` - Find all references to a symbol (with pagination)
- `search_ast` - Search for code patterns using tree-sitter queries (with pagination)
- `list_functions` - List all functions in a file or directory (with pagination)
- `list_classes` - List all classes, structs, or interfaces (with pagination)
- `project_stats` - Get codebase statistics

**File Watcher Tools (NEW in v0.2.0):**
- `watcher_start` - Start automatic re-indexing when files change
- `watcher_stop` - Pause automatic re-indexing
- `get_watcher_status` - Check if watcher is running and get project info

**Batch Operations Tools (NEW in v0.3.0, Production-Ready v0.3.1):**
- `batch_replace` - Replace text across multiple files using regex patterns with preview
  - **Production-Tested**: Validated on real projects (TanStack Query, poetry-core, nlohmann/json)
  - **Supports**: Regex capture groups ($1, $2), file glob filtering, risk assessment
  - **Safe by default**: Preview mode prevents accidental mass edits

**Important: The file watcher starts AUTOMATICALLY when the MCP server starts!** This means:
- Indexes stay fresh as the user edits code
- You don't need to manually re-index after file changes
- If you get "symbol not found" errors, the index might be rebuilding (wait 2-5s and retry)

**When to use watcher tools:**
- **Use `watcher_stop`** before bulk operations (e.g., mass file edits, git operations, npm install) to avoid re-index spam
- **Use `watcher_start`** after bulk operations to resume automatic indexing
- **Use `get_watcher_status`** to check if the watcher is running or to show the user what's being monitored
- **DO NOT** manually call `index_project` on every file change - the watcher handles this automatically!

**When to use batch_replace:**
- **ALWAYS use `preview=true` FIRST** - Never apply batch replacements without previewing!
- **Use for repetitive edits** - Replace patterns across multiple files in one operation
- **Supports regex** - Use capture groups like $1, $2 for complex replacements
- **File filtering** - Use `file_pattern` param (e.g., "*.ts", "**/*.rs") to limit scope
- **Examples:**
  - Fix typos across codebase: `batch_replace("recieve", "receive", preview=true)`
  - Update API URLs: `batch_replace("api\\.old\\.com", "api.new.com", file_pattern="**/*.ts", preview=true)`
  - Add optional chaining: `batch_replace("user\\.([a-zA-Z]+)", "user?.$1", file_pattern="**/*.ts", preview=true)`

**Pagination Support (v0.1.3+):**
All MCP tools that return lists support pagination to prevent token limit errors:
- `limit` parameter: Maximum results to return (default: 100)
- `offset` parameter: Number of results to skip (default: 0)
- Response includes: `count` (total), `has_more` (boolean), and result data

Example: On a project with 1,438 functions, `list_functions` with `limit=100` returns only 100 results instead of exceeding token limits.

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
- **Use `index_project`** only when: (1) Starting work on a new project for the first time, OR (2) The watcher is stopped and you need to manually rebuild
- **DO NOT use `index_project`** repeatedly - the file watcher (v0.2.0+) keeps indexes fresh automatically!
- **Use `goto_definition`** when you need to find where a function/variable is defined
- **Use `find_references`** when you need to find all usages of a symbol
- **Note:** If queries fail with "symbol not found", wait 2-5 seconds for the watcher to finish re-indexing, then retry

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
- **C++**: Full semantic navigation via scip-clang (requires `compile_commands.json`)

**C++ Requirements:**
- Must have `compile_commands.json` (compilation database)
- Generate with CMake: `cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..`
- Or use Bear for Make: `bear -- make`
- scip-clang auto-installs to `~/.local/bin`

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


# Agent Power Tools

**Stop using hand tools for code navigation. Powertools gives Claude IDE-level semantic navigation.**

Grep is a hand tool. Text search is guessing. Pattern matching finds false positives. **Powertools upgrades you to semantic navigation** — the same technology that powers VS Code's "Go to Definition" and "Find All References", now available to Claude Code.

---

## Core Principles

**Precision over pattern matching**
- Know the exact definition location, not just text matches
- Find semantic references, not string occurrences
- Navigate by meaning, not by regex

**Evidence over exploration**
- Jump directly to the source instead of hunting through files
- See all usages instantly instead of searching manually
- Refactor with confidence instead of hoping you found everything

**Speed over thoroughness**
- Index once, navigate infinitely
- Query in milliseconds, not minutes
- Let the compiler-grade indexers do the heavy lifting

**Mandatory when they're the best tool**
- If powertools can do it semantically, use powertools
- If you need cross-file navigation, ALWAYS use semantic tools
- If you're refactoring, ALWAYS preview before applying

---

## MANDATORY Usage Rules

### Code Navigation

**NEVER grep for definitions — ALWAYS use `goto_definition`**
```
❌ grep -r "function myFunc"        # Hand tool: finds comments, strings, false matches
✅ goto_definition("src/file.ts:42:10")  # Power tool: finds THE definition
```

**NEVER search manually for usages — ALWAYS use `find_references`**
```
❌ grep -r "myVariable"              # Hand tool: finds every string match
✅ find_references("myVariable")     # Power tool: finds semantic references only
```

**NEVER pattern match for code structures — ALWAYS use `search_ast`**
```
❌ grep -r "async function"          # Hand tool: misses arrow functions, class methods
✅ search_ast("(async_function) @f") # Power tool: finds ALL async functions by AST
```

### Refactoring Operations

**ALWAYS use `rename_symbol` for renaming across files**
```
❌ batch_replace("oldName", "newName")    # Dangerous: renames strings, comments, everything
✅ rename_symbol(file, line, col, "newName", preview=true)  # Safe: semantic-aware renaming
```

**ALWAYS preview before applying batch operations**
```
✅ batch_replace("pattern", "replacement", preview=true)  # See what changes first
✅ rename_symbol(..., preview=true)                       # Review before execution
✅ inline_variable(..., preview=true)                     # Verify correctness
```

**NEVER manually find-and-replace for refactoring**
- Use `rename_symbol` for renaming variables/functions/classes
- Use `inline_variable` for inlining constants
- Use `batch_replace` only for text patterns (URLs, typos, comments)

### Indexing

**ALWAYS let the file watcher handle re-indexing**
```
❌ index_project() after every file change   # Wastes time, watcher does this automatically
✅ Trust the watcher (starts automatically)  # It re-indexes changed files in background
✅ watcher_stop() before bulk operations     # Pause during git checkout, npm install, etc.
```

---

## Quick Start

### 1. MCP Integration

Create `.mcp.json` at your project root:

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

**Important:** File must be named `.mcp.json` (not `mcp_settings.json`) at project root. Commit to git for team collaboration.

### 2. Restart Claude Code

After creating `.mcp.json`, restart Claude Code. The file watcher starts automatically — indexes stay fresh as you code.

---

## Available Powertools

### Semantic Navigation Tools

**Core navigation tools (work across files):**

- **`index_project`** - Build semantic indexes for a project (run once)
  - Auto-installs language indexers (scip-typescript, scip-python, rust-analyzer, scip-clang)
  - Detects all languages automatically
  - Only needed: (1) First time on new project, OR (2) Watcher is stopped

- **`goto_definition`** - Find where a symbol is defined
  - Input: file:line:column location
  - Output: Exact definition location with file path
  - Works across files, modules, packages

- **`find_references`** - Find all references to a symbol
  - Input: Symbol name or file:line:column
  - Output: All semantic usages (not text matches)
  - Supports pagination (limit/offset for large results)

- **`search_ast`** - Search for code patterns using tree-sitter
  - Input: Tree-sitter query (e.g., `(function_declaration) @func`)
  - Output: Structured AST matches
  - Much more precise than regex

- **`list_functions`** - List all functions in a file/directory
  - Extracts function names, signatures, locations
  - Supports pagination

- **`list_classes`** - List all classes/structs/interfaces
  - Finds type definitions across codebase
  - Supports pagination

- **`project_stats`** - Get codebase statistics
  - File counts, line counts, languages detected

### Refactoring Tools (NEW in v0.4.0)

**Production-tested semantic refactoring:**

- **`rename_symbol`** - Rename symbols across entire codebase
  - **Semantic-aware**: Only renames the actual symbol, not strings/comments
  - **Cross-file**: Updates all references in all files
  - **Import-aware**: Updates import statements automatically
  - **Safe**: ALWAYS preview first (preview=true)
  - **Tested**: Production-validated on TypeScript, Rust, Python, C++ projects

  ```python
  # Example: Rename a function across entire codebase
  rename_symbol(
      file="src/utils.ts",
      line=42,
      column=10,
      new_name="processData",
      preview=true  # ALWAYS preview first!
  )
  ```

- **`inline_variable`** - Inline variables by replacing usages
  - **Safe**: Only works on const/immutable variables
  - **Smart**: Checks for side effects before inlining
  - **Preview**: ALWAYS preview first
  - **Limitations**: Currently single-file only (cross-file coming in v0.5.0)

  ```python
  # Example: Inline a constant
  inline_variable(
      file="src/app.ts",
      line=15,
      column=7,
      preview=true  # ALWAYS preview first!
  )
  ```

### Batch Operations Tools (Production-Ready v0.3.1)

**Text-based mass edits:**

- **`batch_replace`** - Replace text across multiple files using regex
  - **Use for**: Typos, URL updates, copyright notices, comment fixes
  - **Don't use for**: Renaming code (use `rename_symbol` instead)
  - **Features**: Regex capture groups ($1, $2), file glob filtering, risk assessment
  - **Safe**: Preview mode prevents accidental mass edits
  - **Tested**: Validated on TanStack Query (18 files, 138 changes), poetry-core (74 files, 589 changes)

  ```python
  # Example: Fix typos across codebase
  batch_replace(
      pattern="recieve",
      replacement="receive",
      preview=true  # ALWAYS preview first!
  )

  # Example: Update API URLs with capture groups
  batch_replace(
      pattern=r"api\.old\.com/([a-z]+)",
      replacement=r"api.new.com/$1",
      file_pattern="**/*.ts",
      preview=true
  )
  ```

### File Watcher Tools (v0.2.0)

**Automatic re-indexing (starts automatically on MCP server start):**

- **`watcher_start`** - Start automatic re-indexing
  - Usually NOT needed (starts automatically)
  - Use after manually stopping watcher

- **`watcher_stop`** - Pause automatic re-indexing
  - Use before: git checkout, npm install, mass file edits
  - Prevents re-index spam during bulk operations

- **`get_watcher_status`** - Check if watcher is running
  - Shows project root being monitored
  - Useful for debugging "symbol not found" errors

**Important:** The watcher starts automatically when MCP server starts. If you get "symbol not found" errors, wait 2-5 seconds for re-indexing to complete, then retry.

---

## Supported Languages

### Full Semantic Navigation (SCIP-based)

These languages have full cross-file semantic support via SCIP indexers:

- **TypeScript** - via scip-typescript (auto-installed)
- **JavaScript** - via scip-typescript (requires tsconfig.json with `allowJs: true`)
- **Python** - via scip-python (auto-installed)
- **Rust** - via rust-analyzer (auto-installed)
- **C++** - via scip-clang (auto-installed, requires `compile_commands.json`)

**C++ Requirements:**
- Must have `compile_commands.json` (compilation database)
- Generate with CMake: `cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..`
- Or use Bear for Make: `bear -- make`

**Python Known Issue:**
- scip-python has an upstream bug: test file references are not indexed
- Workaround: Manually update test files when using `rename_symbol`
- See README.md "Known Issues" section for details

### Tree-sitter Only Mode

- **Swift** (NEW in v0.4.0) - Tree-sitter-only mode
  - ✅ Local refactoring: inline_variable, extract method
  - ✅ Function/class finding
  - ✅ AST pattern search
  - ⚠️ Rename symbol: Single file only
  - ❌ Cross-file navigation: goto_definition, find_references (requires SCIP indexer - roadmap for v0.5.0)

**Why tree-sitter-only for Swift?** No official SCIP indexer exists for Swift yet. Tree-sitter enables local refactoring, but not cross-file semantic navigation. See `docs/SWIFT_LANGUAGE_SUPPORT_PLAN.md` for full roadmap.

---

## When to Use Which Tool

### Finding Code

| Task | Hand Tool (❌) | Power Tool (✅) | Why |
|------|---------------|----------------|-----|
| Find definition | `grep -r "function foo"` | `goto_definition(file:line:col)` | Semantic precision vs text matching |
| Find usages | `grep -r "myVar"` | `find_references("myVar")` | Filters out strings/comments |
| Find all functions | `grep -r "function "` | `list_functions()` | Finds ALL functions by AST |
| Find classes | `grep -r "class "` | `list_classes()` | Handles all class-like structures |
| Find async functions | `grep -r "async"` | `search_ast("(async_function) @f")` | Precise AST matching |

### Refactoring Code

| Task | Hand Tool (❌) | Power Tool (✅) | Why |
|------|---------------|----------------|-----|
| Rename variable | `batch_replace("old", "new")` | `rename_symbol(..., preview=true)` | Semantic-aware, safe |
| Inline constant | Manual copy-paste | `inline_variable(..., preview=true)` | Handles all usages |
| Fix typos | Manual search | `batch_replace("recieve", "receive", preview=true)` | Fast, safe with preview |
| Update URLs | Manual editing | `batch_replace(pattern, replacement, preview=true)` | Regex + preview |

### Managing Indexes

| Situation | What to Do | Why |
|-----------|------------|-----|
| First time on project | `index_project(auto_install=true)` | Build indexes once |
| File changed | Nothing (watcher handles it) | Auto re-indexing |
| Before `git checkout` | `watcher_stop()` | Avoid re-index spam |
| After bulk operation | `watcher_start()` | Resume monitoring |
| Symbol not found error | Wait 2-5 seconds, retry | Index rebuilding |

---

## Technical Reference

### CLI Commands (if not using MCP)

The powertools binary is located at: `./powertools-cli/target/release/powertools`

**Semantic Navigation:**
```bash
# Index project (auto-detects all languages)
./powertools-cli/target/release/powertools index --auto-install

# Index specific languages
./powertools-cli/target/release/powertools index --languages typescript python rust

# Go to definition
./powertools-cli/target/release/powertools definition src/file.ts:10:5 --format json

# Find references
./powertools-cli/target/release/powertools references myFunction --format json

# Include declarations
./powertools-cli/target/release/powertools references myFunction --include-declarations --format json
```

**Tree-sitter Pattern Matching:**
```bash
# Search AST patterns
./powertools-cli/target/release/powertools search-ast "(function_item) @func" -p src/

# List functions
./powertools-cli/target/release/powertools functions --format json

# List classes
./powertools-cli/target/release/powertools classes --format json

# Project stats
./powertools-cli/target/release/powertools stats
```

**Example Tree-sitter Patterns:**
- Rust: `(function_item) @func`
- TypeScript: `(function_declaration) @func`
- Python: `(function_definition) @func`
- Swift: `(function_declaration) @func`
- Async functions: `(async_function) @func`
- Classes: `(class_declaration) @class`

**Output Formats:**
All commands support `--format json` for structured data perfect for parsing.

### Pagination

All MCP tools that return lists support pagination to prevent token limit errors:

- `limit` parameter: Maximum results to return (default: 100)
- `offset` parameter: Number of results to skip (default: 0)
- Response includes: `count` (total), `has_more` (boolean), result data

**Example:** On a project with 1,438 functions, `list_functions(limit=100)` returns only 100 results.
