# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
   - Move items from `[Unreleased]` to a new version section
   - Add the version number and date: `## [X.Y.Z] - YYYY-MM-DD`
   - Organize changes under categories: Added, Changed, Deprecated, Removed, Fixed, Security

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

4. **Commit and tag**:
   ```bash
   git add CHANGELOG.md powertools-cli/Cargo.toml
   git commit -m "chore: Release vX.Y.Z"
   git push
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. **GitHub Actions** will automatically:
   - Build binaries for macOS and Linux
   - Create a GitHub release
   - Upload release artifacts

6. **Update Homebrew formula** (if applicable):
   - Update version and SHA256 in the tap repository
   - Test installation: `brew upgrade powertools`

## Versioning Strategy

We use [Semantic Versioning](https://semver.org/):

- **MAJOR** version: Incompatible API changes
- **MINOR** version: New functionality in a backward compatible manner
- **PATCH** version: Backward compatible bug fixes

Since we're pre-1.0.0, minor versions may include breaking changes.

[unreleased]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.6...HEAD
[0.1.6]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/zachswift615/agent-power-tools/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/zachswift615/agent-power-tools/releases/tag/v0.1.0
