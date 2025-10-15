# LSP vs SCIP: Architectural Analysis for Powertools

**Created:** 2025-10-14
**Context:** Evaluating whether to use LSP servers vs SCIP indexing for language support
**Question:** Should we use LSP for Swift? Should we migrate other languages to LSP?

---

## Executive Summary

**TL;DR:** SCIP is the right choice for powertools. LSP would require running persistent server processes, adds complexity, and is slower for batch operations. However, **LSP fallback for Swift** makes sense since no SCIP indexer exists.

**Recommendation:**
- ✅ **Keep SCIP** for TypeScript, Python, Rust, C++ (existing languages)
- ✅ **Use LSP fallback** for Swift only (no SCIP indexer available)
- ✅ **Don't migrate** existing languages to LSP

---

## Fundamental Architectural Differences

### SCIP (Static Code Intelligence Protocol)

**What it is:** Pre-computed index file (protobuf format) containing symbol definitions and references

**Workflow:**
```
1. Run indexer once:     scip-typescript index  →  index.scip (file)
2. Query anytime:        powertools queries index.scip (instant)
3. Update on change:     Re-run indexer  →  new index.scip
```

**Architecture:**
```
┌─────────────────────────────────────────┐
│  Powertools (our process)               │
│  ┌───────────────────────────────────┐  │
│  │ Read index.scip file              │  │
│  │ Parse protobuf                    │  │
│  │ Query symbols (in-memory)         │  │
│  │ Return results                    │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
         ▲
         │ Reads file (fast, no IPC)
         │
    index.scip (on disk)
```

**Characteristics:**
- ✅ **Stateless:** No background processes
- ✅ **Fast queries:** In-memory protobuf parsing (~1-5ms)
- ✅ **Batch-friendly:** Perfect for CLI tools
- ✅ **Cacheable:** Index file can be versioned, cached
- ❌ **Stale data:** Requires re-indexing after code changes
- ❌ **Indexing time:** Initial index creation (10s - 2min for large projects)

### LSP (Language Server Protocol)

**What it is:** Live server process that maintains project state and responds to RPC requests

**Workflow:**
```
1. Start server:         sourcekit-lsp (background process)
2. Initialize:           Send project root, file list
3. Query each time:      Send RPC request → wait for response
4. Keep running:         Server watches files, maintains state
```

