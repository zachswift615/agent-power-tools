# Swift Language Support - Implementation Roadmap

**Status:** 🟢 APPROVED - Starting Implementation
**Architecture:** Hybrid SCIP + LSP (LSP fallback for Swift only)
**Timeline:** 9-11 days
**Target Version:** v0.4.1
**Started:** 2025-10-14

---

## Implementation Strategy

**Hybrid Architecture Approved:**
- ✅ Keep SCIP for TypeScript, Python, Rust, C++ (existing, fast)
- ✅ Add LSP client infrastructure (generic, reusable)
- ✅ Integrate sourcekit-lsp for Swift (new capability)
- ✅ Enable full cross-file navigation for Swift

**Trade-offs Accepted:**
- Swift queries slower (40ms vs 1ms) but functional
- Added complexity justified by cross-file navigation
- LSP infrastructure reusable for future languages

---

## Phase 1: Generic LSP Client Infrastructure (Days 1-4)

### Goal
Build reusable LSP client that works with any LSP server (sourcekit-lsp, future servers)

### Deliverables

#### 1.1 Core LSP Client (Day 1)
**File:** `powertools-cli/src/indexers/lsp_client.rs` (NEW)

**Features:**
- Process spawning and lifecycle management
- JSON-RPC 2.0 protocol implementation
- Initialize/Shutdown handshake
- Request/response matching

**API:**
```rust
pub struct LspClient {
    process: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    request_id: AtomicU64,
    server_capabilities: ServerCapabilities,
}

impl LspClient {
    /// Start an LSP server process
    pub fn start(command: &str, args: &[String], root_uri: &str) -> Result<Self>;

    /// Send a request and wait for response
    fn send_request(&mut self, method: &str, params: Value) -> Result<Value>;

    /// Gracefully shut down the server
    pub fn shutdown(&mut self) -> Result<()>;
}
```

**Dependencies:**
```toml
[dependencies]
serde_json = "1.0"
lsp-types = "0.95"  # Official LSP type definitions
```

**Test Cases:**
- Start/stop LSP server
- Send initialize request
- Handle malformed responses
- Timeout handling

**Effort:** 8 hours

#### 1.2 LSP Navigation Methods (Day 2)
**File:** `powertools-cli/src/indexers/lsp_client.rs` (extend)

**Features:**
```rust
impl LspClient {
    /// Notify server of opened file
    pub fn did_open(&mut self, uri: &str, language_id: &str, text: &str) -> Result<()>;

    /// Find definition of symbol at position
    pub fn goto_definition(&mut self, uri: &str, line: u32, character: u32) -> Result<Vec<Location>>;

    /// Find all references to symbol
    pub fn find_references(&mut self, uri: &str, line: u32, character: u32, include_declaration: bool) -> Result<Vec<Location>>;

    /// Convert LSP Location to our internal Location type
    fn convert_location(&self, lsp_loc: lsp_types::Location) -> Location;
}
```

**Error Handling:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("LSP server not running")]
    ServerNotRunning,

    #[error("LSP server initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    #[error("Server returned error: code={code}, message={message}")]
    ServerError { code: i32, message: String },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}
```

**Test Cases:**
- Goto definition on simple Swift function
- Find references across multiple files
- Handle "not found" gracefully
- URI path conversion (file:// URLs)

**Effort:** 6 hours

#### 1.3 LSP Query Abstraction (Day 3)
**File:** `powertools-cli/src/indexers/lsp_query.rs` (NEW)

**Goal:** Unified interface matching `ScipQuery` API

```rust
use crate::core::{Location, Reference, ReferenceKind};
use super::lsp_client::LspClient;

pub struct LspQuery {
    client: LspClient,
    project_root: PathBuf,
}

impl LspQuery {
    pub fn new(server_command: &str, args: &[String], project_root: PathBuf) -> Result<Self> {
        let root_uri = format!("file://{}", project_root.display());
        let client = LspClient::start(server_command, args, &root_uri)?;

        Ok(Self { client, project_root })
    }

    /// Find definition (matches ScipQuery::find_definition signature)
    pub fn find_definition(&mut self, file: &Path, line: usize, column: usize) -> Result<Location> {
        let uri = self.path_to_uri(file)?;
        let locations = self.client.goto_definition(&uri, line as u32 - 1, column as u32 - 1)?;

        locations.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No definition found"))
    }

