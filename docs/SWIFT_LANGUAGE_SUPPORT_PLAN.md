# Swift Language Support Implementation Plan

**Status:** 🟡 Planning Phase
**Target Version:** v0.4.1 (Side Quest from v0.4.0 Roadmap)
**Estimated Timeline:** 5-7 days
**Priority:** HIGH (iOS/macOS development critical)
**Created:** 2025-10-14

---

## Executive Summary

Add comprehensive Swift language support to powertools, enabling all existing features (SCIP navigation, tree-sitter analysis, refactoring) to work with Swift codebases. This unlocks powertools for iOS, macOS, and Swift server-side development.

**Key Differentiator:** Swift is the 5th most important language for agent-assisted development, with massive iOS/macOS developer base and growing server-side adoption (Vapor, Hummingbird).

---

## Current State Analysis

### What We Have
- **4 fully supported languages:** TypeScript, JavaScript, Python, Rust, C++
- **Tree-sitter integration:** Already supports multiple languages via tree-sitter grammars
- **SCIP integration:** Multi-language semantic indexing system
- **Hybrid architecture:** SCIP for cross-file, tree-sitter for local operations

### What We Need for Swift
1. **SCIP Indexer:** Semantic navigation (goto definition, find references)
2. **Tree-sitter Grammar:** AST parsing for local refactoring
3. **Import Analyzer:** Swift module/import analysis
4. **Language Detection:** File extension mapping (.swift)
5. **Refactoring Support:** All existing refactorings (rename, inline, etc.)

---

## Research Findings

### Tree-sitter Swift Grammar ✅ Available

**Two main options:**
1. **tree-sitter/tree-sitter-swift** (official tree-sitter org)
   - URL: https://github.com/tree-sitter/tree-sitter-swift
   - Status: Actively maintained
   - Maturity: Production-ready

2. **alex-pinkus/tree-sitter-swift** (alternative)
   - URL: https://github.com/alex-pinkus/tree-sitter-swift
   - Status: Actively maintained, more stars (351+)
   - Used by: Neovim, Emacs, VS Code extensions

**Decision:** Use **tree-sitter/tree-sitter-swift** (official)
- Better long-term support guarantee
- Aligned with other official grammars we use
- Consistent API with tree-sitter ecosystem

**Rust Crate:** We need to check if `tree-sitter-swift` crate exists on crates.io, or we'll need to build bindings manually.

### SCIP Swift Indexer ⚠️ Not Available

**Problem:** No official SCIP indexer for Swift exists (as of 2025-10-14)

**Available SCIP indexers:**
- ✅ TypeScript/JavaScript: `@sourcegraph/scip-typescript`
- ✅ Python: `scip-python`
- ✅ Rust: `rust-analyzer` (SCIP export)
- ✅ C++: `scip-clang`
- ✅ Java/Kotlin/Scala: `scip-java`
- ❌ Swift: None

**Options:**

#### Option 1: Tree-sitter-Only Mode (RECOMMENDED for v0.4.1)
**Use tree-sitter for everything** (no SCIP semantic indexing)

**Pros:**
- ✅ Fast implementation (2-3 days)
- ✅ Enables all local refactoring features immediately
- ✅ Works for 80% of use cases (single-file refactoring)
- ✅ No external dependencies to install
- ✅ Proven approach (we already use this for inline-variable)

**Cons:**
- ❌ No cross-file semantic navigation (goto definition across modules)
- ❌ Find references limited to single file
- ❌ Rename symbol won't work across modules

**Implementation:**
1. Use tree-sitter for AST parsing (variable extraction, function finding)
2. Use string-based import analysis (parse `import` statements manually)
3. Disable cross-file refactoring features for Swift (with clear error messages)
4. Document limitations in CLAUDE.md

**Use Cases Supported:**
- ✅ Inline variable (local)
- ✅ Extract method/function
- ✅ Find functions/classes (single file or directory scan)
- ✅ AST pattern search
- ⚠️ Rename symbol (single file only, warn user)
- ❌ Move symbol (cross-file, not supported)

#### Option 2: Build Custom SCIP Swift Indexer (Future v0.5.0+)
**Create our own SCIP indexer using Swift SourceKit**

