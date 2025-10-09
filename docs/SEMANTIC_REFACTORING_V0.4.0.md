# Semantic Refactoring Tools - v0.4.0 Implementation

**Status:** 🟡 In Progress - Foundation Phase
**Target Release:** Q1 2026
**Timeline:** 2-3 weeks
**Started:** 2025-10-09

---

## Executive Summary

We're building the world's first **multi-language semantic refactoring tool designed for AI agents**. Unlike traditional IDE refactoring tools, this will work standalone via CLI and MCP (Model Context Protocol), enabling AI coding assistants like Claude Code to perform safe, automated refactorings across entire codebases.

**Key Differentiators:**
- ✅ **Multi-language:** TypeScript, JavaScript, Python, Rust, C++
- ✅ **AI-friendly:** MCP integration with JSON output
- ✅ **100% safe:** Uses SCIP indexes to prevent missed references
- ✅ **Preview-first:** Always show changes before applying
- ✅ **Atomic:** All-or-nothing transactions with rollback

---

## Implementation Plan

### Week 1: Foundation (Days 1-7)

**Goal:** Build the infrastructure needed for all refactorings

#### 1.1 Import/Export Analysis System
**Files:** `src/refactor/imports/`

Build language-specific import analyzers:
- **TypeScript/JavaScript:** Use tree-sitter to parse `import`/`export` statements
- **Python:** Use rustpython-parser to parse `import`/`from...import`
- **Rust:** Use syn crate to parse `use` statements
- **C++:** Use tree-sitter to parse `#include` directives

**Deliverable:**
```rust
pub struct ImportStatement {
    pub source: String,        // e.g., "react", "./utils"
    pub symbols: Vec<String>,  // e.g., ["useState", "useEffect"]
    pub location: Location,    // File location
    pub kind: ImportKind,      // Named, Default, Namespace, etc.
}

pub trait ImportAnalyzer {
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>>;
    fn add_import(&self, file: &Path, import: &ImportStatement) -> Result<String>;
    fn remove_import(&self, file: &Path, symbol: &str) -> Result<String>;
    fn update_import_path(&self, file: &Path, old: &str, new: &str) -> Result<String>;
}
```

**Time:** 2-3 days

#### 1.2 Enhanced Multi-File Preview System
**Files:** `src/refactor/preview.rs` (extend existing)

Extend the preview system built in v0.3.0 to support:
- Multi-file changes (currently single-file)
- Import changes separate from code changes
- Summary statistics (files affected, references updated, imports added/removed)
- Risk assessment (Safe, Caution, Dangerous)

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
    pub risk_level: RiskLevel,
}
```

**Time:** 2 days

#### 1.3 Transactional Refactoring System
**Files:** `src/refactor/transaction.rs`

Build atomic transaction system to ensure all-or-nothing refactorings:
- Stage changes in memory before writing
- Validate all changes before committing
- Rollback mechanism if any step fails
- Dry-run mode for testing

**Deliverable:**
```rust
pub struct RefactoringTransaction {
    staged_changes: HashMap<PathBuf, String>,
    backups: HashMap<PathBuf, String>,
}

