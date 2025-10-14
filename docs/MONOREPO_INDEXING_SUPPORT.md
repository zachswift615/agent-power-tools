# Monorepo Indexing Support Implementation Plan

## Overview

This document outlines the implementation plan for adding monorepo support to powertools. The goal is to enable indexing of multi-package repositories (like TanStack Query) where source code is organized across multiple packages, each with its own configuration files.

**Status**: Planning Phase
**Target Version**: v0.4.0
**Estimated Effort**: 1-2 days implementation + 0.5 day testing

## Problem Statement

Currently, powertools indexing assumes a single-root project structure:
- Detects project type only at root level (checking for `package.json`, `tsconfig.json`, etc.)
- Generates index files at root level (`index.typescript.scip`, etc.)
- Cannot handle monorepos where packages are in subdirectories with their own configs

**Example Issue: TanStack Query**
- Root `tsconfig.json` only contains: `["*.config.*"]`
- Actual source code in `packages/*/` with individual `tsconfig.json` files
- Running `powertools index` at root fails to find any TypeScript source

## Design Decisions

Based on user requirements, the following design has been approved:

### 1. Hybrid Approach with Multi-Root Indexing
- **Detect monorepo structure** at root level
- **Index each package separately** using its own configuration
- **Store index files in package directories** (not root)
- **Accept cross-package reference limitation** (document clearly for users)

### 2. Index File Organization
```
monorepo-root/
├── packages/
│   ├── package-a/
│   │   ├── tsconfig.json
│   │   └── index.typescript.scip  ← Index stored here
│   ├── package-b/
│   │   ├── tsconfig.json
│   │   └── index.typescript.scip  ← Index stored here
│   └── package-c/
│       ├── pyproject.toml
│       └── index.python.scip      ← Index stored here
└── lerna.json / pnpm-workspace.yaml / etc.
```

### 3. Package Identification
- Use **directory name** as package identifier
- Example: `packages/query-core/` → package name is `query-core`
- Fallback to reading `package.json` name field if directory name is generic

### 4. File Watcher Behavior
- **Detect which package changed** based on file path
- **Re-index only the affected package** (not entire monorepo)
- Example: Change in `packages/query-core/src/foo.ts` → only re-index `query-core`

### 5. Known Limitations (To Document)
- ❌ Cross-package references not supported (e.g., `package-a` importing from `package-b`)
- ✅ Within-package navigation works perfectly
- ✅ Query results aggregate across all packages
- ⚠️ Users should run `find_references` knowing results are per-package only

## Implementation Plan

### Phase 1: Monorepo Detection (New Module)

**File**: `powertools-cli/src/indexers/monorepo.rs`

```rust
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct MonorepoPackage {
    pub path: PathBuf,
    pub name: String,
    pub project_types: Vec<ProjectType>,
}

#[derive(Debug)]
pub struct MonorepoDetector {
    root: PathBuf,
}

impl MonorepoDetector {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Detect if the project is a monorepo
    pub fn is_monorepo(&self) -> bool {
        // Check for common monorepo indicators
        self.root.join("lerna.json").exists()
            || self.root.join("pnpm-workspace.yaml").exists()
            || self.root.join("nx.json").exists()
            || self.root.join("turbo.json").exists()
            || self.has_packages_directory()
    }

    fn has_packages_directory(&self) -> bool {
        let packages_dir = self.root.join("packages");
        if !packages_dir.is_dir() {
            return false;
        }

        // Check if any subdirectories have config files
        if let Ok(entries) = std::fs::read_dir(&packages_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if self.has_project_config(&entry.path()) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn has_project_config(&self, dir: &Path) -> bool {
        dir.join("package.json").exists()
            || dir.join("tsconfig.json").exists()
            || dir.join("Cargo.toml").exists()
            || dir.join("pyproject.toml").exists()
    }

    /// Discover all packages in the monorepo
    pub fn discover_packages(&self) -> Result<Vec<MonorepoPackage>> {
        let mut packages = Vec::new();

        // Common monorepo package locations
        let search_paths = vec![
            self.root.join("packages"),
            self.root.join("apps"),
            self.root.join("libs"),
        ];

        for search_path in search_paths {
            if !search_path.is_dir() {
                continue;
            }

            for entry in std::fs::read_dir(&search_path)? {
                let entry = entry?;
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                if let Some(package) = self.analyze_package(path)? {
                    packages.push(package);
                }
            }
        }

        Ok(packages)
    }

    fn analyze_package(&self, path: PathBuf) -> Result<Option<MonorepoPackage>> {
        let project_types = self.detect_project_types(&path);

        if project_types.is_empty() {
            return Ok(None);
        }

        let name = self.infer_package_name(&path)?;

        Ok(Some(MonorepoPackage {
            path,
            name,
            project_types,
        }))
    }

    fn detect_project_types(&self, dir: &Path) -> Vec<ProjectType> {
        let mut types = Vec::new();

        // TypeScript/JavaScript
        if dir.join("tsconfig.json").exists() || dir.join("package.json").exists() {
            types.push(ProjectType::TypeScript);
        }

        // Python
        if dir.join("pyproject.toml").exists()
            || dir.join("setup.py").exists()
            || dir.join("requirements.txt").exists() {
            types.push(ProjectType::Python);
        }

        // Rust
        if dir.join("Cargo.toml").exists() {
            types.push(ProjectType::Rust);
        }

        // C++
        if dir.join("compile_commands.json").exists() {
            types.push(ProjectType::Cpp);
        }

        types
    }

    fn infer_package_name(&self, path: &Path) -> Result<String> {
        // Strategy 1: Read from package.json
        let package_json = path.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                        return Ok(name.to_string());
                    }
                }
            }
        }

        // Strategy 2: Use directory name
        Ok(path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string())
    }
}
```