**Approach:**
- Use Swift's official SourceKit API for semantic analysis
- Export to SCIP format (protobuf)
- Integrate into powertools indexing system

**Pros:**
- ✅ Full semantic navigation support
- ✅ Cross-file refactoring enabled
- ✅ Professional-grade tooling

**Cons:**
- ❌ 2-3 weeks development time
- ❌ Requires Swift toolchain on host machine
- ❌ Complex API (SourceKit learning curve)
- ❌ Maintenance burden

**Effort:** 10-15 days (separate project)

#### Option 3: Hybrid Approach with LSP Fallback (Future v0.6.0+)
**Use Swift LSP server (SourceKit-LSP) as indexing backend**

**Approach:**
- Shell out to `sourcekit-lsp` for semantic queries
- Parse LSP responses
- Convert to powertools internal format

**Pros:**
- ✅ Leverages official Swift tooling
- ✅ Full semantic support
- ✅ No custom indexer maintenance

**Cons:**
- ❌ Requires running LSP server
- ❌ Performance overhead (process spawning)
- ❌ Complex error handling

**Effort:** 5-7 days

---

## Recommended Implementation Plan (v0.4.1)

**Strategy:** Tree-sitter-only mode with clear documentation of limitations

### Phase 1: Foundation (Days 1-2)

#### 1.1 Add Swift to Language Enum
**File:** `powertools-cli/src/core/types.rs`

```rust
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Cpp,
    C,
    Swift,  // NEW
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            // ... existing mappings ...
            "swift" => Language::Swift,  // NEW
            _ => Language::Unknown,
        }
    }

    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        match self {
            // ... existing mappings ...
            Language::Swift => Some(tree_sitter_swift::LANGUAGE.into()),  // NEW
            Language::Unknown => None,
        }
    }
}
```

**Dependencies to add to `Cargo.toml`:**
```toml
[dependencies]
tree-sitter-swift = "0.6"  # Check latest version on crates.io
```

**Effort:** 1 hour

#### 1.2 Verify Tree-sitter Swift Integration
**File:** `powertools-cli/src/analyzers/tree_sitter_analyzer.rs`

Update `find_functions()` to support Swift:

```rust
pub fn find_functions(&mut self, file_path: &Path) -> Result<Vec<FunctionInfo>> {
    let analyzed = self.analyze_file(file_path)?;
    let query_str = match analyzed.language {
        // ... existing languages ...
        Language::Swift => r#"
            (function_declaration name: (simple_identifier) @name) @func
            (init_declaration) @func
        "#,
        _ => return Ok(Vec::new()),
    };
    // ... rest of implementation
}
```

**Test Cases:**
1. Parse simple Swift function
2. Extract function names
3. Verify AST structure

**Effort:** 2 hours

#### 1.3 Add Swift Import Analyzer
**File:** `powertools-cli/src/refactor/imports/swift.rs` (NEW)

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor};

use super::{ImportStatement, ImportAnalyzer, ImportKind};
use crate::core::Location;

pub struct SwiftImportAnalyzer;

impl SwiftImportAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Parse Swift import statements using tree-sitter
    /// Examples:
    ///   import Foundation
    ///   import UIKit
    ///   import struct Swift.String
    ///   import class Foundation.NSObject
    fn parse_import_statement(&self, node: tree_sitter::Node, content: &str) -> Result<ImportStatement> {
        // Extract module name and imported symbols
        let module_name = // ... parse from AST
        let symbols = // ... extract specific imports if any

        Ok(ImportStatement {
            source: module_name,
            symbols: symbols,
            location: // ... node position
            kind: ImportKind::Module,
        })
    }
}

impl ImportAnalyzer for SwiftImportAnalyzer {
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>> {
        let content = fs::read_to_string(file)?;
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_swift::LANGUAGE.into())?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Swift file"))?;

        // Query for import declarations
        let query = Query::new(
            &tree_sitter_swift::LANGUAGE.into(),
            r#"(import_declaration) @import"#
        )?;

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        let mut imports = Vec::new();
        for m in matches {
            for capture in m.captures {
                let import_stmt = self.parse_import_statement(capture.node, &content)?;
                imports.push(import_stmt);
            }
        }

