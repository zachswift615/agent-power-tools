# Agent Power Tools - Feature Wishlist

This document captures future feature ideas based on real AI agent workflow needs.

## High-Impact Tools

### 1. Semantic Code Modification Tools 🔥

**Impact:** Game-changer for safe refactoring

**Tools:**
- **Extract Method/Function** - Select code range, automatically extract with proper parameters/return types
- **Rename Symbol** - True semantic rename across entire codebase (not just text replace)
- **Inline Variable/Function** - Safely inline a definition into its usage sites
- **Change Signature** - Add/remove/reorder parameters and update all call sites
- **Move Symbol** - Move functions/classes between files and auto-update imports

**Why:** Currently done manually with Edit tool, which is error-prone. One missed reference breaks the code. With SCIP indexes, these could be 100% safe.

**Status:** 🟡 Planned for v0.3.0
**Complexity:** Medium-Hard
**Prerequisites:** SCIP indexes, tree-sitter AST manipulation

---

### 2. Batch File Operations 🎯

**Impact:** Immediate productivity boost for repetitive tasks

**Tools:**
- ✅ **Regex Replace Across Files** - Like VSCode's "Replace in Files" but with preview (**IMPLEMENTED in v0.3.0!**)
- **Multi-file Edit** - Apply same Edit pattern across multiple files in one operation (AST-based)
- **Template Expansion** - Generate files from templates (e.g., create React component with test/style/story files)
- **Bulk Rename Files** - Rename files and auto-update all imports

**Why:** Many tasks involve repetitive edits across files. Currently requires loops with individual Edit calls - slow and verbose.

**Example Use Cases:**
- Add null checks across 50 files
- Update API endpoint URLs in entire codebase
- Convert all class components to hooks
- Add TypeScript types to JavaScript files

**Status:** 🟢 **Partially Implemented** (v0.3.0 - batch_replace done!)
**Complexity:** Medium
**Prerequisites:** ✅ Pattern matching, ✅ preview/confirmation system

---

### 3. Code Understanding Tools 🧠

**Impact:** Answers "why" and "how" questions faster

**Tools:**
- **Call Hierarchy** - Show all callers → function → all callees (tree view)
- **Type Hierarchy** - Show inheritance/implementation tree
- **Data Flow Analysis** - "How does this variable flow through the code?"
- **Dependency Graph** - Visual/textual graph of module dependencies
- **Dead Code Detection** - Find unused exports, functions, variables

**Why:** These answer architectural questions that come up constantly. Right now requires manual grep/search/read cycles.

**Status:** 🟢 Easy wins - some can be built with existing tools
**Complexity:** Easy-Medium
**Prerequisites:** SCIP indexes (already have this!)

**Quick Wins:**
- Call Hierarchy: `find_references` already gives us callers!
- Dead Code Detection: symbols with 0 references
- Type Hierarchy: SCIP has this data

---

### 4. Test Generation & Execution ✅

**Impact:** Improves code quality, reduces manual testing

**Tools:**
- **Generate Test Template** - Scaffold test for a function (detect framework, generate boilerplate)
- **Run Specific Test** - `run_test("path/to/test.ts", "test name")` with live output
- **Coverage Report** - Show which lines are covered by tests
- **Test Impact Analysis** - "Which tests should I run after changing this file?"

**Why:** Testing is tedious and often skipped. Making it one tool call away would improve quality.

**Status:** 🔴 Future
**Complexity:** Medium-Hard
**Prerequisites:** Test framework detection, execution environment

---

### 5. Intelligent Code Search 🔍

**Impact:** Understand large codebases faster

**Tools:**
- **Structural Search** - "Find all functions that call X and don't handle errors"
- **Similar Code Finder** - "Find code similar to this snippet" (for refactoring duplicates)
- **Cross-Repository Search** - Search across multiple projects
- **Temporal Search** - "Show me all recent changes to files touching authentication"

**Why:** Current search is either text-based (grep) or too basic. These would help understand complex codebases.

**Status:** 🔴 Future
**Complexity:** Hard
**Prerequisites:** Advanced AST analysis, ML similarity (potentially)

---

### 6. Safe Refactoring Workflows 🛡️

**Impact:** Complex multi-step operations made safe

**Tools:**
- **Extract Component** (React/Vue) - Move JSX to new component with prop inference
- **Split File** - Break large file into modules, preserve all imports
- **Convert Class to Hooks** - Automated class → functional component migration
- **Add Null Checks** - Analyze and add null/undefined checks where needed

**Why:** Complex multi-step operations that need semantic understanding. One wrong step breaks everything.

**Status:** 🔴 Future (v0.4.0+)
**Complexity:** Hard
**Prerequisites:** Framework-specific AST understanding, type inference

---

## Implementation Roadmap

### v0.3.0 - Semantic Refactoring (Target: Q1 2026)

**Phase 1: Foundation**
- [ ] Rename Symbol (across codebase)
- [ ] Import/Export analyzer and updater
- [ ] Preview/diff system for safe refactoring

