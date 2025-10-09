# Semantic Code Modification Tools - Implementation Plan

**Target:** v0.4.0
**Timeline:** 3-4 weeks
**Status:** Planning Phase

## Overview

This plan outlines the implementation of semantic code modification tools that leverage our existing SCIP indexes and tree-sitter AST infrastructure to perform **safe, automated refactorings** across entire codebases.

**Key Principle:** Every operation must be **preview-first** and **100% safe** - no missed references, no broken code.

---

## Phase 1: Foundation (Week 1)

### 1.1 Import/Export Analysis System

**Why First:** All refactorings that move/rename symbols need to update imports. This is foundational.

**What to Build:**
- `src/refactor/imports.rs` - Import statement parser and manipulator
- Detect import styles per language:
  - TypeScript/JavaScript: `import {X} from 'Y'`, `import * as X`, `require()`
  - Python: `from X import Y`, `import X as Y`
  - Rust: `use crate::X`, `use super::Y`
  - C++: `#include "X.h"`, `#include <X>`
- Find all import statements in a file (tree-sitter based)
- Add/remove/modify imports programmatically
- Handle different import formats (named, default, namespace, etc.)

**Deliverable:**
```rust
pub struct ImportAnalyzer {
    // Analyze imports in a file
    pub fn find_imports(file: &Path) -> Vec<ImportStatement>;

    // Modify imports
    pub fn add_import(file: &Path, symbol: &str, source: &str) -> Result<()>;
    pub fn remove_import(file: &Path, symbol: &str) -> Result<()>;
    pub fn update_import_path(file: &Path, old: &str, new: &str) -> Result<()>;
}
```

**Testing:**
- Parse imports from real-world files (TS, Rust, Python, C++)
- Add/remove imports and verify syntax
- Handle edge cases (duplicate imports, aliases, etc.)

---

### 1.2 Enhanced Preview System

**Why:** Refactorings affect multiple files. Need rich, multi-file previews.

**Extend `src/refactor/preview.rs`:**
- Multi-file diff support (currently single-file)
- Show import changes separately from code changes
- Summary view: "Will modify 15 files, update 47 references, add 3 imports"
- Color-coded changes (additions, deletions, modifications)
- Group changes by file for readability

**Deliverable:**
```rust
pub struct MultiFilePreview {
    pub files_affected: Vec<FilePreview>,
    pub summary: RefactoringSummary,
}

pub struct FilePreview {
    pub path: PathBuf,
    pub code_changes: Vec<PreviewChange>,
    pub import_changes: Vec<ImportChange>,
}

pub struct RefactoringSummary {
    pub total_files: usize,
    pub total_changes: usize,
    pub imports_added: usize,
    pub imports_removed: usize,
    pub risk_level: RiskLevel, // Safe, Caution, Dangerous
}
```

---

### 1.3 Refactoring Transaction System

**Why:** If we modify 50 files and file #42 fails, we need to rollback. Atomicity is critical.

**What to Build:**
- `src/refactor/transaction.rs` - Transactional file modification system
- Backup all files before modification
- Apply changes in memory first, write only if all succeed
- Rollback mechanism if any step fails
- Dry-run mode for testing

**Deliverable:**
```rust
pub struct RefactoringTransaction {
    // Start a transaction
    pub fn new() -> Self;

    // Stage changes (don't write yet)
    pub fn stage_change(&mut self, file: &Path, new_content: String);

    // Preview all staged changes
    pub fn preview(&self) -> MultiFilePreview;

    // Commit all changes atomically
    pub fn commit(self) -> Result<()>;

    // Rollback if needed
    pub fn rollback(self) -> Result<()>;
}
```

**Testing:**
- Simulate failures mid-transaction
- Verify rollback restores original state
- Test with 100+ file changes

---

## Phase 2: Core Refactorings (Week 2)

### 2.1 Rename Symbol 🎯 **HIGH VALUE, MEDIUM COMPLEXITY**