        Ok(imports)
    }

    fn add_import(&self, file: &Path, import: &ImportStatement) -> Result<String> {
        let content = fs::read_to_string(file)?;

        // Swift convention: imports at top of file, after any file header comments
        let new_import = format!("import {}\n", import.source);

        // Find insertion point (after existing imports or at top)
        let insertion_point = self.find_import_insertion_point(&content)?;

        Ok(format!("{}{}{}",
            &content[..insertion_point],
            new_import,
            &content[insertion_point..]
        ))
    }

    fn remove_import(&self, file: &Path, symbol: &str) -> Result<String> {
        let content = fs::read_to_string(file)?;
        let imports = self.find_imports(file)?;

        // Remove the import line containing this symbol
        for import in imports {
            if import.source == symbol || import.symbols.contains(&symbol.to_string()) {
                // Remove this line from content
                // ... implementation
            }
        }

        Ok(content)
    }

    fn update_import_path(&self, file: &Path, old: &str, new: &str) -> Result<String> {
        let content = fs::read_to_string(file)?;

        // Replace old module name with new one
        // Handle both: import OldModule and import Something from OldModule
        let updated = content.replace(
            &format!("import {}", old),
            &format!("import {}", new)
        );

        Ok(updated)
    }
}
```

**Test Cases:**
1. Parse `import Foundation`
2. Parse `import struct Swift.String`
3. Add new import
4. Remove existing import
5. Update import path

**Effort:** 4 hours

### Phase 2: Refactoring Support (Days 3-4)

#### 2.1 Inline Variable for Swift
**File:** `powertools-cli/src/refactor/inline.rs`

Add Swift support to existing inline variable implementation:

```rust
fn extract_variable_declaration(&self, file_path: &PathBuf, content: &str, line: usize, column: usize) -> Result<VariableDeclaration> {
    let extension = file_path.extension().and_then(|s| s.to_str())?;

    match extension {
        // ... existing languages ...
        "swift" => self.extract_swift_variable(content, line, column),  // NEW
        _ => anyhow::bail!("Unsupported file extension: {}", extension),
    }
}

fn extract_swift_variable(&self, content: &str, line: usize, column: usize) -> Result<VariableDeclaration> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_swift::LANGUAGE.into())?;

    let tree = parser.parse(content, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse Swift code"))?;

    let root_node = tree.root_node();
    let target_byte = self.position_to_byte(content, line, column)?;

    // Find variable declaration: let/var binding
    let var_node = self.find_node_at_position(root_node, target_byte, "property_declaration")
        .or_else(|| self.find_node_at_position(root_node, target_byte, "value_binding_pattern"))
        .ok_or_else(|| anyhow::anyhow!("No variable declaration found"))?;

    // Extract: let name = initializer
    let mut name = String::new();
    let mut initializer = String::new();
    let mut is_mutable = false;

    let mut cursor = var_node.walk();
    for child in var_node.children(&mut cursor) {
        match child.kind() {
            "simple_identifier" => {
                if name.is_empty() {
                    name = content[child.byte_range()].to_string();
                }
            }
            "var" => {
                is_mutable = true;
            }
            _ if child.kind().ends_with("_expression") || child.kind() == "integer_literal" || child.kind() == "string_literal" => {
                initializer = content[child.byte_range()].trim().to_string();
            }
            _ => {}
        }
    }

    if name.is_empty() || initializer.is_empty() {
        anyhow::bail!("Could not extract Swift variable declaration");
    }

    Ok(VariableDeclaration {
        name,
        initializer,
        location: Location {
            file_path: Default::default(),
            line,
            column,
            end_line: None,
            end_column: None,
        },
        declaration_start_byte: var_node.start_byte(),
        declaration_end_byte: var_node.end_byte(),
        is_mutable,
    })
}