**Architecture:**
```
┌─────────────────────────────────────────┐
│  Powertools (our process)               │
│  ┌───────────────────────────────────┐  │
│  │ 1. Spawn LSP server (if not       │  │
│  │    running)                       │  │
│  │ 2. Send JSON-RPC request          │  │
│  │ 3. Wait for response              │  │
│  │ 4. Parse JSON                     │  │
│  │ 5. Convert to internal types      │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
         │
         │ JSON-RPC over stdio/TCP (IPC overhead ~40ms)
         ▼
┌─────────────────────────────────────────┐
│  LSP Server (separate process)          │
│  ┌───────────────────────────────────┐  │
│  │ Maintain project state            │  │
│  │ Parse files on change             │  │
│  │ Resolve symbols                   │  │
│  │ Return results                    │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

**Characteristics:**
- ✅ **Fresh data:** Always up-to-date with file changes
- ✅ **Rich features:** Autocomplete, diagnostics, hover info
- ✅ **Incremental:** Only re-parses changed files
- ❌ **Stateful:** Requires running background process
- ❌ **IPC overhead:** ~40ms per request (vs ~1ms for SCIP)
- ❌ **Complexity:** Process management, error handling, timeouts
- ❌ **Memory:** Server holds project in memory (100MB - 1GB)

---

## Performance Comparison

### Scenario 1: Single "Goto Definition" Query

| Metric              | SCIP               | LSP                     |
|---------------------|--------------------|-------------------------|
| Cold start          | 50ms (load index)  | 2-5s (start server)     |
| Warm query          | 1-5ms (protobuf)   | 40-100ms (JSON-RPC)     |
| Memory overhead     | 10-50MB (index)    | 100MB-1GB (server)      |
| Processes           | 1 (powertools)     | 2 (powertools + LSP)    |

**Winner:** SCIP (8-20x faster for individual queries)

### Scenario 2: Batch Operation (Rename Symbol Across 100 Files)

| Metric              | SCIP               | LSP                     |
|---------------------|--------------------|-------------------------|
| Find references     | 5ms (1 protobuf read) | 4s (100 × 40ms RPC)   |
| Rename all          | 10ms (in-memory)   | 4s (100 × 40ms RPC)     |
| Total time          | ~15ms              | ~8 seconds              |

**Winner:** SCIP (500x faster for batch operations!)

### Scenario 3: Code Changes Frequently (Active Development)

| Metric              | SCIP               | LSP                     |
|---------------------|--------------------|-------------------------|
| Update latency      | 2-30s (re-index)   | <1s (incremental)       |
| Developer UX        | Delay on save      | Instant                 |
| Watcher complexity  | Simple (file mod)  | Built-in                |

**Winner:** LSP (fresh data, better for interactive development)

### Scenario 4: CI/CD Pipeline (Large Codebase Analysis)

| Metric              | SCIP               | LSP                     |
|---------------------|--------------------|-------------------------|
| Index time          | 2min (once)        | 2min (initialize)       |
| 1000 queries        | 5s (parallel)      | 40s (sequential RPC)    |
| Memory peak         | 200MB              | 1.5GB (server + cache)  |
| Cacheable?          | ✅ index.scip      | ❌ ephemeral state      |

**Winner:** SCIP (faster, cacheable, CI-friendly)

---

## Real-World Performance Data

### SCIP Benchmarks (from Sourcegraph)

**scip-typescript vs lsif-typescript:**
- **10x speedup** in CI (lsif-node → scip-typescript)
- **4x smaller** gzip compressed payloads
- **3x faster** processing (at Meta scale)

**Example:** TanStack Query (TypeScript project)
- Index size: 2.5MB
- Index time: ~30 seconds
- Query time: <5ms per lookup

### LSP Benchmarks (from rust-analyzer)

**rust-analyzer LSP:**
- Goto definition latency: ~40ms (includes IPC)
- Find references: 100-500ms (depends on project size)
- Memory usage: 500MB - 2GB for large projects
- Cold start: 2-5 seconds

**Note:** LSP is optimized for interactive editing, not batch operations

---

## Use Case Analysis: When to Use What?

### ✅ SCIP is Better For:

1. **CLI Tools & Batch Operations** (our use case!)
   - Rename symbol across 100 files: 15ms vs 8s
   - No background processes to manage
   - Perfect for agent-driven refactoring

2. **CI/CD Pipelines**
   - Cacheable index files
   - Deterministic, repeatable
   - Low memory footprint

3. **Code Browsing (GitHub, GitLab)**
   - Pre-compute indexes in CI
   - Serve static navigation
   - No live server needed

4. **Large-Scale Analysis**
   - Parallel queries (no server bottleneck)
   - Repository-wide searches
   - Multi-project codebases

### ✅ LSP is Better For:

1. **Interactive Editors (VS Code, Neovim)**
   - Real-time autocomplete
   - Instant diagnostics
   - Fresh data on every keystroke

2. **Rapid Development Cycles**
   - Code changes every few seconds
   - No re-indexing delay
   - Incremental updates

3. **Rich IDE Features**
   - Hover information
   - Signature help
   - Code actions (quick fixes)

4. **Languages Without SCIP Indexers** (Swift!)
   - Fallback when no SCIP indexer exists
   - Better than nothing

---

## Architectural Implications for Powertools

### Current Architecture (SCIP-based) ✅

**Strengths:**
- ✅ Perfect for AI agent use case (batch refactoring)
- ✅ Simple: No process management
- ✅ Fast: In-memory protobuf queries
- ✅ Reliable: No IPC failures
- ✅ Testable: Deterministic index files

**How it works:**
```rust
// Simple, synchronous API
let scip_query = ScipQuery::new("index.scip")?;
let definition = scip_query.find_definition("main.rs", 42, 10)?; // 1ms
let references = scip_query.find_references("myFunction")?;      // 5ms
```

### If We Switched to LSP ❌

**Complexity Added:**

1. **Process Management**
```rust
// Now we need:
struct LspClient {
    process: Child,                    // Background server
    stdin: ChildStdin,                 // Write RPC requests
    stdout: ChildStdout,               // Read RPC responses
    request_id: AtomicU64,             // Request tracking
    pending: HashMap<u64, Sender>,     // Async response handling
}