    /// Find references (matches ScipQuery::find_references signature)
    pub fn find_references(&mut self, symbol: &str, file: &Path, line: usize, column: usize) -> Result<Vec<Reference>> {
        let uri = self.path_to_uri(file)?;
        let lsp_locations = self.client.find_references(&uri, line as u32 - 1, column as u32 - 1, true)?;

        // Convert LSP locations to our Reference type
        let references = lsp_locations.into_iter().map(|loc| Reference {
            location: self.convert_location(loc),
            kind: ReferenceKind::Reference,
            context: None,
        }).collect();

        Ok(references)
    }

    fn path_to_uri(&self, path: &Path) -> Result<String> {
        Ok(format!("file://{}", path.canonicalize()?.display()))
    }
}
```

**Benefits:**
- Same API as `ScipQuery`
- Easy to swap backends
- Hides LSP complexity from callers

**Test Cases:**
- API compatibility with ScipQuery
- Path/URI conversion
- Error propagation

**Effort:** 4 hours

#### 1.4 Backend Abstraction Layer (Day 4)
**File:** `powertools-cli/src/indexers/mod.rs` (extend)

**Goal:** Unified navigator that chooses SCIP or LSP automatically

```rust
pub enum QueryBackend {
    Scip(ScipQuery),
    Lsp(LspQuery),
}

pub struct UnifiedQuery {
    backend: QueryBackend,
}

impl UnifiedQuery {
    /// Auto-detect and create appropriate backend
    pub fn for_project(project_root: &Path) -> Result<Self> {
        let languages = detect_project_languages(project_root)?;

        // Use LSP for Swift (no SCIP indexer available)
        if languages.contains(&Language::Swift) {
            let lsp = LspQuery::new("sourcekit-lsp", &[], project_root.to_path_buf())?;
            return Ok(Self { backend: QueryBackend::Lsp(lsp) });
        }

        // Use SCIP for everything else (faster)
        let scip = ScipQuery::load(project_root)?;
        Ok(Self { backend: QueryBackend::Scip(scip) })
    }

    pub fn find_definition(&mut self, file: &Path, line: usize, column: usize) -> Result<Location> {
        match &mut self.backend {
            QueryBackend::Scip(scip) => scip.find_definition(file, line, column),
            QueryBackend::Lsp(lsp) => lsp.find_definition(file, line, column),
        }
    }

    pub fn find_references(&mut self, symbol: &str, file: &Path, line: usize, column: usize) -> Result<Vec<Reference>> {
        match &mut self.backend {
            QueryBackend::Scip(scip) => scip.find_references(symbol),
            QueryBackend::Lsp(lsp) => lsp.find_references(symbol, file, line, column),
        }
    }
}
```

**Migration Path:**
- Existing code uses `ScipQuery` directly → works unchanged
- New code uses `UnifiedQuery` → auto-selects backend
- Gradual migration as needed

**Test Cases:**
- Auto-detection for TypeScript project → SCIP
- Auto-detection for Swift project → LSP
- Mixed project (TS + Swift) → ???

**Effort:** 6 hours

**Phase 1 Total:** ~30 hours (3.75 days)

---

## Phase 2: Swift Language Integration (Days 5-7)

### Goal
Add Swift to all language detection, tree-sitter analysis, and refactoring

### Deliverables

#### 2.1 Swift Language Type (Day 5 morning)
**File:** `powertools-cli/src/core/types.rs`

**Changes:**
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
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

**Dependencies to add:**
```toml
[dependencies]
tree-sitter-swift = "0.6"  # Check latest on crates.io
```

**Fallback if crate unavailable:**
- Build tree-sitter-swift from source using build.rs
- Generate bindings with bindgen

**Effort:** 2 hours

#### 2.2 Swift Tree-sitter Queries (Day 5 afternoon)
**File:** `powertools-cli/src/analyzers/tree_sitter_analyzer.rs`

**Add Swift function finding:**
```rust
pub fn find_functions(&mut self, file_path: &Path) -> Result<Vec<FunctionInfo>> {
    let analyzed = self.analyze_file(file_path)?;
    let query_str = match analyzed.language {
        // ... existing languages ...
        Language::Swift => r#"
            (function_declaration name: (simple_identifier) @name) @func
            (init_declaration) @func
            (subscript_declaration) @func
        "#,
        _ => return Ok(Vec::new()),
    };
    // ... rest unchanged
}
```

**Add Swift class finding:**
```rust
// In class_finder.rs
Language::Swift => r#"
    (class_declaration name: (type_identifier) @name) @class
    (struct_declaration name: (type_identifier) @name) @struct
    (enum_declaration name: (type_identifier) @name) @enum
    (protocol_declaration name: (type_identifier) @name) @protocol
    (actor_declaration name: (type_identifier) @name) @actor