fn find_swift_identifiers(&self, content: &str, var_name: &str, declaration_line: usize, file_path: &PathBuf) -> Result<Vec<Reference>> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_swift::LANGUAGE.into())?;

    let tree = parser.parse(content, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse Swift code"))?;

    let root_node = tree.root_node();
    let mut references = Vec::new();

    self.collect_identifiers(root_node, content, var_name, declaration_line, file_path, &mut references);

    Ok(references)
}
```

**Test Cases:**
1. Inline `let` constant
2. Reject `var` mutable variable
3. Handle Swift-specific syntax (optionals, nil coalescing)
4. Multi-line initializers

**Effort:** 6 hours

#### 2.2 Function/Class Finding for Swift
**File:** `powertools-cli/src/analyzers/class_finder.rs`

Add Swift class/struct/enum/protocol detection:

```rust
pub fn find_classes(&mut self, file_path: &Path) -> Result<Vec<ClassInfo>> {
    let analyzed = self.analyze_file(file_path)?;
    let query_str = match analyzed.language {
        // ... existing languages ...
        Language::Swift => r#"
            (class_declaration name: (type_identifier) @name) @class
            (struct_declaration name: (type_identifier) @name) @struct
            (enum_declaration name: (type_identifier) @name) @enum
            (protocol_declaration name: (type_identifier) @name) @protocol
        "#,
        _ => return Ok(Vec::new()),
    };
    // ... rest of implementation
}
```

**Effort:** 2 hours

### Phase 3: CLI & MCP Integration (Day 5)

#### 3.1 Update Index Command
**File:** `powertools-cli/src/commands/index.rs`

Add error message for Swift SCIP indexing:

```rust
pub fn index_project(&self, languages: Vec<String>) -> Result<()> {
    for lang in languages {
        match lang.as_str() {
            // ... existing languages ...
            "swift" => {
                eprintln!("⚠️  SCIP indexing not available for Swift");
                eprintln!("    Swift uses tree-sitter-only mode:");
                eprintln!("    ✅ Local refactoring (inline variable, extract method)");
                eprintln!("    ✅ Function/class finding");
                eprintln!("    ❌ Cross-file navigation (goto definition, find references)");
                eprintln!("    See docs/SWIFT_LANGUAGE_SUPPORT_PLAN.md for details");
                eprintln!("");
                eprintln!("    Tree-sitter grammar ready - no indexing needed!");
                return Ok(());
            }
            _ => {}
        }
    }
}
```

#### 3.2 Update MCP Tools
**File:** `powertools-cli/src/mcp/tools.rs`

Ensure Swift is documented as tree-sitter-only:

```rust
// In tool descriptions:
// "Supported languages: TypeScript, JavaScript, Python, Rust, C++, Swift (tree-sitter only)"
```

**Effort:** 2 hours

### Phase 4: Documentation & Testing (Days 6-7)

#### 4.1 Update CLAUDE.md
**File:** `.claude/CLAUDE.md`

```markdown
### Supported Languages:
- **TypeScript**: Full semantic navigation via scip-typescript
- **JavaScript**: Full semantic navigation via scip-typescript
- **Python**: Full semantic navigation via scip-python
- **Rust**: Full semantic navigation via rust-analyzer
- **C++**: Full semantic navigation via scip-clang
- **Swift**: Tree-sitter-only mode (local refactoring, no cross-file navigation)

**Swift Limitations:**
- ✅ Inline variable (local)
- ✅ Extract method/function
- ✅ Find functions/classes (AST-based)
- ✅ Search code patterns
- ⚠️ Rename symbol (single file only)
- ❌ Cross-file goto definition (requires SCIP indexer - not available)
- ❌ Cross-file find references (requires SCIP indexer - not available)
- ❌ Move symbol (cross-file operation - not supported)

