# Open Source Library Analysis for Semantic Refactoring

**Date:** 2025-10-09
**Status:** Research Complete - Ready for Implementation

## Summary

After extensive research, we found several excellent open-source libraries that can accelerate our semantic refactoring implementation. We'll **leverage existing parsers** while building our own **refactoring logic** on top.

---

## Recommended Libraries to Use

### 1. **SWC (Speedy Web Compiler)** ⭐ RECOMMENDED for TypeScript/JavaScript

**What it is:** Rust-based TypeScript/JavaScript compiler used by Next.js, Vercel, Shopify

**Why use it:**
- ✅ **Extremely fast** - Production-proven at scale
- ✅ **Full TypeScript/JavaScript AST** - Comprehensive syntax tree
- ✅ **Active maintenance** - Used by major projects (Next.js, Turbopack)
- ✅ **Visitor pattern** - Easy AST traversal and transformation
- ✅ **Source maps** - Track transformations for accurate edits
- ✅ **Well documented** - Extensive examples and API docs

**Crates to use:**
```toml
swc_ecma_parser = "0.149"      # Parser
swc_ecma_ast = "0.115"         # AST types
swc_ecma_visit = "0.101"       # Visitor pattern for traversal
swc_common = "0.36"            # Common utilities (spans, source maps)
```

**What we'll build:**
- Import/export analyzer using SWC AST
- Rename refactoring using SWC visitor pattern
- Extract method using SWC scope analysis
- Code generation back to source

**Example:**
```rust
use swc_ecma_parser::{Parser, StringInput};
use swc_ecma_ast::*;

// Parse TypeScript/JavaScript
let module = parser.parse_module().unwrap();

// Visit and transform AST
// (we build the refactoring logic)
```

---

### 2. **Ruff Parser** ⭐ RECOMMENDED for Python

**What it is:** Hand-written recursive descent parser from Astral (makers of Ruff linter)

**Why use it:**
- ✅ **Super fast** - 2x faster than previous parsers, designed for performance
- ✅ **Error resilient** - Handles malformed code gracefully
- ✅ **Python 3.x support** - Full modern Python syntax
- ✅ **Active development** - Major update in v0.4.0 (April 2024)
- ✅ **AST compatible** - Similar to Python's official AST module

**Crate:**
```toml
ruff_python_parser = "0.8"     # Latest parser
ruff_python_ast = "0.8"        # AST types
ruff_text_size = "0.8"         # Source locations
```

**What we'll build:**
- Python import analyzer
- Rename refactoring for Python
- Extract function for Python
- Scope analysis for Python

**Alternative:** `rustpython-parser` (older, less maintained, but still works)

---

### 3. **rust-analyzer** 🤔 USE VIA SCIP for Rust

**What it is:** Official Rust Language Server with full semantic analysis

**Why NOT use directly:**
- ❌ Complex integration (designed for IDE use)
- ❌ Heavy dependencies
- ❌ Overkill for our needs

**Why use indirectly:**
- ✅ We already use it via **SCIP indexes** (goto_definition, find_references)
- ✅ For AST manipulation, use **syn** crate (lighter weight)

**Crates to use:**
```toml
syn = "2.0"           # Parse Rust code
quote = "1.0"         # Generate Rust code
prettyplease = "0.2"  # Format generated code
```

**What we'll build:**
- Rust import analyzer using `syn`
- Rename using SCIP + syn for precise edits
- Extract method using syn AST manipulation

---

### 4. **Oxc Parser** 🚀 FUTURE CONSIDERATION for TypeScript/JavaScript

**What it is:** Next-generation JavaScript/TypeScript parser, 3x faster than SWC

**Why NOT use yet:**
- ⚠️ Alpha/beta quality (transformer just released in Sept 2024)
- ⚠️ API still evolving
- ⚠️ Less production usage than SWC

**Why consider later:**
- ✅ Insanely fast (3x faster than SWC)
- ✅ Modern architecture
- ✅ Full TypeScript support

**Decision:** Start with **SWC** (stable, proven), migrate to Oxc in v0.5.0+ when it's more mature.

---

### 5. **Tree-sitter** ✅ KEEP USING for C++ and Multi-language

**What it is:** What we already use

**Why keep it:**
- ✅ Already integrated
- ✅ Supports C++, Go, Java
- ✅ Good for pattern matching (search_ast)
- ✅ Language-agnostic

