# Release Scripts

Scripts for managing releases of the agent-powertools project.

## release.sh

Automates the release process including version bumping, tagging, and pushing.

### Prerequisites

- All changes must be committed before running the script
- You must be on the `main` branch
- You must have push access to the repository

### Usage

**Interactive mode** (prompts for version bump type):
```bash
./scripts/release.sh
```

You'll be prompted to choose:
- `1` - Major version bump (breaking changes) - e.g., 1.0.0 → 2.0.0
- `2` - Minor version bump (new features) - e.g., 1.0.0 → 1.1.0
- `3` - Patch version bump (bug fixes) - e.g., 1.0.0 → 1.0.1

**Explicit version** (provide version as parameter):
```bash
./scripts/release.sh 1.2.3
```

### What the script does

1. **Reads current version** from `powertools-cli/Cargo.toml`
2. **Calculates new version** based on your input (major/minor/patch bump or explicit version)
3. **Updates Cargo.toml** with the new version
4. **Commits the version bump** with message: `chore: Bump version to vX.Y.Z`
5. **Pushes to main** branch
6. **Checks for existing tags** (both local and remote)
   - If tag exists, prompts to delete and recreate
7. **Creates and pushes the git tag** (e.g., `v1.2.3`)

### GitHub Actions automation

Once the tag is pushed, GitHub Actions automatically:
- Builds binaries for macOS (ARM64 & x86_64) and Linux (x86_64)
- Creates a GitHub release with the binaries
- Generates SHA256 checksums for each binary
- Updates the Homebrew formula in the `homebrew-powertools` repository

### Example session

```bash
$ ./scripts/release.sh
Current version: 0.1.0

Select version bump:
  1) Major (breaking changes)
  2) Minor (new features)
  3) Patch (bug fixes)
Choice [1-3]: 3
New version: 0.1.1

Continue with version 0.1.1? [y/N]: y
Updating version in Cargo.toml...
Committing version bump...
[main abc1234] chore: Bump version to v0.1.1
 1 file changed, 1 insertion(+), 1 deletion(-)
Pushing to main...
Creating tag v0.1.1...
Pushing tag v0.1.1...

✅ Release v0.1.1 initiated successfully!

GitHub Actions will now:
  1. Build binaries for macOS (ARM64 & x86_64) and Linux
  2. Create GitHub release with binaries and checksums
  3. Update Homebrew formula automatically

Monitor progress at:
  https://github.com/zachswift615/agent-power-tools/actions

Release will be available at:
  https://github.com/zachswift615/agent-power-tools/releases/tag/v0.1.1
```

### Error handling

The script will exit with an error if:
- Invalid version format is provided (must be X.Y.Z)
- Invalid bump type is selected
- User declines to continue at confirmation prompts
- Tag exists and user declines to delete/recreate

### Manual tag deletion

If you need to manually delete a tag:

```bash
# Delete local tag
git tag -d v1.2.3

# Delete remote tag
git push origin :refs/tags/v1.2.3

# Or both at once
git tag -d v1.2.3 && git push origin :refs/tags/v1.2.3
```
