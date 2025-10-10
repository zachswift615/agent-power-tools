# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2025-10-09

### Added
- **Production Testing & Documentation**: Comprehensive real-world testing across all 4 languages
  - Tested `batch-replace` on open-source projects: TanStack Query (TypeScript), powertools (Rust), poetry-core (Python), nlohmann/json (C++)
  - Validated regex capture groups, alternation, character classes, word boundaries across all languages
  - Tested large-scale operations (74 files, 589 changes in Python project)
  - Added comprehensive testing documentation to `SEMANTIC_REFACTORING_V0.4.0.md`
  - Updated README.md with detailed batch-replace examples and usage

- **Documentation Improvements**:
  - Added dedicated "Batch Replace Examples" section to README
  - Documented all advanced features (capture groups, file filtering, risk assessment)
  - Added 6 real-world usage examples (typo fixes, API URLs, JSDoc, type hints, copyright)
  - Updated MCP tools list to include `batch_replace`

### Changed
- Enhanced README.md with production-ready batch-replace documentation
- Reorganized CLI examples to highlight batch operations

### Technical Validation
- ✅ TypeScript: 18 files, 138 changes (regex + capture groups)
- ✅ Rust: 3 files, 9 changes (method signatures)
- ✅ Python: 74 files, 589 changes (largest scale test)
- ✅ C++: 46 files, 50 changes (copyright updates)
- ✅ All safety features validated: preview mode, ignore patterns, risk assessment, JSON output

## [0.3.0] - 2025-10-08

### Added
- **Batch File Operations**: Replace text across multiple files with regex patterns
  - New `batch-replace` CLI command with preview and apply modes
  - Regex pattern matching with capture group support ($1, $2, etc.)
  - File glob filtering (e.g., `*.ts`, `**/*.rs`) to limit scope
  - Beautiful preview diffs showing exact changes before applying
  - Smart ignore patterns (.git, target, node_modules, etc.)
  - JSON output for programmatic usage
  - New MCP tool: `batch_replace` with preview-first safety

- **Refactoring Infrastructure**: Foundation for semantic refactoring tools
  - New `src/refactor/` module with preview and replacement systems
  - Preview system with diff formatting and change summaries
  - Batch result tracking (files scanned, matched, modified)
  - File traversal with ignore pattern support

### Changed
- Added `regex` dependency (v1.11) for pattern matching

### Technical Details
- Built on `regex` crate (same as ripgrep) for fast, reliable pattern matching
- Respects .gitignore-style patterns during file traversal
- Preview-first workflow prevents accidental mass edits
- Thread-safe for future parallel processing

### Examples
```bash
# Preview changes across TypeScript files
powertools batch-replace "console\.log" "logger.debug" --files "**/*.ts" --preview

# Apply typo fix across entire codebase
powertools batch-replace "recieve" "receive" --files "**/*.md"

# Use capture groups for complex replacements
powertools batch-replace "user\.([a-zA-Z]+)" "user?.$1" --files "**/*.ts" --preview
```

## [0.2.0] - 2025-10-08

### Added
- **File Watching and Auto Re-indexing**: Automatic background re-indexing when source files change
  - New `watch` CLI command for manual file watching with configurable debounce
  - MCP server now starts file watcher automatically on startup
  - Language-specific re-indexing: Only re-indexes the changed language (not all languages)
  - Smart debouncing (2s default) prevents re-index spam during rapid file changes
  - Respects ignore patterns: `.git/`, `target/`, `node_modules/`, `*.scip`, etc.
  - New MCP tools: `watcher_start`, `watcher_stop`, `get_watcher_status`

- **Index Metadata and Validation**: Track index freshness and detect staleness
  - Metadata stored alongside each SCIP index (`.scip.meta` files)
  - Tracks creation time, file count, and hash of source files
  - Enables smart staleness detection to avoid unnecessary re-indexing

- **Language-Specific Reindexing**: Added `reindex_language()` method to ScipIndexer
  - Supports incremental workflows for multi-language projects
  - Significantly faster than full re-indexing (5s vs 30s on mixed projects)

### Changed
- **MCP Server**: Now requires project root path and automatically starts file watcher
- **ScipIndexer**: Enhanced with `ProjectType` enum and language-specific indexing methods

### Technical Details
- Added dependencies: `notify` (v7.0), `notify-debouncer-full` (v0.3), `crossbeam-channel` (v0.5)
- New module: `src/watcher/` with filters, metadata, and FileWatcher implementation
- File watcher uses debounced events to batch rapid changes
- Watcher state managed with Arc<Mutex<>> for thread-safe MCP integration

## [0.1.6] - 2025-10-08

### Added
- **C++ Tree-sitter Support**: Added tree-sitter queries for C++ functions and classes
  - `list_functions` now works with C++ files (extracts function names from qualified identifiers)
  - `list_classes` now works with C++ files (finds class and struct definitions)
  - Custom name extraction for C++ qualified names (e.g., `ClassName::functionName`)

### Fixed
- **C++ Function/Class Finding**: Tree-sitter-based tools now properly parse C++ code
  - Added function_definition and class_specifier/struct_specifier queries
  - Implemented C++-specific name extraction logic for complex declarators