**What we'll build:**
- C++ import analyzer (parse #include statements)
- Refactorings for C++ using tree-sitter AST

---

## Libraries We WON'T Use (and Why)

### ❌ rust-analyzer internals
- Too complex for our use case
- Designed for IDE integration, not standalone tools
- Heavy dependencies

### ❌ Babel (via Node.js)
- Wrong ecosystem (we're Rust-native)
- Performance overhead of calling Node.js from Rust
- We have better options (SWC, Oxc)

### ❌ libclang refactoring tools
- C++-specific
- Complex API
- Tree-sitter is good enough for C++

---

## Architecture Decision

### **Hybrid Approach:** Use existing parsers + build refactoring logic

```
┌─────────────────────────────────────────────┐
│         Powertools Refactoring Engine       │
├─────────────────────────────────────────────┤
│  Our Code:                                  │
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
│       External Parser Libraries              │
├─────────────────────────────────────────────┤
│  TypeScript/JavaScript: SWC                 │
│  Python: Ruff Parser                        │
│  Rust: syn                                  │
│  C++: Tree-sitter (existing)                │
└─────────────────────────────────────────────┘
                    ▲
                    │ Uses
                    ▼
┌─────────────────────────────────────────────┐
│       SCIP Indexes (existing)               │
├─────────────────────────────────────────────┤
│  - goto_definition                          │
│  - find_references                          │
│  - Symbol resolution                        │
└─────────────────────────────────────────────┘
```

**Why this works:**
1. **Leverage mature parsers** - Don't reinvent parsing
2. **Build unique value** - Refactoring logic is our IP
3. **Language-specific optimization** - Best parser per language
4. **SCIP for semantics** - Already have semantic understanding

---

## Implementation Plan Update

### Week 1: Foundation (Updated with Libraries)

#### 1.1 Import/Export Analyzer - TypeScript/JavaScript
**Using:** SWC parser

```rust
// src/refactor/imports/typescript.rs
use swc_ecma_parser::{Parser, Syntax};
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};

pub struct ImportAnalyzer {
    imports: Vec<ImportStatement>,
}

impl Visit for ImportAnalyzer {
    fn visit_import_decl(&mut self, import: &ImportDecl) {
        // Extract import info: source, specifiers, etc.
        // SWC provides full AST - we just extract what we need
    }
}

pub fn analyze_imports_ts(source: &str) -> Result<Vec<ImportStatement>> {
    let module = parse_typescript(source)?;
    let mut analyzer = ImportAnalyzer::new();
    module.visit_with(&mut analyzer);
    Ok(analyzer.imports)
}
```

**Time saved:** 3-4 days (don't need to write TypeScript parser)

#### 1.2 Import/Export Analyzer - Python
**Using:** Ruff parser

```rust
// src/refactor/imports/python.rs
use ruff_python_parser::parse_module;
use ruff_python_ast::*;

pub fn analyze_imports_python(source: &str) -> Result<Vec<ImportStatement>> {
    let module = parse_module(source)?;

    let mut imports = Vec::new();
    for stmt in module.body {
        match stmt {
            Stmt::Import(import_stmt) => {
                // Extract import info
            }
            Stmt::ImportFrom(from_stmt) => {
                // Extract from...import info
            }
            _ => {}
        }
    }
    Ok(imports)
}
```

**Time saved:** 2-3 days (don't need to write Python parser)

#### 1.3 Import/Export Analyzer - Rust
**Using:** syn crate

```rust
// src/refactor/imports/rust.rs
use syn::{File, Item, UseTree};

pub fn analyze_imports_rust(source: &str) -> Result<Vec<ImportStatement>> {
    let ast: File = syn::parse_str(source)?;

    let mut imports = Vec::new();
    for item in ast.items {
        if let Item::Use(use_item) = item {
            // Extract use statement info
            extract_use_tree(&use_item.tree, &mut imports);
        }
    }
    Ok(imports)
}
```

**Time saved:** 1-2 days (syn is very easy to use)

---

### Week 2: Rename Symbol (Updated with Libraries)

**Using:** SWC (TypeScript), Ruff (Python), syn (Rust), SCIP (all languages)

```rust
// src/refactor/rename.rs
pub struct RenameRefactoring {
    language: Language,
}

impl RenameRefactoring {
    pub fn rename_symbol(
        &self,
        symbol_location: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<RefactoringTransaction> {
        // 1. Find all references via SCIP (language-agnostic)
        let references = scip_find_references(symbol_location)?;

        // 2. For each file with references, parse and modify
        let mut transaction = RefactoringTransaction::new();

        for reference in references {
            match self.language {
                Language::TypeScript | Language::JavaScript => {
                    // Use SWC to parse and modify
                    let new_content = rename_in_typescript(
                        &reference.file,
                        &reference.location,
                        old_name,
                        new_name
                    )?;
                    transaction.stage_change(&reference.file, new_content);
                }
                Language::Python => {
                    // Use Ruff to parse and modify
                    let new_content = rename_in_python(/* ... */)?;
                    transaction.stage_change(&reference.file, new_content);
                }
                Language::Rust => {
                    // Use syn to parse and modify
                    let new_content = rename_in_rust(/* ... */)?;
                    transaction.stage_change(&reference.file, new_content);
                }
                // ...
            }
        }

        Ok(transaction)
    }
}
```

**Time saved:** 5-7 days (parsers handle all syntax complexity)

---

## Dependencies to Add

```toml
# Cargo.toml additions

# TypeScript/JavaScript parsing
swc_ecma_parser = "0.149"
swc_ecma_ast = "0.115"
swc_ecma_visit = "0.101"
swc_common = "0.36"

# Python parsing
ruff_python_parser = "0.8"
ruff_python_ast = "0.8"
ruff_text_size = "0.8"

# Rust parsing and code generation
syn = { version = "2.0", features = ["full", "parsing", "extra-traits"] }
quote = "1.0"
prettyplease = "0.2"

# Utilities
similar = "2.5"  # Better diff algorithm for previews
```

**Total added:** ~8 crates
**Binary size impact:** ~5-10 MB (acceptable, these are production libraries)
**Compile time impact:** +30-60 seconds (one-time cost, worth it)

---

## What We Still Build (Our Unique Value)

### 1. **Refactoring Logic** 🔧
- How to safely rename across files
- How to extract methods with correct scope analysis
- How to inline functions preserving semantics
- How to change signatures and update call sites

### 2. **Multi-Language Coordination** 🌐
- Unified API across TypeScript, Python, Rust, C++
- Language-specific optimizations
- Consistent preview format

### 3. **SCIP Integration** 🔍
- Use SCIP for finding references (already have this!)
- Combine semantic understanding with AST manipulation
- Cross-file refactoring intelligence

### 4. **Transaction System** 💾
- Atomic all-or-nothing refactorings
- Rollback on failure
- Preview before applying

### 5. **MCP Integration** 🤖
- AI-friendly tool interface
- JSON output for Claude Code
- Preview-first safety model

### 6. **CLI Tools** ⌨️
- Beautiful terminal output
- Progress indicators
- User-friendly error messages

---

## Estimated Time Savings

### Original Estimate: 3-4 weeks
### With Libraries: 2-3 weeks

**Breakdown:**
- Week 1 Foundation: **Save 5-6 days** (parsers provided)
  - Original: 7 days → New: 2-3 days

- Week 2 Core Refactorings: **Save 3-4 days** (AST manipulation easier)
  - Original: 7 days → New: 4-5 days

- Week 3 Advanced: **Save 2-3 days** (scope analysis via parsers)
  - Original: 7 days → New: 5-6 days

- Week 4 Polish: **No change** (testing still needed)
  - Original: 7 days → New: 7 days

**Total Time: 18-21 days instead of 28 days** ✅

---

## Risk Mitigation

### Risk: Dependency on External Libraries
**Mitigation:**
- All chosen libraries are production-proven (SWC, Ruff, syn)
- Large community, active maintenance
- Can fork if needed (all open source)

### Risk: API Changes
**Mitigation:**
- Pin exact versions in Cargo.toml
- Test suite will catch breaking changes
- SWC/Ruff have stable APIs

### Risk: Performance
**Mitigation:**
- All libraries chosen for speed (SWC, Ruff are fastest in class)
- Benchmark early, optimize if needed

---

## Next Steps

1. ✅ Add dependencies to Cargo.toml
2. ✅ Create `src/refactor/imports/` module structure
3. ✅ Implement TypeScript import analyzer using SWC
4. ✅ Implement Python import analyzer using Ruff
5. ✅ Implement Rust import analyzer using syn
6. ✅ Build unified ImportAnalyzer API
7. ✅ Write tests with real-world code samples
8. → Move to Week 2: Rename Symbol

---

## Conclusion

**Decision: Use external parser libraries** ✅

**Rationale:**
1. **Faster development** - 10+ days saved
2. **Better quality** - Production-proven parsers
3. **Focus on value** - Build refactoring logic, not parsers
4. **Maintainability** - Let experts handle parsing edge cases

**We're building:**
- The world's first multi-language semantic refactoring tool for AI agents
- Unique SCIP + parser combination
- MCP-first design for AI workflows

**We're not building:**
- Yet another TypeScript parser
- Yet another Python parser
- Parser generators or low-level tooling

This is the right trade-off. Let's ship! 🚀