"#,
```

**Test Cases:**
- Find Swift functions (func, init, subscript)
- Find Swift types (class, struct, enum, protocol, actor)
- Parse Swift syntax (optionals, generics)

**Effort:** 3 hours

#### 2.3 Swift Import Analyzer (Day 6)
**File:** `powertools-cli/src/refactor/imports/swift.rs` (NEW)

**Implementation:**
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
}

impl ImportAnalyzer for SwiftImportAnalyzer {
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>> {
        let content = fs::read_to_string(file)?;
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_swift::LANGUAGE.into())?;

        let tree = parser.parse(&content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Swift file"))?;

        // Query: (import_declaration) @import
        let query = Query::new(
            &tree_sitter_swift::LANGUAGE.into(),
            r#"(import_declaration) @import"#
        )?;

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        let mut imports = Vec::new();
        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                let import_text = &content[node.byte_range()];

                // Parse: "import Foundation" or "import struct Swift.String"
                let module_name = self.extract_module_name(import_text)?;

                imports.push(ImportStatement {
                    source: module_name,
                    symbols: vec![],  // Swift imports entire modules
                    location: Location {
                        file_path: file.to_path_buf(),
                        line: node.start_position().row + 1,
                        column: node.start_position().column + 1,
                        end_line: Some(node.end_position().row + 1),
                        end_column: Some(node.end_position().column + 1),
                    },
                    kind: ImportKind::Module,
                });
            }
        }

        Ok(imports)
    }

    fn add_import(&self, file: &Path, import: &ImportStatement) -> Result<String> {
        let content = fs::read_to_string(file)?;
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

        // Remove line containing "import {symbol}"
        let lines: Vec<&str> = content.lines().collect();
        let filtered: Vec<&str> = lines.into_iter()
            .filter(|line| !line.contains(&format!("import {}", symbol)))
            .collect();

        Ok(filtered.join("\n"))
    }

    fn update_import_path(&self, file: &Path, old: &str, new: &str) -> Result<String> {
        let content = fs::read_to_string(file)?;
        Ok(content.replace(&format!("import {}", old), &format!("import {}", new)))
    }
}

impl SwiftImportAnalyzer {
    fn extract_module_name(&self, import_text: &str) -> Result<String> {
        // "import Foundation" → "Foundation"
        // "import struct Swift.String" → "Swift"
        let parts: Vec<&str> = import_text.split_whitespace().collect();

        if parts.len() >= 2 {
            // Skip "import" keyword and optional kind (struct/class/enum)
            let module = parts.iter()
                .skip(1)
                .find(|p| !matches!(*p, "struct" | "class" | "enum" | "func" | "var" | "let" | "typealias"))
                .ok_or_else(|| anyhow::anyhow!("No module name found"))?;

            // Handle "Swift.String" → "Swift"
            Ok(module.split('.').next().unwrap().to_string())
        } else {
            anyhow::bail!("Invalid import statement: {}", import_text)
        }
    }

    fn find_import_insertion_point(&self, content: &str) -> Result<usize> {
        // Find the last import line, or insert at top
        let lines: Vec<&str> = content.lines().collect();

        for (idx, line) in lines.iter().enumerate().rev() {
            if line.trim().starts_with("import ") {
                // Insert after this import
                let offset: usize = lines.iter().take(idx + 1).map(|l| l.len() + 1).sum();
                return Ok(offset);
            }
        }

        // No imports found, insert at top (after file header comments)
        Ok(self.skip_file_header(content))
    }

    fn skip_file_header(&self, content: &str) -> usize {
        // Skip copyright/license comments at top
        let lines: Vec<&str> = content.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                // First non-comment line
                let offset: usize = lines.iter().take(idx).map(|l| l.len() + 1).sum();
                return offset;
            }
        }

        0
    }
}
```