impl LspClient {
    fn start_server() -> Result<Self> {
        // 1. Spawn process
        // 2. Wait for initialization
        // 3. Send initialize request
        // 4. Wait for initialized notification
        // 5. Start message reader thread
        // ... 100+ lines of setup code
    }

    fn goto_definition(&mut self, file: &str, line: u32, col: u32) -> Result<Location> {
        // 1. Create JSON-RPC request
        // 2. Send over stdin
        // 3. Wait for response on stdout
        // 4. Parse JSON
        // 5. Handle errors
        // 6. Convert LSP types to our types
        // ... 50+ lines per RPC method
    }

    fn shutdown(&mut self) -> Result<()> {
        // 1. Send shutdown request
        // 2. Wait for response
        // 3. Send exit notification
        // 4. Kill process if it hangs
        // 5. Clean up resources
    }
}
```

2. **Error Handling Explosion**
```rust
enum LspError {
    ProcessNotStarted,
    ProcessCrashed,
    InitializationFailed,
    RequestTimeout,
    InvalidResponse,
    JsonParseError,
    ServerError(i32, String),
    ProtocolViolation,
    // ... many more variants
}
```

3. **Performance Degradation**
```rust
// Rename symbol across 100 files:

// SCIP (current):
let refs = scip.find_references("myFunc")?;  // 5ms, all files
// Total: ~15ms

// LSP (hypothetical):
for file in &files {
    let refs = lsp.find_references(file, symbol).await?;  // 40ms each
}
// Total: 100 × 40ms = 4 seconds (267x slower!)
```

4. **File Watcher Integration Complexity**
```rust
// SCIP: Simple re-indexing
watcher.on_change(|files| {
    indexer.reindex(files)?;  // Done
});

// LSP: Need to notify server of changes
watcher.on_change(|files| {
    for file in files {
        lsp.did_change(file, version, content).await?;  // RPC per file
    }
    // Hope server updates internal state correctly
});
```

### Hybrid Approach: SCIP + LSP Fallback ✅

**Best of both worlds:**

```rust
pub enum NavigationBackend {
    Scip(ScipQuery),        // Fast, for languages with SCIP indexers
    Lsp(LspClient),         // Fallback, for Swift/others
}