impl RefactoringTransaction {
    pub fn new() -> Self;
    pub fn stage_change(&mut self, file: &Path, new_content: String);
    pub fn preview(&self) -> MultiFilePreview;
    pub fn commit(self) -> Result<()>;
    pub fn rollback(self) -> Result<()>;
}
```

**Time:** 2 days

**Week 1 Total:** ~6-7 days

---

### Week 2: Core Refactorings (Days 8-14)

**Goal:** Implement the most valuable, frequently-used refactorings

#### 2.1 Rename Symbol 🎯 **PRIORITY #1**
**Files:** `src/refactor/rename.rs`, `src/commands/rename_symbol.rs`

**Algorithm:**
1. Find symbol definition using SCIP `goto_definition`
2. Find all references using SCIP `find_references`
3. For each file with references:
   - Parse with language-specific parser (tree-sitter/syn/rustpython)
   - Replace identifier at exact location
   - Update imports if symbol is exported
4. Return transaction with all changes

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

**Edge Cases:**
- Symbol conflicts with existing names → error
- Partial matches (e.g., `user` vs `username`) → precise AST matching
- String literals/comments containing symbol → skip or warn

**Time:** 4 days

#### 2.2 Inline Variable
**Files:** `src/refactor/inline.rs`, `src/commands/inline_variable.rs`

**Algorithm:**
1. Find variable definition (SCIP + tree-sitter)
2. Extract initializer value from AST
3. Find all usages (SCIP)
4. Replace each usage with the value
5. Remove variable declaration

**CLI:**
```bash
powertools inline-variable src/app.ts:15:10 --preview
```

**Edge Cases:**
- Side effects in initializer (e.g., `const x = fn()`) → error/warn
- Variable reassigned → error (can't inline mutable)
- Multi-line values → add parentheses

**Time:** 2 days

#### 2.3 Move Symbol
**Files:** `src/refactor/move.rs`, `src/commands/move_symbol.rs`

**Algorithm:**
1. Extract symbol definition from source file (tree-sitter/syn)
2. Find all references (SCIP)
3. Remove from source file
4. Insert into destination file
5. Update all imports in referencing files

**CLI:**
```bash
powertools move-symbol src/utils.ts:getUserById src/api/users.ts --preview
```

**Edge Cases:**
- Symbol has dependencies on other local symbols → error or move together
- Circular dependencies → detect and error
- Destination file doesn't exist → create or error (user choice)

**Time:** 1 day

**Week 2 Total:** ~7 days

---

### Week 3: Advanced Refactorings (Days 15-21)

**Goal:** Implement complex, high-value refactorings

#### 3.1 Extract Method/Function 🔥 **HIGHEST VALUE**
**Files:** `src/refactor/extract.rs`, `src/commands/extract_method.rs`

**Algorithm (Complex):**
1. Parse code range into AST nodes
2. **Scope analysis:**
   - Variables used but defined outside → parameters
   - Variables defined inside → local variables
   - Variables modified inside and used outside → return values
3. Infer parameter types from usage
4. Infer return type from context
5. Generate function signature
6. Extract code into new function
7. Replace original code with function call

**CLI:**
```bash
powertools extract-method src/app.ts:20-45 validateUser --preview
```

**Edge Cases:**
- Multiple return paths → preserve control flow
- Early returns → handle correctly
- Exception handling → preserve try/catch
- Async code → mark function as async

**Time:** 5 days (most complex)

#### 3.2 Inline Function
**Files:** `src/refactor/inline.rs` (extend)

**Algorithm:**
1. Find function definition (SCIP + parser)
2. Extract function body
3. Find all call sites (SCIP)
4. For each call:
   - Map arguments to parameters
   - Substitute parameters in body
   - Insert substituted body at call site
5. Remove function definition

**Edge Cases:**
- Recursive functions → error
- Multiple return statements → wrap in block
- Variable name conflicts → rename with unique suffix

**Time:** 2 days

**Week 3 Total:** ~7 days

---

### Week 4: Polish & Integration (Days 22-28)

**Goal:** Make everything production-ready

#### 4.1 Change Signature (Optional - if time allows)
**Files:** `src/refactor/signature.rs`

**Algorithm:**
1. Find function definition
2. Find all call sites (SCIP)
3. Update definition signature
4. Update each call site:
   - Reorder arguments
   - Add defaults for new params
   - Remove deleted params

**Time:** 3 days (if we have time)

#### 4.2 MCP Tool Integration
**Files:** `src/mcp/tools.rs`

Add all refactorings as MCP tools:
- `rename_symbol`
- `inline_variable`
- `inline_function`
- `move_symbol`
- `extract_method`
- `change_signature` (optional)

All default to `preview: true` for safety.

**Time:** 1 day

#### 4.3 CLI Commands
**Files:** `src/commands/`

Implement CLI commands for all refactorings with beautiful terminal output.

**Time:** 1 day

#### 4.4 Documentation & Testing
**Files:** `CLAUDE.md`, `README.md`, tests

- Update CLAUDE.md with refactoring tool usage for AI agents
- Update README.md with examples
- Comprehensive test suite (real-world codebases)
- Integration tests for multi-file refactorings

**Time:** 2 days

**Week 4 Total:** ~7 days

---

## Technical Architecture

### Parser Strategy (Decided 2025-10-09)

**Decision:** Use pragmatic parser stack instead of bleeding-edge libraries

```
Language          | Parser              | Rationale
------------------|---------------------|----------------------------------
TypeScript/JS     | tree-sitter         | Already integrated, works well
Python            | rustpython-parser   | Mature, stable, on crates.io
Rust              | syn                 | Industry standard for proc macros
C++               | tree-sitter         | Already integrated
```

**Why not SWC?** Encountered complex serde version conflicts. Can add in v0.5.0 if needed.

**Dependencies Added:**
```toml
syn = { version = "2.0", features = ["full", "parsing", "extra-traits"] }
rustpython-parser = "0.4"
quote = "1.0"
prettyplease = "0.2"
similar = "2.5"
```

### Hybrid Architecture

```
┌─────────────────────────────────────────────┐
│    Powertools Refactoring Engine (Our IP)  │
├─────────────────────────────────────────────┤
│  - Import/export analyzer                   │
│  - Rename logic                             │
│  - Extract method logic                     │
│  - Scope analyzer                           │
│  - Transaction system                       │
│  - Preview system                           │
└─────────────────────────────────────────────┘
                    ▲
                    │ Uses
                    ▼