**Why these limitations?** No official SCIP indexer exists for Swift yet. We use tree-sitter for AST parsing, which enables local refactoring but not cross-file semantic navigation. See `docs/SWIFT_LANGUAGE_SUPPORT_PLAN.md` for roadmap.
```

#### 4.2 Create Testing Guide
**File:** `docs/SWIFT_TESTING.md`

Document test cases and known issues for Swift support.

#### 4.3 Real-World Testing

**Test Projects:**
1. **Vapor** (Swift server framework)
   ```bash
   git clone https://github.com/vapor/vapor.git
   cd vapor
   # Test: find functions, inline variable, extract method
   ```

2. **Alamofire** (Swift HTTP networking)
   ```bash
   git clone https://github.com/Alamofire/Alamofire.git
   cd Alamofire
   # Test: class finding, function extraction
   ```

**Test Cases:**
- ✅ Inline `let` constant in function
- ✅ Extract method from code block
- ✅ Find all functions in .swift file
- ✅ Find all classes/structs
- ✅ Parse import statements
- ⚠️ Rename symbol (verify single-file only)
- ❌ Cross-file operations (verify error message)

**Effort:** 8 hours

---

## File Structure Changes

### New Files
```
powertools-cli/src/refactor/imports/
└── swift.rs                      # NEW: Swift import analyzer

docs/
├── SWIFT_LANGUAGE_SUPPORT_PLAN.md   # NEW: This document
└── SWIFT_TESTING.md                 # NEW: Testing guide
```

### Modified Files
```
powertools-cli/src/core/
└── types.rs                      # Add Swift to Language enum

powertools-cli/src/analyzers/
├── tree_sitter_analyzer.rs       # Add Swift function/class queries
└── class_finder.rs               # Add Swift class detection

powertools-cli/src/refactor/
└── inline.rs                     # Add Swift variable extraction

powertools-cli/src/commands/
└── index.rs                      # Add Swift indexing message

powertools-cli/src/mcp/
└── tools.rs                      # Update tool descriptions

.claude/
└── CLAUDE.md                     # Document Swift support + limitations

Cargo.toml                        # Add tree-sitter-swift dependency
```

**Estimated New/Modified Code:** ~800 lines

---

## Dependencies

### Cargo.toml Additions
```toml
[dependencies]
# Tree-sitter Swift grammar
tree-sitter-swift = "0.6"  # Verify latest version on crates.io

# Note: If crate doesn't exist, we'll need to:
# 1. Add git dependency: tree-sitter-swift = { git = "https://github.com/tree-sitter/tree-sitter-swift" }
# 2. Or build bindings manually using bindgen
```

**Fallback if crate unavailable:**
```toml
[build-dependencies]
cc = "1.0"

