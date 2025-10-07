# Power Tools Examples

## Tree-sitter Query Patterns

### Basic Patterns

#### Find all functions
```bash
# Rust
powertools search-ast "(function_item) @func"

# TypeScript/JavaScript
powertools search-ast "(function_declaration) @func"

# Python
powertools search-ast "(function_definition) @func"
```

#### Find specific function by name
```bash
# Find function named "processData"
powertools search-ast '(function_declaration name: (identifier) @name (#eq? @name "processData"))'
```

#### Find all classes
```bash
# TypeScript/JavaScript
powertools search-ast "(class_declaration) @class"

# Python
powertools search-ast "(class_definition) @class"

# Rust (structs)
powertools search-ast "(struct_item) @struct"
```

### Advanced Patterns

#### Find async functions
```bash
# JavaScript/TypeScript
powertools search-ast "(function_declaration async: (async)) @async_func"

# Rust
powertools search-ast '(function_item (function_modifiers (async))) @async_func'
```

#### Find functions with specific parameters
```bash
# Find functions taking a string parameter (TypeScript)
powertools search-ast '(function_declaration
  parameters: (formal_parameters
    (required_parameter
      type: (type_annotation (predefined_type) @type)))
  (#eq? @type "string"))'
```

#### Find all imports
```bash
# JavaScript/TypeScript
powertools search-ast "(import_statement) @import"

# Python
powertools search-ast "[
  (import_statement)
  (import_from_statement)
] @import"

# Rust
powertools search-ast "(use_declaration) @use"
```

#### Find all TODO comments
```bash
powertools search-ast '(comment) @comment (#match? @comment "TODO|FIXME|HACK")'
```

## Claude Code Usage Examples

### Example 1: Understanding a New Codebase

```bash
# Get project overview
powertools stats --detailed

# Find main entry points
powertools search-ast '(function_declaration name: (identifier) @name (#match? @name "main|init|start"))'

# List all public functions
powertools functions

# Find all classes/types
powertools classes
```

### Example 2: Finding Symbol Usage

```bash
# Find where a function is defined
powertools search-ast '(function_declaration name: (identifier) @name (#eq? @name "authenticate"))'

# Find where a class is used (basic text search for now)
powertools search-ast '(identifier) @id (#eq? @id "UserService")'
```

### Example 3: Code Analysis

```bash
# Find all error handling
powertools search-ast "[
  (try_statement)
  (catch_clause)
] @error_handling"

# Find all async operations
powertools search-ast "[
  (await_expression)
  (async_function)
] @async"

# Find all test functions
powertools search-ast '(function_declaration name: (identifier) @name (#match? @name "^test"))'
```

## JSON Output Examples

### Function Search Result
```json
{
  "functions": [
    {
      "name": "processData",
      "kind": "function",
      "location": {
        "file_path": "src/processor.ts",
        "line": 15,
        "column": 1
      },
      "signature": "processData(input: string): Promise<Result>",
      "is_public": true
    }
  ]
}
```

### Pattern Search Result
```json
{
  "results": [
    {
      "location": {
        "file_path": "src/auth.ts",
        "line": 42,
        "column": 5
      },
      "matched_text": "async function authenticate(user: User)",
      "node_type": "function_declaration",
      "language": "typescript",
      "context_before": "export class AuthService {",
      "context_after": "  const token = await generateToken(user);"
    }
  ]
}
```

### Project Statistics
```json
{
  "total_files": 150,
  "total_symbols": 1250,
  "languages": [
    ["typescript", 100],
    ["javascript", 30],
    ["python", 20]
  ],
  "index_time_ms": 500,
  "index_size_bytes": 2048000
}
```

## Integration with Claude Code

### In .claude/CLAUDE.md
```markdown
## Power Tools Commands

Use these commands to navigate code:

- `bash .claude/commands/search-ast.sh "pattern"` - Search for AST patterns
- `bash .claude/commands/find-functions.sh` - List all functions
- `bash .claude/commands/find-classes.sh` - List all classes
- `bash .claude/commands/go-to-definition.sh "file:line:col"` - Jump to definition
```

### Example Claude Code Session
```
User: Find all the async functions in this project

Claude: I'll search for all async functions in the project using the power tools.

[Runs: bash .claude/commands/search-ast.sh "(async_function) @func"]

I found 15 async functions in your project:
- `processData` in src/processor.ts:15
- `fetchUser` in src/api/users.ts:23
- `validateInput` in src/validators.ts:45
...

User: Show me the processData function

Claude: Let me find the exact location of the processData function.

[Runs: powertools search-ast '(function_declaration name: (identifier) @name (#eq? @name "processData"))']

Found it! The processData function is at src/processor.ts:15:1

[Reads file and shows the function]
```

## Tips and Tricks

### 1. Use Wildcards in Patterns
```bash
# Find functions starting with "handle"
powertools search-ast '(function_declaration name: (identifier) @name (#match? @name "^handle"))'

# Find functions ending with "Controller"
powertools search-ast '(class_declaration name: (identifier) @name (#match? @name "Controller$"))'
```

### 2. Combine Multiple Node Types
```bash
# Find all function-like constructs
powertools search-ast "[
  (function_declaration)
  (arrow_function)
  (method_definition)
] @function"
```

### 3. Use Capture Groups
```bash
# Capture both function name and its parameters
powertools search-ast '(function_declaration
  name: (identifier) @func_name
  parameters: (formal_parameters) @params)'
```

### 4. Filter by File Extension
```bash
# Search only in TypeScript files
powertools search-ast "pattern" -e .ts -e .tsx

# Search only in Python files
powertools search-ast "pattern" -e .py
```

### 5. Limit Results for Large Codebases
```bash
# Get only first 10 matches
powertools search-ast "pattern" -m 10
```