**Test Cases:**
- Parse `import Foundation`
- Parse `import struct Swift.String`
- Add new import
- Remove existing import
- Handle file header comments

**Effort:** 6 hours

#### 2.4 Swift Inline Variable (Day 7)
**File:** `powertools-cli/src/refactor/inline.rs`

**Add Swift support:**
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

    // Find: let name = value or var name = value
    let var_node = self.find_node_at_position(root_node, target_byte, "property_declaration")
        .ok_or_else(|| anyhow::anyhow!("No variable declaration found"))?;

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

**Add to reference finding:**
```rust
match extension {
    // ... existing languages ...
    "swift" => {
        references = self.find_swift_identifiers(&content, var_name, declaration_line, file_path)?;
    }
    _ => anyhow::bail!("Unsupported file extension: {}", extension),
}
```

**Test Cases:**
- Inline `let` constant
- Reject `var` mutable variable
- Handle optionals (`let x: String? = nil`)
- Handle Swift-specific syntax (nil coalescing `??`)

**Effort:** 6 hours

**Phase 2 Total:** ~24 hours (3 days)

---

## Phase 3: sourcekit-lsp Integration (Days 8-9)

### Goal
Connect Swift LSP backend to existing refactoring infrastructure

### Deliverables

#### 3.1 sourcekit-lsp Configuration (Day 8 morning)
**File:** `powertools-cli/src/indexers/sourcekit_lsp.rs` (NEW)

```rust
use super::lsp_query::LspQuery;
use std::path::PathBuf;
use anyhow::Result;

pub struct SourceKitLsp;

impl SourceKitLsp {
    pub fn new(project_root: PathBuf) -> Result<LspQuery> {
        // sourcekit-lsp expects to be run from project root
        // No additional args needed
        LspQuery::new("sourcekit-lsp", &[], project_root)
    }

    pub fn check_installed() -> bool {
        // Check if sourcekit-lsp is in PATH
        std::process::Command::new("which")
            .arg("sourcekit-lsp")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}
```

**Installation Detection:**
```rust
// In commands/index.rs or similar
if Language::Swift.detected() && !SourceKitLsp::check_installed() {
    eprintln!("⚠️  sourcekit-lsp not found");
    eprintln!("    Install Xcode or Swift toolchain:");
    eprintln!("    https://www.swift.org/download/");
    eprintln!("");
    eprintln!("    sourcekit-lsp is included with Swift 5.6+");
    return Err(anyhow!("sourcekit-lsp required for Swift support"));
}
```

**Effort:** 2 hours

#### 3.2 Rename Symbol with LSP (Day 8 afternoon)
**File:** `powertools-cli/src/refactor/rename.rs`

**Update to use UnifiedQuery:**
```rust
pub struct SymbolRenamer<'a> {
    query: &'a mut UnifiedQuery,  // Changed from ScipQuery
    project_root: PathBuf,
}

impl<'a> SymbolRenamer<'a> {
    pub fn new(query: &'a mut UnifiedQuery, project_root: PathBuf) -> Self {
        Self { query, project_root }
    }

    pub fn rename(&mut self, options: RenameOptions) -> Result<RenameResult> {
        // 1. Find definition (works with both SCIP and LSP)
        let definition = self.query.find_definition(
            &options.file_path,
            options.line,
            options.column,
        )?;

        // 2. Find all references (works with both SCIP and LSP)
        let references = self.query.find_references(
            &options.old_name,
            &options.file_path,
            options.line,
            options.column,
        )?;

        // ... rest of implementation unchanged
    }
}
```

**No changes needed to:**
- Reference replacement logic (works with any backend)
- Import updating (language-specific, not backend-specific)
- Transaction system (backend-agnostic)

**Test Cases:**
- Rename Swift function across files (LSP backend)
- Rename TypeScript function (SCIP backend still works)
- Ensure backend selection is automatic

**Effort:** 4 hours

#### 3.3 Integration Testing (Day 9)
**Test Cases:**

**Swift-specific:**
1. Rename Swift function across 3 files
2. Find references to Swift class
3. Goto definition for Swift method
4. Inline Swift let constant