┌─────────────────────────────────────────────┐
│      External Parser Libraries               │
├─────────────────────────────────────────────┤
│  TypeScript/JS: tree-sitter (existing)      │
│  Python: rustpython-parser                  │
│  Rust: syn                                  │
│  C++: tree-sitter (existing)                │
└─────────────────────────────────────────────┘
                    ▲
                    │ Uses
                    ▼
┌─────────────────────────────────────────────┐
│         SCIP Indexes (existing)              │
├─────────────────────────────────────────────┤
│  - goto_definition                          │
│  - find_references                          │
│  - Symbol resolution                        │
└─────────────────────────────────────────────┘
```

**Why this works:**
1. Leverage mature parsers (don't reinvent parsing)
2. Build unique value (refactoring logic is our IP)
3. Language-specific optimization (best tool per language)
4. SCIP for semantics (already have it!)

---

## File Structure

```
src/refactor/
├── mod.rs                 # Public API
├── preview.rs             # ✅ Multi-file preview (extend existing)
├── replacer.rs            # ✅ Batch replacer (exists from v0.3.0)
├── transaction.rs         # NEW: Atomic transactions
├── imports/               # NEW: Import analysis
│   ├── mod.rs
│   ├── typescript.rs      # TypeScript/JavaScript imports
│   ├── python.rs          # Python imports
│   ├── rust.rs            # Rust imports
│   └── cpp.rs             # C++ includes
├── scope.rs               # NEW: Scope analysis (for extract method)
├── rename.rs              # NEW: Rename symbol
├── inline.rs              # NEW: Inline variable/function
├── move_symbol.rs         # NEW: Move symbol
├── extract.rs             # NEW: Extract method
├── signature.rs           # NEW: Change signature (optional)
└── tests/                 # Comprehensive tests
    ├── test_imports.rs
    ├── test_rename.rs
    ├── test_inline.rs
    ├── test_extract.rs
    └── fixtures/          # Real-world code samples

src/commands/
├── rename_symbol.rs       # NEW: CLI command
├── inline_variable.rs     # NEW: CLI command
├── move_symbol.rs         # NEW: CLI command
├── extract_method.rs      # NEW: CLI command
└── change_signature.rs    # NEW: CLI command (optional)