## [0.1.5] - 2025-10-08

### Fixed
- **C++ Semantic Navigation**: Fixed SCIP query tools not finding C++ index files
  - Added `index.cpp.scip` to the file lookup list in `ScipQuery::from_project()`
  - Fixes "No SCIP indexes found" errors for C++ projects
  - All C++ semantic tools now work: `goto_definition`, `find_references`

## [0.1.4] - 2025-10-08

### Added
- **C++ Language Support**: Full SCIP-based semantic navigation for C++ projects
  - Auto-installation of scip-clang v0.3.2 binary
  - Multi-platform support (macOS Apple Silicon/Intel via Rosetta, Linux x86_64)
  - C++ project detection via `compile_commands.json`, `CMakeLists.txt`, or `.cpp` files
  - Auto-download and installation to `~/.local/bin`
- Added `dirs` dependency for home directory detection

### Changed
- Updated `check_indexer_installed()` to check both PATH and `~/.local/bin`
- Enhanced error messages for missing compilation database

### Fixed
- PATH handling for locally installed indexers in `~/.local/bin`

## [0.1.3] - 2025-10-08

### Added
- **Pagination Support**: All MCP tools now support pagination to prevent token limit errors
  - `limit` parameter (default: 100) to control max results
  - `offset` parameter (default: 0) to skip results
  - Response includes `count`, `has_more`, and result data
- Quick Start section in README

### Changed
- **Documentation Overhaul**: Updated README with complete feature set
  - Moved SCIP features from "In Progress" to "Implemented"
  - Reorganized usage section with MCP Server first (recommended approach)
  - Updated language support table with auto-install package names
  - Added real-world performance data and tested project examples
  - Updated roadmap to mark Phase 2 (Semantic Indexing) as complete

### Fixed
- MCP tools now return complete data structures instead of just success status
  - All tools properly serialize and return full results
  - Enables proper pagination and data consumption by AI agents

## [0.1.2] - 2025-10-08

### Fixed
- MCP tools now return actual data instead of just success status messages
  - Fixed `goto_definition` to return location data
  - Fixed `find_references` to return reference list
  - Fixed `search_ast`, `list_functions`, `list_classes` to return search results
  - Improved data serialization for MCP protocol

## [0.1.1] - 2025-10-08

### Fixed
- MCP server now correctly advertises tools capability
  - Fixed capability negotiation in MCP protocol
  - Ensures tools are discoverable by MCP clients like Claude Code

## [0.1.0] - 2025-10-08

### Added
- **Initial Release**: Agent Power Tools with SCIP-based semantic code navigation
- **Language Support**: TypeScript, JavaScript, Python, and Rust
  - Auto-installation of language-specific indexers
  - Multi-language project support (indexes all detected languages)
- **Semantic Navigation**:
  - `goto_definition` - Find where symbols are defined
  - `find_references` - Find all symbol usages
  - Cross-file navigation with SCIP indexes
- **AST Pattern Matching** (Tree-sitter based):
  - `search_ast` - Search for code patterns using tree-sitter queries
  - `list_functions` - List all functions with signatures
  - `list_classes` - Find classes, structs, interfaces
  - `project_stats` - Get codebase statistics
- **MCP Server Integration**:
  - Full Model Context Protocol support for Claude Code
  - All tools available as first-class MCP tools
  - JSON-RPC 2.0 communication via stdio
- **CLI Interface**: Complete command-line tool with JSON output
- **Homebrew Distribution**: Installation via custom tap

### Fixed
- Added `allow(dead_code)` to unused `ScipQuery::new` methods (backward compatibility)

---

## Release Process

When creating a new release:

1. **Update CHANGELOG.md**:
   - Add a new version section: `## [X.Y.Z] - YYYY-MM-DD`
   - Document changes under categories: Added, Changed, Deprecated, Removed, Fixed, Security
   - Update the version comparison links at the bottom

2. **Update version in Cargo.toml**:
   ```bash
   # Edit powertools-cli/Cargo.toml
   version = "X.Y.Z"
   ```

3. **Build and test**:
   ```bash
   cargo build --release
   cargo test
   ```

4. **Use the release script** (recommended):
   ```bash
   # From project root
   ./scripts/release.sh
   ```

   The release script will:
   - Extract the changelog entry for the version
   - Show you the changes for review
   - Commit with the message "chore: Release vX.Y.Z"
   - Create an annotated git tag with the changelog as the message
   - Push the commit and tag to GitHub

5. **GitHub Actions** will automatically:
   - Build binaries for macOS and Linux
   - Create a GitHub release with the changelog
   - Upload release artifacts
   - Update the Homebrew formula (via separate workflow)

## Versioning Strategy

We use [Semantic Versioning](https://semver.org/):

- **MAJOR** version: Incompatible API changes
- **MINOR** version: New functionality in a backward compatible manner
- **PATCH** version: Backward compatible bug fixes

Since we're pre-1.0.0, minor versions may include breaking changes.

[unreleased]: https://github.com/zachswift615/agent-power-tools/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/zachswift615/agent-power-tools/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/zachswift615/agent-power-tools/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.6...v0.2.0
[0.1.6]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/zachswift615/agent-power-tools/releases/tag/v0.1.0
