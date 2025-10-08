# Release Process

This document describes how to create and publish a new release.

## Prerequisites

1. Update `YOUR_USERNAME` in all files:
   - `README.md`
   - `install.sh`
   - `Formula/powertools.rb`
   - `.github/workflows/release.yml`

2. Push code to GitHub

## Creating a Release

### 1. Tag a Version

```bash
# Create and push a tag
git tag v0.1.0
git push origin v0.1.0
```

This will automatically trigger the GitHub Actions workflow to:
- Build binaries for macOS (ARM64 + x86_64) and Linux
- Create a GitHub release
- Upload binaries and checksums

### 2. Update Homebrew Formula

After the release is created:

1. Download the release archives and get their SHA256 checksums:

```bash
# The checksums are automatically uploaded as .sha256 files
# Download them from the release page or use:
curl -sL https://github.com/YOUR_USERNAME/agent-power-tools/releases/download/v0.1.0/powertools-macos-arm64.tar.gz.sha256
curl -sL https://github.com/YOUR_USERNAME/agent-power-tools/releases/download/v0.1.0/powertools-macos-x86_64.tar.gz.sha256
curl -sL https://github.com/YOUR_USERNAME/agent-power-tools/releases/download/v0.1.0/powertools-linux-x86_64.tar.gz.sha256
```

2. Update `Formula/powertools.rb` with the actual SHA256 values

3. Commit and push the formula:

```bash
git add Formula/powertools.rb
git commit -m "Update Homebrew formula for v0.1.0"
git push
```

### 3. Create Homebrew Tap (First Release Only)

For Homebrew installation to work, you need to create a separate "tap" repository:

1. Create a new GitHub repo named `homebrew-powertools`

2. Copy the formula:

```bash
# In the homebrew-powertools repo
mkdir Formula
cp /path/to/agent-power-tools/Formula/powertools.rb Formula/
git add Formula/powertools.rb
git commit -m "Add powertools formula"
git push
```

3. Users can now install with:

```bash
brew tap YOUR_USERNAME/powertools
brew install powertools
```

### 4. Test the Release

Test each installation method:

```bash
# Install script
curl -fsSL https://raw.githubusercontent.com/YOUR_USERNAME/agent-power-tools/main/install.sh | sh

# Homebrew
brew tap YOUR_USERNAME/powertools
brew install powertools

# Direct download
wget https://github.com/YOUR_USERNAME/agent-power-tools/releases/download/v0.1.0/powertools-macos-arm64.tar.gz
tar xzf powertools-macos-arm64.tar.gz
./powertools --version
```

## Version Bumping

For subsequent releases:

1. Update version in:
   - `powertools-cli/Cargo.toml`
   - `Formula/powertools.rb`

2. Create and push new tag:

```bash
git tag v0.2.0
git push origin v0.2.0
```

3. Update Homebrew formula SHA256 values after binaries are built

## Troubleshooting

### GitHub Actions Fails

- Check the Actions tab in GitHub
- Common issues:
  - Missing GITHUB_TOKEN permissions
  - Build dependencies not installed
  - Cross-compilation issues

### Homebrew Install Fails

- Verify SHA256 checksums match
- Test formula locally:

```bash
brew install --build-from-source Formula/powertools.rb
brew audit --strict Formula/powertools.rb
```

### Install Script Fails

- Test locally:

```bash
bash -x install.sh
```

- Check platform detection:

```bash
uname -s  # OS
uname -m  # Architecture
```