**Cross-language:**
1. Project with Swift + TypeScript → correct backend per file
2. Refactoring preserves import statements
3. Error messages guide user if sourcekit-lsp missing

**Performance:**
1. Measure LSP query latency (should be 40-100ms)
2. Compare to SCIP (should be 1-5ms for other languages)
3. Ensure no regression for existing languages

**Effort:** 8 hours

**Phase 3 Total:** ~14 hours (1.75 days)

---

## Phase 4: Documentation & Polish (Days 10-11)

### Goal
Document Swift support, limitations, and architecture for users

### Deliverables

#### 4.1 Update CLAUDE.md (Day 10 morning)
**File:** `.claude/CLAUDE.md`

**Add Swift to supported languages:**
```markdown
### Supported Languages:
- **TypeScript**: Full semantic navigation via scip-typescript
- **JavaScript**: Full semantic navigation via scip-typescript
- **Python**: Full semantic navigation via scip-python
- **Rust**: Full semantic navigation via rust-analyzer
- **C++**: Full semantic navigation via scip-clang (requires compile_commands.json)
- **Swift**: Full semantic navigation via sourcekit-lsp (LSP backend) 🆕

**Swift Requirements:**
- Xcode or Swift toolchain installed (includes sourcekit-lsp)
- sourcekit-lsp in PATH (verify with: `which sourcekit-lsp`)
- Swift 5.6 or later

**Swift Architecture:**
- Uses LSP (Language Server Protocol) instead of SCIP
- Slightly slower queries (40-100ms vs 1-5ms) but full cross-file navigation
- No indexing required - sourcekit-lsp analyzes project in real-time
```

**Document performance differences:**
```markdown
### Performance Characteristics by Language:

| Language   | Backend | Index Required | Query Latency | Notes                    |
|------------|---------|----------------|---------------|--------------------------|
| TypeScript | SCIP    | Yes (30-60s)   | 1-5ms         | Fast batch operations    |
| Python     | SCIP    | Yes (30-90s)   | 1-5ms         | Fast batch operations    |
| Rust       | SCIP    | Yes (30-120s)  | 1-5ms         | Fast batch operations    |
| C++        | SCIP    | Yes (60-180s)  | 1-5ms         | Fast batch operations    |
| Swift      | LSP     | No             | 40-100ms      | Fresh data, no re-index  |
```

**Effort:** 2 hours

#### 4.2 Create Swift Testing Guide (Day 10 afternoon)
**File:** `docs/SWIFT_TESTING.md`

**Content:**
- Installation verification steps
- Test project setup (Vapor example)
- Feature checklist (rename, inline, find functions)
- Known limitations
- Troubleshooting guide

**Effort:** 3 hours

#### 4.3 Update MCP Tool Descriptions (Day 11 morning)
**File:** `powertools-cli/src/mcp/tools.rs`

**Update tool descriptions:**
```rust
// In rename_symbol tool:
description: "Rename a symbol across the codebase. Supports TypeScript, Python, Rust, C++, and Swift (via LSP)."

// In goto_definition tool:
description: "Find where a symbol is defined. Uses SCIP indexes for TS/Py/Rust/C++, LSP for Swift."

// Add note about performance:
"Note: Swift queries use LSP and may take 40-100ms (vs 1-5ms for SCIP languages)"
```

**Effort:** 1 hour

#### 4.4 Real-World Testing (Day 11)
**Test on production Swift projects:**

**Vapor (Swift server framework):**
```bash
git clone https://github.com/vapor/vapor.git
cd vapor
powertools rename-symbol Sources/Vapor/Application.swift:50:10 app myApp --preview
```

**Alamofire (HTTP networking):**
```bash
git clone https://github.com/Alamofire/Alamofire.git
cd Alamofire
powertools find-functions Source/Core/Request.swift
```

**Create test results document:**
- Feature coverage matrix
- Performance measurements
- Known issues discovered
- Edge cases handled

**Effort:** 5 hours

**Phase 4 Total:** ~11 hours (1.4 days)

---

## Timeline Summary