**Phase 2: Core Refactorings**
- [ ] Extract Method/Function
- [ ] Inline Variable
- [ ] Inline Function
- [ ] Move Symbol

**Phase 3: Batch Operations**
- [ ] Multi-file Edit with preview
- [ ] Regex Replace Across Files
- [ ] Bulk File Rename

### v0.4.0 - Code Understanding (Target: Q2 2026)
- [ ] Call Hierarchy
- [ ] Type Hierarchy
- [ ] Dead Code Detection
- [ ] Dependency Graph
- [ ] Data Flow Analysis

### v0.5.0+ - Advanced Features (Future)
- [ ] Test generation and execution
- [ ] Structural code search
- [ ] Framework-specific refactorings

---

## Why Powertools is Perfect for This

**Unique Position:**
1. ✅ Already have SCIP indexes (semantic understanding)
2. ✅ Already have tree-sitter (AST manipulation)
3. ✅ Already have MCP integration (tool discoverability)
4. ✅ Already have file watcher (automatic updates)

**What we need to add:**
- Code modification layer (apply edits at semantic locations)
- Import/export graph analysis
- Preview/confirmation system for batch operations
- Type system integration (for signature changes)

**Competitive Advantage:**
- Most tools do semantic *analysis* (LSP servers, ctags)
- Few do semantic *modification* safely
- We can bridge the gap with SCIP + tree-sitter

---

## Proposed CLI Interface

### Semantic Refactoring
```bash
# Rename symbol across entire codebase
powertools rename-symbol OldName NewName --preview --format json

# Extract method from code range
powertools extract-method src/file.ts:10-50 --name "validateUser" --preview

# Inline variable
powertools inline-variable src/file.ts:42:10 --preview

# Change function signature
powertools change-signature src/utils.ts:getUserById \
  --add-param "includeDeleted:boolean=false" \
  --preview

# Move symbol to different file
powertools move-symbol MyClass src/old.ts src/new.ts --update-imports
```

### Batch Operations
```bash
# Apply edit pattern across files
powertools batch-edit \
  --pattern "if (user.{field}) {" \
  --replace "if (user?.{field}) {" \
  --files "src/**/*.ts" \
  --preview

# Regex replace in files
powertools batch-replace \
  --regex "console\.log\((.*?)\)" \
  --replace "logger.debug(\$1)" \
  --files "src/**/*.ts" \
  --preview

# Bulk rename files
powertools batch-rename \
  --pattern "*.spec.ts" \
  --replace "*.test.ts" \
  --update-imports
```

### Code Understanding
```bash
# Show call hierarchy
powertools call-hierarchy src/utils.ts:getUserById --depth 3 --format json

# Find dead code
powertools find-dead-code --format json

# Show type hierarchy
powertools type-hierarchy MyInterface --format json

# Dependency graph
powertools dep-graph src/services --format dot | dot -Tpng > deps.png
```

---

## MCP Tool Interface

All tools would be available as MCP tools in Claude Code:

```typescript
// Semantic refactoring
rename_symbol(old_name: string, new_name: string, preview?: boolean)
extract_method(file_location: string, range: Range, method_name: string)
inline_variable(file_location: string)
change_signature(function_location: string, changes: SignatureChange[])
move_symbol(symbol: string, from_file: string, to_file: string)

// Batch operations
batch_edit(pattern: string, replacement: string, files: string[], preview?: boolean)
batch_replace(regex: string, replacement: string, files: string[], preview?: boolean)
bulk_rename_files(pattern: string, replacement: string, update_imports?: boolean)

// Code understanding
call_hierarchy(symbol_location: string, depth?: number)
find_dead_code(path?: string)
type_hierarchy(type_name: string)
dependency_graph(path: string, format?: 'json' | 'dot')
```

---

## Implementation Complexity Estimates

### Easy (Weekend Project)
- **Call Hierarchy** - Already have find_references!
- **Dead Code Detection** - Find symbols with 0 references
- **Batch Rename Files** - File ops + update imports

### Medium (1-2 Weeks)
- **Rename Symbol** - find_references + modify each location + update imports
- **Inline Variable** - goto_definition + find_references + text manipulation
- **Batch Edit** - File operations + preview/confirmation system
- **Move Symbol** - Complex but well-defined problem

### Hard (2-4 Weeks)
- **Extract Method** - Parameter inference + scope analysis + code generation
- **Change Signature** - Type system integration + call site analysis
- **Data Flow Analysis** - Requires control flow graph

---

## Notes

- Focus on **safe** operations with preview/confirmation
- All operations should support `--preview` flag
- Use SCIP for semantic understanding, tree-sitter for AST manipulation
- MCP tools should return structured JSON for AI consumption
- Consider adding `--dry-run` flag for testing
- Add comprehensive tests for each refactoring operation

---

## Community Feedback

Want to suggest a feature? Open an issue on GitHub with the label `enhancement` and tag it `wishlist`.

Priority is determined by:
1. Impact on AI agent workflows
2. Implementation complexity vs. value
3. Synergy with existing powertools features
4. Community demand
