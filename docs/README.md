# Agent Power Tools Documentation

This directory contains detailed documentation for development, features, and implementation plans.

## 📚 Documentation Index

### Current Development

- **[SEMANTIC_REFACTORING_V0.4.0.md](./SEMANTIC_REFACTORING_V0.4.0.md)** - Active development plan for v0.4.0
  - Implementation timeline and milestones
  - Technical architecture decisions
  - Progress tracking and status updates
  - Risk assessment and mitigation strategies

### Planning Documents (Root)

- **[SEMANTIC_REFACTORING_PLAN.md](../SEMANTIC_REFACTORING_PLAN.md)** - Detailed 4-week implementation plan
  - Week-by-week breakdown
  - Algorithm descriptions for each refactoring
  - Code examples and API designs
  - Success criteria and testing strategy

- **[LIBRARY_ANALYSIS.md](../LIBRARY_ANALYSIS.md)** - Parser library research and decisions
  - Evaluation of SWC, Oxc, Ruff, rust-analyzer, syn
  - Trade-off analysis and recommendations
  - Dependency selection rationale
  - Time savings estimates

- **[WISHLIST.md](../WISHLIST.md)** - Feature roadmap and future ideas
  - High-impact tools for AI agents
  - Implementation complexity estimates
  - Community feedback and priorities
  - Long-term vision (v0.5.0+)

- **[CHANGELOG.md](../CHANGELOG.md)** - Version history and release notes
  - What changed in each version
  - Breaking changes and migrations
  - Release process documentation

### User Documentation

- **[README.md](../README.md)** - Main project README
  - Quick start guide
  - Feature overview
  - Installation instructions
  - Usage examples

- **[.claude/CLAUDE.md](../.claude/CLAUDE.md)** - AI agent instructions
  - How Claude Code should use powertools
  - When to use which tools
  - MCP integration guide
  - Best practices for AI workflows

## 🗺️ Documentation Roadmap

### Planned Documentation

- [ ] **API_REFERENCE.md** - Complete API documentation for all MCP tools
- [ ] **CLI_GUIDE.md** - Comprehensive CLI usage guide
- [ ] **ARCHITECTURE.md** - System architecture and design decisions
- [ ] **TESTING_GUIDE.md** - How to write and run tests
- [ ] **PERFORMANCE.md** - Performance benchmarks and optimization tips

## 📋 Version History

### v0.3.0 - Batch File Operations (Released 2025-10-09)
- Batch replace with regex patterns
- Preview-first safety model
- Refactoring infrastructure foundation

### v0.2.0 - File Watching (Released 2025-10-09)
- Automatic re-indexing on file changes
- Language-specific index updates
- MCP watcher control tools

### v0.1.x - SCIP Semantic Navigation (Released 2025-10-08)
- Multi-language SCIP indexing
- goto_definition and find_references
- Tree-sitter pattern matching
- C++ support

## 🤝 Contributing

When adding new documentation:

1. **Implementation Plans:** Place in `docs/` with version number (e.g., `FEATURE_V0.X.0.md`)
2. **User Guides:** Place in `docs/` with descriptive name (e.g., `CLI_GUIDE.md`)
3. **Planning Documents:** Place in root for high visibility (e.g., `WISHLIST.md`)
4. **Update this README:** Add your document to the index above

## 📞 Questions?

- **Development questions:** See active implementation docs
- **Usage questions:** See user documentation
- **Feature requests:** Add to [WISHLIST.md](../WISHLIST.md)
- **Bug reports:** Open a GitHub issue

---

**Last Updated:** 2025-10-09