# In build.rs:
# - Clone tree-sitter-swift
# - Compile parser.c with cc crate
# - Generate Rust bindings
```

---

## Timeline & Milestones

| Day   | Milestone                        | Deliverable                                      |
|-------|----------------------------------|--------------------------------------------------|
| 1     | Language Foundation              | Swift added to Language enum, tree-sitter works  |
| 2     | Import Analysis                  | Swift import analyzer complete                   |
| 3-4   | Refactoring Support             | Inline variable + function finding for Swift     |
| 5     | CLI & MCP Integration           | Commands work, documentation updated             |
| 6-7   | Testing & Documentation         | Real-world testing, known issues documented      |

**Total:** 5-7 days

---

## Success Criteria

### Minimum Viable Product (MVP)
- ✅ Swift files recognized (Language::Swift)
- ✅ Tree-sitter parsing works
- ✅ Inline variable works for Swift `let` constants
- ✅ Function/class finding works
- ✅ Clear error messages for unsupported operations
- ✅ Documentation updated with limitations

### V1 Release (v0.4.1)
- ✅ All local refactoring features work
- ✅ Import analyzer functional
- ✅ Extract method/function works
- ✅ Tested on real Swift projects (Vapor, Alamofire)
- ✅ Known limitations documented
- ✅ Error messages guide users

---

## Future Enhancements (v0.5.0+)

### Option 1: Custom SCIP Swift Indexer
**Effort:** 10-15 days
**Value:** Full cross-file semantic navigation

**Approach:**
1. Research Swift SourceKit API
2. Build SCIP export functionality
3. Auto-install as `~/.local/bin/scip-swift`
4. Integrate into powertools indexing pipeline

**Resources:**
- https://github.com/apple/swift/tree/main/tools/SourceKit
- https://github.com/sourcegraph/scip
- https://docs.sourcegraph.com/code_navigation/explanations/writing_an_indexer

### Option 2: LSP Integration
**Effort:** 5-7 days
**Value:** Leverage existing Swift tooling

**Approach:**
1. Shell out to `sourcekit-lsp` for queries
2. Parse LSP responses (JSON-RPC)
3. Convert to powertools internal types

**Trade-offs:**
- ✅ No custom indexer maintenance
- ❌ Slower (process spawning)
- ❌ Requires Swift toolchain installed

---

## Risk Assessment

### Low Risk
- ✅ Tree-sitter Swift grammar exists and is mature
- ✅ We already support tree-sitter-only mode (inline-variable uses this)
- ✅ Clear documentation mitigates user confusion

### Medium Risk
- ⚠️ `tree-sitter-swift` Rust crate availability (may need manual bindings)
- ⚠️ Swift syntax edge cases (optionals, generics, closures)
- ⚠️ User expectations (may expect full SCIP support)

### Mitigation
1. **Check crates.io first** before committing to implementation
2. **Comprehensive testing** on real Swift projects
3. **Clear documentation** of limitations upfront
4. **Future roadmap** visible (SCIP indexer in v0.5.0)

---

## User Communication

### What to Tell Users

**Positive Framing:**
> "Powertools now supports Swift for local refactoring! You can inline variables, extract methods, and find functions/classes. Cross-file navigation (goto definition, find references) will come in v0.5.0 when we add our custom SCIP indexer."

**Feature Matrix:**

| Feature                  | Swift Support | Notes                        |
|--------------------------|---------------|------------------------------|
| Inline Variable          | ✅ Full       | Tree-sitter AST parsing      |
| Extract Method           | ✅ Full       | Tree-sitter AST parsing      |
| Find Functions/Classes   | ✅ Full       | Tree-sitter queries          |
| Search Code Patterns     | ✅ Full       | Tree-sitter queries          |
| Rename Symbol            | ⚠️ Local only | Single file, warns user      |
| Goto Definition          | ❌ Not yet    | Requires SCIP (v0.5.0)       |
| Find References          | ❌ Not yet    | Requires SCIP (v0.5.0)       |
| Move Symbol              | ❌ Not yet    | Requires SCIP (v0.5.0)       |
| Import Analysis          | ✅ Full       | Tree-sitter parsing          |

### Error Messages

When user tries cross-file operation:
```
❌ Cross-file navigation not available for Swift (yet!)

Swift currently uses tree-sitter-only mode, which enables:
  ✅ Local refactoring (inline variable, extract method)
  ✅ Function/class finding
  ✅ Code pattern search

For cross-file operations (goto definition, find references), we need
a SCIP indexer. This is on our roadmap for v0.5.0!

Why? No official SCIP indexer for Swift exists yet. We're planning to
build one using Swift SourceKit. See docs/SWIFT_LANGUAGE_SUPPORT_PLAN.md
for details.
```

---

## Next Steps

1. **Verify tree-sitter-swift crate availability** on crates.io
2. **Get approval for tree-sitter-only approach** from user
3. **Begin Phase 1 implementation** (Language enum, dependencies)
4. **Test on simple Swift file** to validate approach
5. **Iterate based on findings**

---

## Questions for User

1. ✅ **Approved to proceed with tree-sitter-only mode for v0.4.1?**
   - This gives 80% of value in 20% of time
   - Full SCIP support can come later in v0.5.0

2. ✅ **Priority: Which Swift features matter most?**
   - Inline variable + extract method (local refactoring)?
   - Or cross-file navigation (requires custom SCIP indexer)?

3. ✅ **Testing: Have Swift projects we should test against?**
   - Vapor, Alamofire, or user's own Swift codebase?

---

## Related Documentation

- **Main Roadmap:** [SEMANTIC_REFACTORING_V0.4.0.md](./SEMANTIC_REFACTORING_V0.4.0.md)
- **Tree-sitter Architecture:** See inline-variable implementation in `src/refactor/inline.rs`
- **SCIP Integration:** [SCIP Protocol](https://github.com/sourcegraph/scip)
- **Writing SCIP Indexers:** [Sourcegraph Docs](https://docs.sourcegraph.com/code_navigation/explanations/writing_an_indexer)

---

**Last Updated:** 2025-10-14
**Status:** Awaiting approval to proceed
**Next Update:** After Phase 1 completion