| Phase | Days | Hours | Deliverables |
|-------|------|-------|--------------|
| Phase 1: LSP Infrastructure | 3.75 | 30 | Generic LSP client, JSON-RPC, backend abstraction |
| Phase 2: Swift Integration | 3 | 24 | Language enum, tree-sitter, import analyzer, inline variable |
| Phase 3: sourcekit-lsp | 1.75 | 14 | LSP integration, rename symbol, testing |
| Phase 4: Documentation | 1.4 | 11 | CLAUDE.md, testing guide, MCP tools, real-world testing |
| **Total** | **9.9** | **79** | **Full Swift support with LSP backend** |

**Realistic Timeline:** 10-11 days (accounting for debugging, edge cases)

---

## Success Criteria

### Must Have (MVP)
- ✅ sourcekit-lsp integration working
- ✅ Rename symbol across Swift files
- ✅ Goto definition for Swift symbols
- ✅ Find references for Swift symbols
- ✅ Inline variable for Swift `let` constants
- ✅ Function/class finding for Swift
- ✅ Import analyzer for Swift modules
- ✅ Existing languages (TS, Py, Rust, C++) still use SCIP (no regression)
- ✅ Clear documentation of architecture and trade-offs

### Nice to Have (Polish)
- ✅ Performance logging (compare SCIP vs LSP)
- ✅ Automatic sourcekit-lsp installation check
- ✅ Error messages guide to installation
- ✅ LSP connection pooling (reuse server)
- ✅ Graceful degradation if LSP unavailable

### Future Enhancements (v0.5.0+)
- Move symbol refactoring for Swift
- Extract method for Swift
- LSP-based autocomplete (if needed)
- Custom SCIP indexer for Swift (replace LSP)

---

## Risk Mitigation

### Risk 1: sourcekit-lsp Compatibility Issues
**Mitigation:**
- Test on multiple Swift versions (5.6, 5.8, 5.9)
- Document minimum version requirement
- Provide troubleshooting guide

### Risk 2: LSP Performance Concerns
**Mitigation:**
- Implement connection pooling (reuse server)
- Cache frequent queries
- Document performance trade-offs clearly

### Risk 3: Swift AST Complexity
**Mitigation:**
- Start with simple cases (let constants)
- Add complexity incrementally
- Comprehensive test suite

### Risk 4: Mixed Language Projects
**Mitigation:**
- Test iOS projects (Swift + Objective-C)
- Ensure correct backend selection per file
- Handle edge cases gracefully

---

## Dependencies

### New Cargo Dependencies
```toml
[dependencies]
# LSP support
lsp-types = "0.95"         # Official LSP type definitions
serde_json = "1.0"         # JSON-RPC (already have, but ensure compatible)

# Swift tree-sitter
tree-sitter-swift = "0.6"  # Check latest on crates.io

# Error handling
thiserror = "1.0"          # For LspError enum
```

### External Dependencies
- **sourcekit-lsp**: Included with Xcode or Swift toolchain (Swift 5.6+)
- **Swift toolchain**: https://www.swift.org/download/

---

## Testing Strategy

### Unit Tests
- LSP client: start/stop, requests, error handling
- Swift parsers: variable extraction, import parsing
- Backend selection: SCIP vs LSP routing

### Integration Tests
- End-to-end refactoring on Swift projects
- Cross-language project support
- Error handling and recovery

### Performance Tests
- LSP query latency measurements
- Comparison with SCIP baseline
- Memory usage profiling

### Real-World Tests
- Vapor: Server-side Swift
- Alamofire: Networking library
- User's own Swift projects (iOS/macOS)

---

## Next Steps

1. ✅ **Get user approval** - APPROVED
2. **Start Phase 1** - Build LSP client infrastructure
3. **Daily updates** - Track progress, blockers
4. **Milestone reviews** - Demo after each phase
5. **Final release** - v0.4.1 with Swift support

---

## Questions & Decisions

### 2025-10-14: Hybrid Architecture Approved

**Question:** Use LSP for Swift only, or migrate all languages?

**Decision:** ✅ Hybrid approach
- Keep SCIP for TypeScript, Python, Rust, C++ (fast, proven)
- Add LSP for Swift only (no SCIP indexer available)
- Unified backend abstraction for future flexibility

**Rationale:**
- SCIP is 20-500x faster for batch operations
- LSP complexity only justified when SCIP unavailable
- Best of both worlds: performance + coverage

---

**Last Updated:** 2025-10-14
**Status:** 🟢 Starting Phase 1 (LSP Infrastructure)
**Next Update:** End of Phase 1 (Day 4)
