# Homebrew Formula Automation Setup

This guide explains how to set up automatic Homebrew formula updates when you publish a release.

## Overview

When you push a git tag (e.g., `v0.1.0`), the following happens automatically:

1. **Release workflow** builds binaries for all platforms
2. **Update Homebrew workflow** (this guide):
   - Downloads the SHA256 checksums from the release
   - Updates the Homebrew formula with new version and checksums
   - Commits and pushes to `homebrew-powertools` repo

## Prerequisites

You need a GitHub Personal Access Token (PAT) that can push to the `homebrew-powertools` repository.

## Setup Instructions

### Step 1: Create a Personal Access Token

1. Go to GitHub Settings → Developer settings → Personal access tokens → Tokens (classic)
   - Or visit: https://github.com/settings/tokens

2. Click "Generate new token" → "Generate new token (classic)"

3. Configure the token:
   - **Note**: `Homebrew Formula Updater for agent-power-tools`
   - **Expiration**: 90 days (or No expiration if you prefer)
   - **Scopes**: Select these:
     - ✅ `repo` (Full control of private repositories)
       - This gives access to push to `homebrew-powertools`

4. Click "Generate token"

5. **Copy the token immediately** (you won't see it again!)

### Step 2: Add Token to agent-power-tools Repository

1. Go to your `agent-power-tools` repository on GitHub

2. Click Settings → Secrets and variables → Actions

3. Click "New repository secret"

4. Create the secret:
   - **Name**: `HOMEBREW_TAP_TOKEN`
   - **Secret**: Paste the token you copied
   - Click "Add secret"

### Step 3: Test the Workflow

Now when you create a release, both workflows will run:

```bash
cd /path/to/agent-power-tools

# Create and push tag
git tag v0.1.0
git push origin v0.1.0
```

You can watch the workflows run at:
- Release workflow: https://github.com/zachswift615/agent-power-tools/actions/workflows/release.yml
- Homebrew update: https://github.com/zachswift615/agent-power-tools/actions/workflows/update-homebrew.yml

### Step 4: Verify the Update

After the workflows complete (~5-10 minutes):

1. Check the `homebrew-powertools` repo for the new commit
2. Verify the formula has real SHA256s (no `REPLACE_WITH` placeholders)
3. Test installation:

```bash
brew upgrade zachswift615/powertools/powertools
# or
brew uninstall powertools
brew install zachswift615/powertools/powertools
```

## Troubleshooting

### Token Permission Issues

If you see errors like `remote: Permission to zachswift615/homebrew-powertools.git denied`:

- Verify the token has `repo` scope
- Verify the token isn't expired
- Recreate the token and update the secret

### Workflow Doesn't Trigger

If `update-homebrew.yml` doesn't run:

- Verify the workflow file is on the `main` branch
- Check that the release was "published" (not just created)
- Look for errors in the Actions tab

### SHA256 Mismatches

If checksums don't match after installation:

- Verify the release assets were fully uploaded
- Check that GitHub Actions completed successfully
- Manually verify checksums match:

```bash
curl -sL https://github.com/zachswift615/agent-power-tools/releases/download/v0.1.0/powertools-macos-arm64.tar.gz | shasum -a 256
```

## Manual Override

If automation fails, you can manually update the formula:

```bash
cd ~/homebrew-powertools

# Get checksums from release page
curl -sL https://github.com/zachswift615/agent-power-tools/releases/download/v0.1.0/powertools-macos-arm64.tar.gz.sha256

# Edit Formula/powertools.rb with real values
vim Formula/powertools.rb

# Commit and push
git add Formula/powertools.rb
git commit -m "Update powertools to v0.1.0"
git push
```

## Security Notes

- The `HOMEBREW_TAP_TOKEN` gives write access to `homebrew-powertools` only
- It does NOT have access to other repositories
- Consider setting an expiration date and renewing periodically
- Never commit tokens to git or share them publicly