**Impact:** Massive. This is the #1 requested refactoring.

**Implementation:**

1. **Find the symbol definition** (SCIP `goto_definition`)
2. **Find all references** (SCIP `find_references`)
3. **Update each reference** (tree-sitter for precise location)
4. **Update imports** if symbol is exported/imported
5. **Handle special cases:**
   - Rename exports: update export statements
   - Rename class members: check for inheritance/overrides
   - Rename file-local symbols: no import changes needed

**Algorithm:**
```rust
pub struct RenameRefactoring {
    pub fn rename_symbol(
        symbol_location: &str,  // "file.ts:10:5"
        old_name: &str,
        new_name: &str,
    ) -> Result<RefactoringTransaction> {
        // 1. Validate new name (syntax check)
        // 2. Find definition via SCIP
        // 3. Find all references via SCIP
        // 4. For each reference:
        //    - Parse AST with tree-sitter
        //    - Replace identifier node
        // 5. Update imports/exports
        // 6. Return transaction with all changes
    }
}
```

**Edge Cases:**
- Symbol used in string literals (comments, docs) - skip or warn?
- Symbol conflicts with existing names in scope - error
- Partial matches (e.g., renaming `user` shouldn't affect `username`)

**CLI:**
```bash
powertools rename-symbol src/utils.ts:42:5 oldName newName --preview
```

**MCP Tool:**
```json
{
  "name": "rename_symbol",
  "parameters": {
    "symbol_location": "src/utils.ts:42:5",
    "old_name": "getUserById",
    "new_name": "fetchUser",
    "preview": true
  }
}
```

**Testing:**
- Rename function across 10+ files
- Rename class property
- Rename exported symbol with imports
- Conflict detection

---

### 2.2 Inline Variable 🎯 **MEDIUM VALUE, EASY-MEDIUM COMPLEXITY**

**Impact:** Good for cleanup and simplification.

**Implementation:**

1. **Find variable definition** (SCIP `goto_definition`)
2. **Get variable value** (tree-sitter AST extraction)
3. **Find all usages** (SCIP `find_references`)
4. **Replace each usage with the value**
5. **Remove variable declaration**

**Algorithm:**
```rust
pub struct InlineVariableRefactoring {
    pub fn inline_variable(
        variable_location: &str,  // "file.ts:10:5"
    ) -> Result<RefactoringTransaction> {
        // 1. Find variable definition (SCIP)
        // 2. Extract initializer value (tree-sitter)
        // 3. Validate it's a simple value (not a complex expression)
        // 4. Find all references (SCIP)
        // 5. Replace each reference with the value
        // 6. Remove the variable declaration
        // 7. Return transaction
    }
}
```

**Edge Cases:**
- Multi-line values need parentheses
- Side effects (e.g., `const x = fn()`) - warn/error
- Variable reassigned - error (can't inline mutable vars)

**CLI:**
```bash
powertools inline-variable src/app.ts:15:10 --preview
```

**Testing:**
- Inline simple constant
- Inline string/number/boolean
- Error on function call inline
- Error on reassigned variable

---

### 2.3 Move Symbol 🎯 **HIGH VALUE, MEDIUM COMPLEXITY**

**Impact:** Essential for organizing code.

**Implementation:**

1. **Find symbol definition** (tree-sitter to extract entire definition)
2. **Find all references** (SCIP)
3. **Extract symbol AST** (tree-sitter)
4. **Remove from source file**
5. **Add to destination file**
6. **Update all imports** in referencing files
7. **Add import in destination file** if needed

**Algorithm:**
```rust
pub struct MoveSymbolRefactoring {
    pub fn move_symbol(
        symbol_location: &str,  // "src/old.ts:20:0"
        destination_file: &Path,
    ) -> Result<RefactoringTransaction> {
        // 1. Parse source file AST (tree-sitter)
        // 2. Find and extract symbol definition
        // 3. Find all references (SCIP)
        // 4. Remove from source file
        // 5. Insert into destination file
        // 6. Update imports in all referencing files
        // 7. Return transaction
    }
}
```

**Edge Cases:**
- Symbol depends on other local symbols - error or move dependencies too
- Circular dependencies - detect and error
- File doesn't exist - create or error?
- Multiple symbols with same name in destination - conflict

**CLI:**
```bash
powertools move-symbol src/utils.ts:getUserById src/api/users.ts --preview
```

**Testing:**
- Move function between files
- Move class with dependencies
- Update 10+ import statements

---

## Phase 3: Advanced Refactorings (Week 3)

### 3.1 Extract Method/Function 🎯 **HIGHEST VALUE, HARD COMPLEXITY**

**Impact:** Huge. Most complex but most valuable.

**Implementation:**

1. **Parse code range** (tree-sitter)
2. **Analyze variable usage:**
   - Variables used but defined outside range → parameters
   - Variables defined inside range → local variables
   - Variables modified inside range and used outside → return values
3. **Infer return type** (from usage context)
4. **Generate function signature**
5. **Extract code to new function**
6. **Replace original code with function call**

**Algorithm:**
```rust
pub struct ExtractMethodRefactoring {
    pub fn extract_method(
        file: &Path,
        start_line: usize,
        end_line: usize,
        function_name: &str,
    ) -> Result<RefactoringTransaction> {
        // 1. Parse file AST
        // 2. Find nodes in range [start_line, end_line]
        // 3. Analyze variable usage:
        //    - Scope analysis (which vars are in scope)
        //    - Data flow (which vars are read/written)
        // 4. Determine parameters (vars used, not defined in range)
        // 5. Determine return type (vars modified, used after range)
        // 6. Generate function signature
        // 7. Extract code into new function
        // 8. Replace range with function call
        // 9. Return transaction
    }
}
```

**Edge Cases:**
- Multiple return paths - need to preserve control flow
- Early returns in extracted code - handle correctly
- Exception handling - preserve try/catch semantics
- Async code - mark function as async if needed

**CLI:**
```bash
powertools extract-method src/app.ts:20-45 validateUser --preview
```

**Challenges:**
- **Scope analysis:** Need to track variable scopes precisely
- **Control flow:** Preserve break/continue/return semantics
- **Type inference:** Infer parameter and return types
- **Language-specific:** Different syntax per language

**Testing:**
- Extract simple function (no params, no return)
- Extract with parameters
- Extract with return value
- Extract with multiple returns
- Extract async code

---

### 3.2 Inline Function 🎯 **MEDIUM VALUE, MEDIUM-HARD COMPLEXITY**

**Impact:** Opposite of extract - useful for over-abstracted code.

**Implementation:**

1. **Find function definition** (SCIP + tree-sitter)
2. **Get function body** (tree-sitter)
3. **Find all call sites** (SCIP `find_references`)
4. **For each call site:**
   - Map arguments to parameters
   - Substitute parameters in function body
   - Insert body at call site
5. **Remove function definition**

**Algorithm:**
```rust
pub struct InlineFunctionRefactoring {
    pub fn inline_function(
        function_location: &str,
    ) -> Result<RefactoringTransaction> {
        // 1. Find function definition (SCIP + tree-sitter)
        // 2. Extract function body
        // 3. Find all call sites (SCIP)
        // 4. For each call:
        //    - Parse call arguments
        //    - Substitute params with args in body
        //    - Replace call with substituted body
        // 5. Remove function definition
        // 6. Return transaction
    }
}
```

**Edge Cases:**
- Recursive functions - error
- Function with side effects - warn
- Multiple return statements - need block wrapper
- Variable name conflicts - rename with unique suffix

---

### 3.3 Change Signature 🎯 **HIGH VALUE, HARD COMPLEXITY**

**Impact:** Essential for API evolution.

**Implementation:**

1. **Find function definition** (SCIP + tree-sitter)
2. **Parse new signature** (parameter changes)
3. **Find all call sites** (SCIP `find_references`)
4. **Update function definition**
5. **Update each call site:**
   - Add new parameters (with default values or placeholders)
   - Remove deleted parameters
   - Reorder parameters

**Algorithm:**
```rust
pub struct ChangeSignatureRefactoring {
    pub fn change_signature(
        function_location: &str,
        changes: SignatureChanges,
    ) -> Result<RefactoringTransaction> {
        // 1. Find function definition
        // 2. Find all call sites (SCIP)
        // 3. Update definition signature
        // 4. For each call site:
        //    - Reorder arguments
        //    - Add defaults for new params
        //    - Remove deleted params
        // 5. Return transaction
    }
}

pub struct SignatureChanges {
    pub add_params: Vec<(String, String, Option<String>)>, // (name, type, default)
    pub remove_params: Vec<String>,
    pub reorder: Option<Vec<usize>>,
}
```

**Edge Cases:**
- New required parameters - need placeholder or error
- Overloaded functions - handle each overload
- Callback parameters - need to recurse

---

## Phase 4: Polish & Integration (Week 4)

### 4.1 MCP Tool Integration

Add all refactorings as MCP tools:

```rust
// In src/mcp/tools.rs
async fn rename_symbol(&self, params: RenameSymbolParams) -> Result<CallToolResult>;
async fn inline_variable(&self, params: InlineVariableParams) -> Result<CallToolResult>;
async fn inline_function(&self, params: InlineFunctionParams) -> Result<CallToolResult>;
async fn move_symbol(&self, params: MoveSymbolParams) -> Result<CallToolResult>;
async fn extract_method(&self, params: ExtractMethodParams) -> Result<CallToolResult>;
async fn change_signature(&self, params: ChangeSignatureParams) -> Result<CallToolResult>;
```

All tools default to `preview: true` for safety.

---

### 4.2 CLI Commands

Add commands to `src/commands/`:

```bash
powertools rename-symbol <location> <old> <new> [--preview]
powertools inline-variable <location> [--preview]
powertools inline-function <location> [--preview]
powertools move-symbol <location> <destination> [--preview]
powertools extract-method <file> <start>-<end> <name> [--preview]
powertools change-signature <location> <changes> [--preview]
```

---

### 4.3 Documentation

Update `CLAUDE.md` with:
- When to use each refactoring
- Safety guarantees
- Preview-first workflow
- Examples for AI agents

Update `README.md` with:
- New semantic refactoring section
- Example use cases
- Performance characteristics

---

### 4.4 Testing Strategy

**Unit Tests:**
- Each refactoring operation in isolation
- Edge cases and error conditions
- Language-specific behavior

**Integration Tests:**
- Multi-file refactorings on real codebases
- Transaction rollback scenarios
- Import/export updates

**Performance Tests:**
- Large codebases (1000+ files)
- Symbols with 100+ references
- Benchmark against manual Edit operations

---

## Implementation Order (Recommended)

### Week 1: Foundation
1. ✅ Import/Export analyzer (3 days)
2. ✅ Enhanced preview system (2 days)
3. ✅ Transaction system (2 days)

### Week 2: Quick Wins
1. ✅ **Rename Symbol** (4 days) - Highest value, medium complexity
2. ✅ **Inline Variable** (2 days) - Easy win
3. ✅ Move Symbol (1 day) - Build on rename

### Week 3: Advanced
1. ✅ **Extract Method** (5 days) - Most complex, highest value
2. ✅ Inline Function (2 days) - Similar to inline variable

### Week 4: Polish
1. ✅ Change Signature (3 days)
2. ✅ MCP integration (1 day)
3. ✅ Documentation (1 day)
4. ✅ Testing & bug fixes (2 days)

---

## Success Criteria

**MVP (Minimum Viable Product):**
- ✅ Rename Symbol working for TypeScript/Rust
- ✅ Preview system shows all changes
- ✅ Atomic transactions (all-or-nothing)
- ✅ MCP tool integration
- ✅ Zero broken code (100% safe refactorings)

**V1 (Full Release):**
- All 6 refactorings implemented
- Support for TS, Rust, Python, C++
- Comprehensive tests (90%+ coverage)
- Documentation for AI agents
- Performance: <2s for 100-file rename

---

## Risk Assessment

### High Risk Areas:
1. **Extract Method scope analysis** - Complex, language-dependent
2. **Import/Export updates** - Many edge cases per language
3. **Transaction atomicity** - File system failures mid-operation

### Mitigation:
1. Start with TypeScript/Rust (simpler than Python/C++)
2. Comprehensive test suite with real-world codebases
3. Extensive preview testing before applying changes
4. Rollback mechanism for all operations

---

## Dependencies

**Already Have:**
- ✅ SCIP indexes (`goto_definition`, `find_references`)
- ✅ Tree-sitter AST parsing (all languages)
- ✅ Preview system foundation (`src/refactor/preview.rs`)
- ✅ Batch operation infrastructure (`src/refactor/replacer.rs`)
- ✅ File watcher for auto re-indexing

**Need to Add:**
- Import/export analysis
- Scope analysis (variable binding)
- Control flow analysis (for extract method)
- Type inference (for signature generation)

**External Crates (Potential):**
- `tree-sitter-graph` - For scope analysis
- `similar` - Better diff algorithm for previews
- None needed initially - build with what we have

---

## Competitive Analysis

**Existing Tools:**
- **rust-analyzer** - Has rename, but IDE-only
- **typescript-language-server** - Has refactorings, but LSP protocol
- **rope** (Python) - Refactoring library, but Python-only

**Our Advantage:**
1. **Multi-language** - Works across TS, Rust, Python, C++
2. **AI-friendly** - MCP integration, JSON output
3. **Batch-aware** - Can combine with batch operations
4. **Standalone** - No IDE required, CLI + MCP

---

## Future Enhancements (v0.5.0+)

- **Extract Component** (React/Vue)
- **Convert Class to Hooks**
- **Add Null Checks** (automated safety)
- **Split File** (break monoliths)
- **Merge Files**
- **Safe Delete** (find + remove all references)

---

## Notes

- **Safety First:** Every operation MUST have preview mode
- **Atomicity:** All-or-nothing - no partial refactorings
- **Transparency:** Show every change in preview
- **Language Parity:** Start with TS/Rust, expand to Python/C++
- **Testing:** Real codebases, not toy examples

---

## Questions for Discussion

1. **Language Priority:** Start with TypeScript only, or TS + Rust in parallel?
2. **Extract Method Complexity:** Should we support complex control flow in v1, or save for v2?
3. **Change Signature:** Should we auto-infer default values, or require user input?
4. **Error Handling:** Fail-fast or continue-on-error with warnings?

---

## Appendix: Code Structure

```
src/refactor/
├── mod.rs              # Public API
├── preview.rs          # ✅ Preview system (exists)
├── replacer.rs         # ✅ Batch replacer (exists)
├── transaction.rs      # NEW: Atomic transactions
├── imports.rs          # NEW: Import/export analysis
├── scope.rs            # NEW: Scope analysis for extract method
├── rename.rs           # NEW: Rename symbol
├── inline.rs           # NEW: Inline variable/function
├── move_symbol.rs      # NEW: Move symbol between files
├── extract.rs          # NEW: Extract method
├── signature.rs        # NEW: Change signature
└── tests/              # Comprehensive tests
```

**Estimated Lines of Code:** ~5,000-7,000 (with tests)

**Estimated Complexity:** Medium-Hard

**Estimated Value:** 🔥🔥🔥 Extremely High