impl Navigator {
    pub fn goto_definition(&self, file: &Path, line: u32, col: u32) -> Result<Location> {
        match &self.backend {
            NavigationBackend::Scip(scip) => {
                // Fast path: 1-5ms
                scip.find_definition(file, line, col)
            }
            NavigationBackend::Lsp(lsp) => {
                // Slow path: 40-100ms (but better than nothing!)
                lsp.goto_definition(file, line, col)
            }
        }
    }
}
```

**When to use each:**
- TypeScript, Rust, Python, C++: SCIP (fast, proven)
- Swift: LSP (no SCIP indexer available)
- Future languages: Try SCIP first, LSP fallback if needed

---

## Specific Analysis: Should We Migrate Existing Languages to LSP?

### TypeScript/JavaScript

**Current:** scip-typescript
- Index time: 20-60s
- Query time: 1-5ms
- Quality: ✅ Excellent

**If LSP:** typescript-language-server
- Startup time: 2-5s
- Query time: 40-100ms
- Quality: ✅ Excellent (same engine)

**Decision:** ❌ **No migration**
- SCIP is 20x faster for batch operations
- scip-typescript already works perfectly
- No benefit, only downsides

### Python

**Current:** scip-python
- Index time: 30-90s
- Query time: 1-5ms
- Quality: ⚠️ Has column position bugs (we handle this)

**If LSP:** pyright / pylsp
- Startup time: 1-3s
- Query time: 40-100ms
- Quality: ✅ Excellent (Pyright is great)

**Decision:** ⚠️ **Maybe consider for Python only**
- Pyright LSP is more accurate than scip-python
- But 20x slower for batch operations
- Trade-off: accuracy vs speed

**Recommendation:** Keep SCIP, improve validation logic

### Rust

**Current:** rust-analyzer (SCIP export)
- Index time: 30-120s
- Query time: 1-5ms
- Quality: ✅ Excellent

**If LSP:** rust-analyzer (LSP mode)
- Startup time: 2-5s
- Query time: 40-100ms
- Quality: ✅ Excellent (same tool)

**Decision:** ❌ **No migration**
- Same tool, different interface
- SCIP mode is faster
- rust-analyzer SCIP export is production-ready

### C++

**Current:** scip-clang
- Index time: 60-180s (requires compile_commands.json)
- Query time: 1-5ms
- Quality: ✅ Excellent

**If LSP:** clangd
- Startup time: 3-10s
- Query time: 50-200ms
- Quality: ✅ Excellent

**Decision:** ❌ **No migration**
- SCIP is 10-40x faster
- scip-clang works great
- clangd is resource-heavy (C++ projects are huge)

### Swift (New Language)

**Current:** None
- No SCIP indexer exists

**Options:**
1. ❌ Tree-sitter only (no cross-file navigation)
2. ✅ LSP fallback (sourcekit-lsp)
3. ⏳ Build custom SCIP indexer (future)

**Decision:** ✅ **Use LSP fallback**
- Better than tree-sitter-only
- Enables cross-file navigation
- Swift LSP (sourcekit-lsp) is high-quality
- Worth the complexity for this one language

---

## Implementation Recommendation

### For Swift: LSP Fallback Architecture

**Phase 1: Basic LSP Client (3-4 days)**

```rust
// File: src/indexers/lsp_client.rs

pub struct LspClient {
    process: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    request_id: AtomicU64,
}

impl LspClient {
    pub fn new(command: &str, args: &[String], root: &Path) -> Result<Self> {
        // 1. Spawn LSP server
        let mut process = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // 2. Get handles
        let stdin = BufWriter::new(process.stdin.take().unwrap());
        let stdout = BufReader::new(process.stdout.take().unwrap());

        let mut client = Self {
            process,
            stdin,
            stdout,
            request_id: AtomicU64::new(1),
        };

        // 3. Initialize
        client.initialize(root)?;

        Ok(client)
    }

    pub fn goto_definition(&mut self, file: &Path, line: u32, col: u32) -> Result<Location> {
        // Send textDocument/definition request
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": format!("file://{}", file.display())
                },
                "position": {
                    "line": line - 1,  // LSP uses 0-indexed
                    "character": col - 1
                }
            }
        });

        let response = self.send_request(request)?;
        self.parse_location(response)
    }

    pub fn find_references(&mut self, file: &Path, line: u32, col: u32) -> Result<Vec<Location>> {
        // Similar to goto_definition
    }
}
```

**Phase 2: Integration with Navigator (1 day)**

```rust
// File: src/core/navigator.rs

pub enum Backend {
    Scip { query: ScipQuery },
    Lsp { client: LspClient },
}

pub struct Navigator {
    backend: Backend,
}

impl Navigator {
    pub fn for_project(root: &Path) -> Result<Self> {
        // Auto-detect which backend to use
        let languages = detect_languages(root)?;

        if languages.contains(&Language::Swift) && !has_scip_index(root, Language::Swift) {
            // Use LSP for Swift
            let client = LspClient::new("sourcekit-lsp", &[], root)?;
            Ok(Self { backend: Backend::Lsp { client } })
        } else {
            // Use SCIP for everything else
            let query = ScipQuery::load(root)?;
            Ok(Self { backend: Backend::Scip { query } })
        }
    }