src/mcp/
└── tools.rs               # EXTEND: Add refactoring tools
```

**Estimated New Code:** 5,000-7,000 lines (with tests)

---

## Success Criteria

### Minimum Viable Product (MVP)
- ✅ Rename Symbol working for TypeScript/Rust
- ✅ Preview system shows all changes across files
- ✅ Atomic transactions (all-or-nothing)
- ✅ MCP tool integration
- ✅ Zero broken code (100% safe refactorings)

### V1 Release (v0.4.0)
- All 5-6 refactorings implemented
- Support for TypeScript, Rust, Python
- Comprehensive test suite (90%+ coverage)
- Documentation for AI agents
- Performance: <2s for 100-file rename
- C++ support (optional, if time allows)

---

## Progress Tracking

### ✅ Completed (2025-10-09)

- [x] Research open-source parser libraries
- [x] Create implementation plan (SEMANTIC_REFACTORING_PLAN.md)
- [x] Library analysis and trade-off documentation (LIBRARY_ANALYSIS.md)
- [x] Add parser dependencies to Cargo.toml
- [x] Resolve dependency conflicts (chose pragmatic stack)
- [x] Document plan in repository

### 🟡 In Progress

- [ ] Week 1: Foundation
  - [ ] Import/export analyzer (TypeScript)
  - [ ] Import/export analyzer (Python)
  - [ ] Import/export analyzer (Rust)
  - [ ] Enhanced multi-file preview system
  - [ ] Transactional refactoring system

### ⏳ Pending

- [ ] Week 2: Core Refactorings
- [ ] Week 3: Advanced Refactorings
- [ ] Week 4: Polish & Integration

---

## Risk Assessment & Mitigation

### High Risk Areas

1. **Extract Method Scope Analysis**
   - **Risk:** Complex, language-dependent variable scoping
   - **Mitigation:** Start with simple cases, add complexity incrementally

2. **Import/Export Updates**
   - **Risk:** Many edge cases per language (aliases, re-exports, etc.)
   - **Mitigation:** Comprehensive test suite with real-world code

3. **Transaction Atomicity**
   - **Risk:** File system failures mid-operation
   - **Mitigation:** Stage in memory, write only after validation

### Dependency Risks

- **SWC version conflicts:** Avoided by using tree-sitter + syn
- **API stability:** All chosen libraries are mature (tree-sitter, syn, rustpython)
- **Performance:** All libraries chosen for speed

---

## Timeline & Milestones

| Week | Milestone                    | Deliverable                           |
|------|------------------------------|---------------------------------------|
| 1    | Foundation Complete          | Import analyzer, preview, transactions |
| 2    | Core Refactorings            | Rename, inline variable, move symbol   |
| 3    | Advanced Refactorings        | Extract method, inline function        |
| 4    | Production Ready             | MCP tools, CLI, docs, tests            |

**Target Release Date:** End of Week 4 (Q1 2026)

---

## Future Enhancements (v0.5.0+)

Beyond v0.4.0, we can build:
- **Extract Component** (React/Vue)
- **Convert Class to Hooks**
- **Add Null Checks** (automated safety)
- **Split File** (break monoliths)
- **Safe Delete** (find + remove all references)
- **Upgrade to Oxc/SWC** (if needed for advanced TS transformations)

---

## Related Documentation

- **Implementation Plan:** [SEMANTIC_REFACTORING_PLAN.md](../SEMANTIC_REFACTORING_PLAN.md)
- **Library Analysis:** [LIBRARY_ANALYSIS.md](../LIBRARY_ANALYSIS.md)
- **Feature Wishlist:** [WISHLIST.md](../WISHLIST.md)
- **Changelog:** [CHANGELOG.md](../CHANGELOG.md)

---

## Questions & Decisions Log

### 2025-10-09: Parser Library Selection

**Question:** Should we use SWC for TypeScript/JavaScript parsing?

**Decision:** No, use tree-sitter (already integrated)

**Reasoning:** Encountered serde version conflicts with swc_common. Tree-sitter already works well for our use case (import analysis, AST traversal). Can upgrade to SWC/Oxc in v0.5.0 if we need more advanced transformations.

---

## Contact & Contribution

This is an active development effort. If you have questions or suggestions:
- Open an issue on GitHub with label `semantic-refactoring`
- Tag with `v0.4.0` milestone
- See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines

---

**Last Updated:** 2025-10-09
**Status:** Foundation phase started
**Next Update:** End of Week 1