### Phase 2: Update ScipIndexer for Monorepo Support

**File**: `powertools-cli/src/indexers/scip_indexer.rs`

**Changes needed**:

1. Add monorepo detection to constructor:
```rust
use crate::indexers::monorepo::{MonorepoDetector, MonorepoPackage};

impl ScipIndexer {
    pub fn new(project_root: PathBuf) -> Self {
        let detector = MonorepoDetector::new(project_root.clone());
        let is_monorepo = detector.is_monorepo();

        Self {
            project_root,
            is_monorepo,
            // ... existing fields
        }
    }
}
```

2. Add new method for monorepo indexing:
```rust
impl ScipIndexer {
    /// Generate indexes for all packages in a monorepo
    pub fn generate_indexes_monorepo(&self, filter_languages: Vec<String>) -> Result<Vec<PathBuf>> {
        let detector = MonorepoDetector::new(self.project_root.clone());
        let packages = detector.discover_packages()?;

        eprintln!("[Monorepo] Discovered {} packages", packages.len());

        let mut all_index_paths = Vec::new();

        for package in packages {
            eprintln!("[Monorepo] Indexing package: {} at {:?}", package.name, package.path);

            // Create a temporary ScipIndexer for this package
            let package_indexer = ScipIndexer::new(package.path.clone());

            // Generate indexes for this package
            match package_indexer.generate_indexes(filter_languages.clone()) {
                Ok(index_paths) => {
                    all_index_paths.extend(index_paths);
                }
                Err(e) => {
                    eprintln!("[Monorepo] Failed to index package {}: {}", package.name, e);
                    // Continue with other packages
                }
            }
        }

        Ok(all_index_paths)
    }
}
```

3. Update `generate_indexes()` to delegate to monorepo handler:
```rust
pub fn generate_indexes(&self, filter_languages: Vec<String>) -> Result<Vec<PathBuf>> {
    // Check if this is a monorepo
    let detector = MonorepoDetector::new(self.project_root.clone());
    if detector.is_monorepo() {
        eprintln!("[Index] Detected monorepo structure");
        return self.generate_indexes_monorepo(filter_languages);
    }

    // Single-root indexing (existing code)
    let detected_types = self.detect_project_types();
    // ... rest of existing implementation
}
```

4. Update `get_index_path()` - already works correctly since it uses `self.project_root`:
```rust
fn get_index_path(&self, project_type: &ProjectType) -> PathBuf {
    // This already works correctly for monorepo packages
    // because each package gets its own ScipIndexer with package.path as project_root
    self.project_root.join(format!("index.{}.scip", project_type.extension()))
}
```

### Phase 3: Update ScipQuery for Recursive Index Loading

**File**: `powertools-cli/src/indexers/scip_query_simple.rs`

**Changes needed**:

```rust
impl ScipQuery {
    pub fn from_project(project_root: PathBuf) -> Result<Self> {
        let mut indexes = Vec::new();

        // Check if this is a monorepo
        let detector = MonorepoDetector::new(project_root.clone());

        if detector.is_monorepo() {
            // Load indexes from all packages
            let packages = detector.discover_packages()?;

            for package in packages {
                let package_indexes = Self::load_indexes_from_dir(&package.path)?;
                indexes.extend(package_indexes);
            }
        } else {
            // Single-root: load indexes from project root (existing behavior)
            let root_indexes = Self::load_indexes_from_dir(&project_root)?;
            indexes.extend(root_indexes);
        }

        if indexes.is_empty() {
            return Err(anyhow::anyhow!(
                "No SCIP indexes found. Run `powertools index` first."
            ));
        }

        eprintln!("[Query] Loaded {} SCIP indexes", indexes.len());

        Ok(Self { indexes })
    }

    fn load_indexes_from_dir(dir: &Path) -> Result<Vec<scip::Index>> {
        let mut indexes = Vec::new();

        for filename in &[
            "index.typescript.scip",
            "index.javascript.scip",
            "index.python.scip",
            "index.rust.scip",
            "index.cpp.scip",
            "index.scip", // Legacy fallback
        ] {
            let path = dir.join(filename);
            if path.exists() {
                match Self::load_index(&path) {
                    Ok(index) => {
                        eprintln!("[Query] Loaded index: {:?}", path);
                        indexes.push(index);
                    }
                    Err(e) => {
                        eprintln!("[Query] Failed to load {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(indexes)
    }

    fn load_index(path: &Path) -> Result<scip::Index> {
        use prost::Message;
        let bytes = std::fs::read(path)?;
        let index = scip::Index::decode(&bytes[..])?;
        Ok(index)
    }
}
```

### Phase 4: Update File Watcher for Per-Package Re-indexing

**File**: `powertools-cli/src/watcher/mod.rs`

**Changes needed**:

1. Add package detection to watcher event handler:
```rust
use crate::indexers::monorepo::MonorepoDetector;

pub fn watch_and_reindex(
    project_root: PathBuf,
    auto_install: bool,
) -> Result<()> {
    let detector = MonorepoDetector::new(project_root.clone());
    let is_monorepo = detector.is_monorepo();
    let packages = if is_monorepo {
        Some(detector.discover_packages()?)
    } else {
        None
    };

    // ... existing watcher setup ...

    move |result: DebounceEventResult| {
        match result {
            Ok(events) => {
                for event in events {
                    for path in &event.paths {
                        if is_relevant_file(path) {
                            if let Some(lang) = detect_language_from_path(path) {
                                // Determine which package to re-index
                                if let Some(ref pkgs) = packages {
                                    // Find the package containing this file
                                    if let Some(package) = find_package_for_path(path, pkgs) {
                                        eprintln!("[Watcher] Re-indexing package: {}", package.name);
                                        reindex_package(package, lang, auto_install);
                                        continue;
                                    }
                                }

                                // Fallback: single-root re-indexing
                                eprintln!("[Watcher] Re-indexing language: {:?}", lang);
                                reindex_tx.send(lang).ok();
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("[Watcher] Error: {:?}", e),
        }
    }
}

fn find_package_for_path(
    path: &Path,
    packages: &[MonorepoPackage],
) -> Option<&MonorepoPackage> {
    packages.iter().find(|pkg| path.starts_with(&pkg.path))
}

fn reindex_package(package: &MonorepoPackage, lang: Language, auto_install: bool) {
    // Create indexer for just this package
    let indexer = ScipIndexer::new(package.path.clone());

    // Re-index only the changed language
    let filter = vec![lang.to_string()];

    match indexer.generate_indexes(filter) {
        Ok(paths) => {
            eprintln!("[Watcher] Re-indexed package: {} ({:?})", package.name, paths);
        }
        Err(e) => {
            eprintln!("[Watcher] Re-index failed for {}: {}", package.name, e);
        }
    }
}
```

### Phase 5: Update MCP Server Integration

**File**: `powertools-cli/src/mcp/server.rs`

**Changes needed**:

Update tool descriptions to mention monorepo support:

```rust
fn create_index_project_tool() -> Tool {
    Tool {
        name: "index_project".to_string(),
        description: Some(
            "Index a project for semantic code navigation. \
             Supports TypeScript, JavaScript, Python, Rust, and C++. \
             Automatically detects all languages in the project. \
             For monorepos, indexes all packages and stores indexes in package directories."
                .to_string()
        ),
        // ... rest of tool definition
    }
}
```

Add helpful messages in tool responses:
```rust
// In index_project handler
if detector.is_monorepo() {
    let packages = detector.discover_packages()?;
    response.push_str(&format!(
        "\n[Monorepo] Discovered {} packages\n",
        packages.len()
    ));
    for pkg in &packages {
        response.push_str(&format!("  - {} at {:?}\n", pkg.name, pkg.path));
    }
}
```

### Phase 6: CLI Command Updates

**File**: `powertools-cli/src/main.rs`

Update help text for `index` command:

```rust
#[derive(Parser)]
#[command(
    name = "powertools",
    about = "Code indexing and navigation for AI agents",
    long_about = "Supports monorepo indexing - run at root to index all packages"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index the project for semantic navigation (monorepo-aware)
    Index {
        /// Project root directory (defaults to current directory)
        #[arg(short = 'p', long)]
        project: Option<PathBuf>,

        /// Auto-install missing indexers
        #[arg(long)]
        auto_install: bool,

        /// Filter by specific languages (e.g., typescript, python)
        #[arg(long, value_delimiter = ',')]
        languages: Vec<String>,
    },
    // ... other commands
}
```

## Testing Strategy

### Test Case 1: TanStack Query (TypeScript Monorepo)

**Setup**:
```bash
git clone https://github.com/TanStack/query.git
cd query
```

**Test Steps**:
1. Run `powertools index --auto-install` at root
2. Verify indexes created in `packages/*/index.typescript.scip`
3. Run `powertools functions --format json` - should list functions from all packages
4. Run `powertools references QueryClient` - should find references within packages
5. Modify a file in `packages/query-core/src/queryClient.ts`
6. Verify watcher only re-indexes `query-core` package (not all packages)

**Expected Results**:
- ✅ Indexes created: `packages/query-core/index.typescript.scip`, `packages/react-query/index.typescript.scip`, etc.
- ✅ `list_functions` returns functions from all packages
- ✅ `find_references` finds references within each package
- ⚠️ Cross-package references not found (documented limitation)

### Test Case 2: Mixed-Language Monorepo

**Setup**: Create test monorepo:
```
test-monorepo/
├── packages/
│   ├── ts-app/
│   │   ├── tsconfig.json
│   │   └── src/index.ts
│   ├── py-lib/
│   │   ├── pyproject.toml
│   │   └── src/main.py
│   └── rust-cli/
│       ├── Cargo.toml
│       └── src/main.rs
└── pnpm-workspace.yaml
```

**Test Steps**:
1. Run `powertools index --auto-install` at root
2. Verify multiple language indexes created in respective packages
3. Query across languages with `powertools functions`

**Expected Results**:
- ✅ `packages/ts-app/index.typescript.scip` created
- ✅ `packages/py-lib/index.python.scip` created
- ✅ `packages/rust-cli/index.rust.scip` created
- ✅ Queries aggregate results across all languages

### Test Case 3: Single-Root Regression Testing

**Setup**: Use existing single-root projects (agent-power-tools, etc.)

**Test Steps**:
1. Run `powertools index --auto-install` on single-root projects
2. Verify behavior unchanged from current version

**Expected Results**:
- ✅ No regressions in single-root indexing
- ✅ Index files still created at project root
- ✅ All queries work as before

### Test Case 4: File Watcher in Monorepo

**Setup**: TanStack Query with watcher running

**Test Steps**:
1. Start MCP server (watcher auto-starts)
2. Modify `packages/query-core/src/queryClient.ts`
3. Wait for re-index (2-5 seconds)
4. Verify only `query-core` package re-indexed (check timestamps on index files)

**Expected Results**:
- ✅ Only `packages/query-core/index.typescript.scip` timestamp updated
- ✅ Other package indexes unchanged
- ✅ Re-index completes in ~5s (not 30s for full monorepo)

## Documentation Updates

### 1. README.md

Add monorepo section:

```markdown
## Monorepo Support (NEW in v0.4.0)

Powertools now supports monorepo indexing! Run `powertools index` at the root of any monorepo and it will:
- Automatically detect all packages
- Index each package with its own configuration
- Store indexes in package directories
- Enable per-package incremental re-indexing

### Supported Monorepo Structures

- ✅ Lerna (detects `lerna.json`)
- ✅ pnpm workspaces (detects `pnpm-workspace.yaml`)
- ✅ Nx (detects `nx.json`)
- ✅ Turborepo (detects `turbo.json`)
- ✅ Generic `packages/` directories

### Example: TanStack Query

```bash
git clone https://github.com/TanStack/query.git
cd query
powertools index --auto-install

# Indexes created:
# - packages/query-core/index.typescript.scip
# - packages/react-query/index.typescript.scip
# - packages/vue-query/index.typescript.scip
# ... etc

# Query across all packages
powertools functions --format json
powertools references QueryClient
```

### Known Limitations

- ❌ **Cross-package references not supported**: References from `package-a` to symbols in `package-b` won't be found
- ✅ **Within-package navigation works perfectly**: All queries within a single package work as expected
- ✅ **Aggregated queries**: Results from all packages are combined for `list_functions`, `list_classes`, etc.

For most use cases (editing a single package at a time), monorepo indexing works great!
```

### 2. .claude/CLAUDE.md

Update the "When to use" section:

```markdown
**Monorepo Support (v0.4.0+):**
- Run `index_project` at monorepo root to index all packages
- Each package gets its own index file stored in its directory
- File watcher only re-indexes changed packages (efficient!)
- **Limitation**: Cross-package references not supported (within-package only)
```

### 3. MCP Tool Descriptions

Update `index_project` tool description to mention monorepo support and limitations.

## Success Criteria

- ✅ Can run `powertools index` at TanStack Query root and successfully index all packages
- ✅ Index files created in package directories (e.g., `packages/query-core/index.typescript.scip`)
- ✅ `find_references` works within individual packages
- ✅ `list_functions` aggregates results across all packages
- ✅ File watcher only re-indexes affected package (not entire monorepo)
- ✅ Single-root projects continue to work without regression
- ✅ Clear documentation about cross-package reference limitation
- ✅ MCP server integration works seamlessly with monorepos

## Implementation Checklist

### Core Implementation
- [ ] Create `src/indexers/monorepo.rs` with detection and package discovery
- [ ] Add monorepo detection to `ScipIndexer::new()`
- [ ] Implement `ScipIndexer::generate_indexes_monorepo()`
- [ ] Update `ScipIndexer::generate_indexes()` to delegate to monorepo handler
- [ ] Update `ScipQuery::from_project()` with recursive index loading
- [ ] Add `ScipQuery::load_indexes_from_dir()` helper method

### File Watcher Updates
- [ ] Update watcher event handler to detect affected package
- [ ] Implement `find_package_for_path()` helper
- [ ] Implement `reindex_package()` for targeted re-indexing
- [ ] Test debouncing behavior in monorepo context

### CLI and MCP Updates
- [ ] Update CLI help text for `index` command
- [ ] Update MCP tool descriptions
- [ ] Add monorepo detection messages in tool responses

### Testing
- [ ] Test on TanStack Query (TypeScript monorepo)
- [ ] Test on mixed-language monorepo
- [ ] Regression test on single-root projects
- [ ] Test file watcher per-package re-indexing
- [ ] Verify cross-package limitation and document behavior

### Documentation
- [ ] Add monorepo section to README.md
- [ ] Update .claude/CLAUDE.md with monorepo usage
- [ ] Document known limitations clearly
- [ ] Add troubleshooting section for monorepos

## Troubleshooting

### Issue: Packages not detected

**Symptoms**: Running `powertools index` at monorepo root doesn't find packages

**Diagnosis**:
```bash
# Check for monorepo indicators
ls -la | grep -E "lerna.json|pnpm-workspace.yaml|nx.json|turbo.json"

# Check packages directory
ls -la packages/
```

**Solutions**:
- Ensure `packages/` directory exists
- Verify each package has a config file (`package.json`, `tsconfig.json`, etc.)
- Check if monorepo uses non-standard directory names (e.g., `apps/`, `libs/`)

### Issue: Cross-package references not found

**Symptoms**: `find_references` doesn't return results from other packages

**This is expected behavior**:
- Cross-package references are a documented limitation
- Each package is indexed independently
- Workaround: Use grep-based search for cross-package usage tracking

### Issue: Slow re-indexing in monorepo

**Symptoms**: File watcher re-indexes entire monorepo on single file change

**Diagnosis**: Check watcher logs to see which packages are being re-indexed

**Solutions**:
- Verify `find_package_for_path()` is correctly identifying affected package
- Check for overlapping package paths
- Ensure debounce delay is sufficient (default 2s)

## Future Enhancements

### Post-v0.4.0 Improvements

1. **Cross-Package Reference Support**
   - Combine indexes at query time with package context
   - Use relative paths to resolve cross-package imports
   - Requires LSP-style workspace resolution

2. **Workspace-Level Index**
   - Generate a workspace-level index that links packages
   - Store package dependency graph
   - Enable "find implementations" across packages

3. **Smart Package Discovery**
   - Read workspace configuration files (pnpm-workspace.yaml, lerna.json)
   - Respect workspace globs (e.g., `packages/*`, `apps/*`)
   - Handle nested workspaces

4. **Incremental Monorepo Indexing**
   - Only re-index files that changed (not entire package)
   - Use SCIP index merging
   - Requires file-level granularity in indexers

## References

- **TanStack Query**: https://github.com/TanStack/query (test case)
- **SCIP Protocol**: https://github.com/sourcegraph/scip
- **Monorepo Tools**: Lerna, pnpm, Nx, Turborepo
- **Related Issue**: Discovered during v0.4.0 refactoring testing (lines 860-873 in SEMANTIC_REFACTORING_V0.4.0.md)