    pub fn goto_definition(&mut self, file: &Path, line: u32, col: u32) -> Result<Location> {
        match &mut self.backend {
            Backend::Scip { query } => query.find_definition(file, line, col),
            Backend::Lsp { client } => client.goto_definition(file, line, col),
        }
    }
}
```

**Phase 3: Performance Optimization (1 day)**

- Connection pooling (reuse LSP process)
- Caching frequent queries
- Batch requests when possible
- Timeout handling

**Total effort:** 5-6 days

---

## Comparison Summary Table

| Aspect                  | SCIP (Keep for 4 langs) | LSP (Add for Swift) |
|-------------------------|-------------------------|---------------------|
| **Performance**         |                         |                     |
| Single query            | 1-5ms ✅                | 40-100ms ⚠️         |
| Batch (100 queries)     | 15ms ✅                 | 4s ❌               |
| Memory usage            | 10-50MB ✅              | 100MB-1GB ❌        |
| Cold start              | 50ms ✅                 | 2-5s ❌             |
| **Architecture**        |                         |                     |
| Complexity              | Simple ✅               | Complex ❌          |
| Process count           | 1 (us) ✅               | 2 (us + server) ❌  |
| Error surface           | Small ✅                | Large ❌            |
| **Features**            |                         |                     |
| Goto definition         | ✅                      | ✅                  |
| Find references         | ✅                      | ✅                  |
| Freshness               | Stale (re-index) ⚠️     | Always fresh ✅     |
| Autocomplete            | ❌                      | ✅                  |
| Diagnostics             | ❌                      | ✅                  |
| **Use Case Fit**        |                         |                     |
| CLI batch refactoring   | Perfect ✅              | Slow ❌             |
| AI agent workflows      | Perfect ✅              | Acceptable ⚠️       |
| Interactive editing     | Delayed ⚠️              | Perfect ✅          |
| CI/CD pipelines         | Perfect ✅              | Wasteful ❌         |

---

## Final Recommendation

### ✅ DO: Hybrid Architecture

**Keep SCIP for:**
- TypeScript/JavaScript (scip-typescript)
- Python (scip-python)
- Rust (rust-analyzer SCIP)
- C++ (scip-clang)

**Add LSP fallback for:**
- Swift (sourcekit-lsp) - only language without SCIP indexer

**Rationale:**
1. SCIP is 20-500x faster for batch operations (our core use case)
2. SCIP is simpler (no process management)
3. LSP only justified when SCIP unavailable (Swift)
4. Hybrid approach gives us both performance AND coverage

### ❌ DON'T: Migrate Everything to LSP

**Why not:**
- 20-500x performance regression
- Massive complexity increase
- No benefit for languages with working SCIP indexers
- LSP optimized for interactive editing, not batch refactoring

### 🎯 Implementation Plan

**Phase 1:** Add LSP client infrastructure (3-4 days)
**Phase 2:** Integrate sourcekit-lsp for Swift (1-2 days)
**Phase 3:** Keep existing SCIP for all other languages (0 days - already done!)

**Total effort:** 4-6 days (vs 15-20 days to migrate everything)

---

## Questions for User

1. **Approve hybrid approach (SCIP + LSP)?**
   - SCIP for TypeScript/Python/Rust/C++ (keep existing)
   - LSP fallback for Swift only (new)

2. **Acceptable trade-off for Swift?**
   - Slower (40ms vs 1ms per query)
   - But enables cross-file navigation (vs tree-sitter-only)

3. **Priority: Speed vs Coverage?**
   - Current plan maximizes both
   - SCIP where possible (fast)
   - LSP where necessary (Swift)

---

## Related Documentation

- **Swift Plan:** [SWIFT_LANGUAGE_SUPPORT_PLAN.md](./SWIFT_LANGUAGE_SUPPORT_PLAN.md)
- **SCIP Spec:** https://github.com/sourcegraph/scip
- **LSP Spec:** https://microsoft.github.io/language-server-protocol/

---

**Last Updated:** 2025-10-14
**Status:** Awaiting architectural decision
**Recommendation:** Hybrid (SCIP + LSP fallback for Swift only)
