# Release Process

This document describes the complete release process for Agent Power Tools.

## Overview

We use semantic versioning and maintain a detailed changelog. Each release involves:
1. Updating the changelog
2. Bumping the version
3. Creating git tags
4. Automated CI/CD builds
5. Homebrew formula updates (if needed)

## Step-by-Step Release Process

### 1. Prepare the Changelog

Edit `CHANGELOG.md` to move items from `[Unreleased]` to a new version section:

```markdown
## [Unreleased]

## [X.Y.Z] - YYYY-MM-DD

### Added
- New features go here

### Changed
- Changes to existing functionality

### Fixed
- Bug fixes

### Removed
- Removed features

### Deprecated
- Soon-to-be removed features

### Security
- Security fixes
```

**Guidelines:**
- Use present tense ("Add feature" not "Added feature")
- Reference issue/PR numbers when applicable
- Group related changes together
- Focus on user-facing changes, not internal refactoring (unless notable)
- Add comparison link at the bottom:
  ```markdown
  [X.Y.Z]: https://github.com/zachswift615/agent-power-tools/compare/vX.Y.Z-1...vX.Y.Z
  ```

### 2. Update Version Number

Edit `powertools-cli/Cargo.toml`:

```toml
[package]
name = "powertools"
version = "X.Y.Z"  # Update this line
```

### 3. Build and Test

```bash
# Build release binary
cd powertools-cli
cargo build --release

# Run tests
cargo test

# Verify the binary works
./target/release/powertools --version
```

### 4. Commit Changes

```bash
# Stage changelog and version bump
git add CHANGELOG.md powertools-cli/Cargo.toml

# Commit with conventional commit format
git commit -m "chore: Release vX.Y.Z

- Updated CHANGELOG.md
- Bumped version to X.Y.Z
"

# Push to main
git push origin main
```

### 5. Create and Push Tag

```bash
# Create annotated tag
git tag -a vX.Y.Z -m "Release vX.Y.Z

Brief description of major changes in this release.
"

# Push tag to trigger CI/CD
git push origin vX.Y.Z
```

**Alternative:** Use the release script (if available):
```bash
./scripts/release.sh X.Y.Z
```

### 6. GitHub Release (Automated)

GitHub Actions will automatically:
- Detect the new tag
- Build binaries for:
  - macOS (Apple Silicon)
  - macOS (Intel)
  - Linux (x86_64)
- Create a GitHub Release
- Upload binaries as release assets

**Monitor:** Check https://github.com/zachswift615/agent-power-tools/actions

### 7. Update Homebrew Formula (If Needed)

For new releases, update the Homebrew tap:

```bash
# Clone the tap repository
git clone https://github.com/zachswift615/homebrew-powertools
cd homebrew-powertools

# Download the new tarball
curl -L -o powertools-X.Y.Z.tar.gz \
  https://github.com/zachswift615/agent-power-tools/archive/vX.Y.Z.tar.gz

# Calculate SHA256
SHA256=$(sha256sum powertools-X.Y.Z.tar.gz | awk '{print $1}')

# Update Formula/powertools.rb
# - Change version = "X.Y.Z"
# - Update sha256 with new value
# - Test locally: brew install --build-from-source ./Formula/powertools.rb

# Commit and push
git add Formula/powertools.rb
git commit -m "powertools X.Y.Z"
git push
```

**Note:** If using GitHub releases as the source, update the formula to point to the release tarball.

### 8. Announce Release

After successful release:
- Update project README if needed
- Announce in relevant channels
- Close related issues/PRs
- Update project documentation

## Version Number Guidelines

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR.MINOR.PATCH** (e.g., 1.2.3)

### When to Bump:

**MAJOR (X.0.0):**
- Breaking API changes
- Removed features
- Incompatible changes requiring user action

**MINOR (0.X.0):**
- New features (backward compatible)
- New language support
- Significant enhancements
- Deprecations (with backward compatibility)

**PATCH (0.0.X):**
- Bug fixes
- Performance improvements
- Documentation updates
- Internal refactoring (no user-facing changes)

### Pre-1.0.0 Versioning:

While we're in 0.x.x releases:
- MINOR versions may include breaking changes
- PATCH versions should be backward compatible
- Document breaking changes clearly in CHANGELOG

## Changelog Categories

### Added
New features, functionality, or support for new languages/platforms.

**Examples:**
- "C++ language support with scip-clang indexing"
- "Pagination support for MCP tools"
- "New `--format` flag for JSON output"

### Changed
Changes to existing functionality, behavior, or defaults.

**Examples:**
- "Updated `check_indexer_installed()` to check `~/.local/bin`"
- "Improved error messages for missing dependencies"
- "Changed default output format to JSON"

### Deprecated
Features that will be removed in future releases (but still work).

**Examples:**
- "Deprecated legacy `.scip` index format (use language-specific indexes)"
- "Old CLI flag `--path` deprecated in favor of `-p`"

### Removed
Features or functionality that have been removed.

**Examples:**
- "Removed support for Node.js 14"
- "Removed legacy command aliases"

### Fixed
Bug fixes, error handling improvements, and corrections.

**Examples:**
- "Fixed MCP tools returning empty data structures"
- "Fixed PATH handling for locally installed indexers"
- "Fixed panic when indexing empty directories"

### Security
Security-related fixes or improvements.

**Examples:**
- "Fixed arbitrary code execution in custom indexer paths"
- "Updated dependencies with security vulnerabilities"

## Troubleshooting

### CI/CD Build Fails

1. Check GitHub Actions logs
2. Test build locally: `cargo build --release`
3. Verify cross-compilation works
4. Check for platform-specific issues

### Homebrew Formula Issues

1. Test locally: `brew install --build-from-source ./Formula/powertools.rb`
2. Check SHA256 matches tarball
3. Verify download URL is accessible
4. Test on clean system (GitHub Actions)

### Version Conflicts

If you need to re-release a version:
1. Delete the tag: `git tag -d vX.Y.Z && git push origin :vX.Y.Z`
2. Delete the GitHub release
3. Fix issues
4. Re-tag and push

## Quick Reference

```bash
# Full release workflow
VERSION="0.1.5"

# 1. Update CHANGELOG.md manually

# 2. Update Cargo.toml version
sed -i '' "s/version = \".*\"/version = \"$VERSION\"/" powertools-cli/Cargo.toml

# 3. Build and test
cargo build --release && cargo test

# 4. Commit
git add CHANGELOG.md powertools-cli/Cargo.toml
git commit -m "chore: Release v$VERSION"
git push

# 5. Tag and push
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"

# 6. Wait for CI/CD (check GitHub Actions)

# 7. Update Homebrew formula (if needed)
```

## Resources

- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
