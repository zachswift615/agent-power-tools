#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' powertools-cli/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

echo -e "${BLUE}Current version: ${CURRENT_VERSION}${NC}"

# Function to parse version components
parse_version() {
    local version=$1
    echo "$version" | sed 's/\([0-9]*\)\.\([0-9]*\)\.\([0-9]*\).*/\1 \2 \3/'
}

# Function to increment version
increment_version() {
    local version=$1
    local bump_type=$2

    read -r major minor patch <<< $(parse_version "$version")

    case $bump_type in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
        *)
            echo "Invalid bump type: $bump_type"
            exit 1
            ;;
    esac

    echo "${major}.${minor}.${patch}"
}

# Determine new version
if [ -z "$1" ]; then
    # No version provided, prompt for bump type
    echo ""
    echo "Select version bump:"
    echo "  1) Major (breaking changes)"
    echo "  2) Minor (new features)"
    echo "  3) Patch (bug fixes)"
    echo -n "Choice [1-3]: "
    read -r choice

    case $choice in
        1)
            NEW_VERSION=$(increment_version "$CURRENT_VERSION" "major")
            ;;
        2)
            NEW_VERSION=$(increment_version "$CURRENT_VERSION" "minor")
            ;;
        3)
            NEW_VERSION=$(increment_version "$CURRENT_VERSION" "patch")
            ;;
        *)
            echo -e "${RED}Invalid choice${NC}"
            exit 1
            ;;
    esac
else
    # Version provided as parameter
    NEW_VERSION=$1
    # Validate version format
    if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo -e "${RED}Invalid version format: $NEW_VERSION${NC}"
        echo "Expected format: X.Y.Z (e.g., 1.2.3)"
        exit 1
    fi
fi

echo -e "${GREEN}New version: ${NEW_VERSION}${NC}"
echo ""

# Confirm version bump
echo -n "Continue with version ${NEW_VERSION}? [y/N]: "
read -r confirm
if [[ ! $confirm =~ ^[Yy]$ ]]; then
    echo "Release cancelled"
    exit 0
fi

# Update version in Cargo.toml
echo -e "${BLUE}Updating version in Cargo.toml...${NC}"
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" powertools-cli/Cargo.toml
rm powertools-cli/Cargo.toml.bak

# Update CHANGELOG.md with new version
echo -e "${BLUE}Updating CHANGELOG.md...${NC}"

# Check if CHANGELOG.md exists
if [ ! -f "CHANGELOG.md" ]; then
    echo -e "${RED}CHANGELOG.md not found${NC}"
    exit 1
fi

# Extract the current release section from CHANGELOG.md
# This gets everything between the first ## [X.Y.Z] header and the next ## header
CHANGELOG_ENTRY=$(awk -v version="$NEW_VERSION" '
    /^## \['"$NEW_VERSION"'\]/ { found=1; print; next }
    found && /^## \[/ { exit }
    found { print }
' CHANGELOG.md)

if [ -z "$CHANGELOG_ENTRY" ]; then
    echo -e "${YELLOW}Warning: No changelog entry found for version ${NEW_VERSION}${NC}"
    echo -e "${YELLOW}Please update CHANGELOG.md manually before releasing${NC}"
    echo -n "Continue anyway? [y/N]: "
    read -r changelog_confirm
    if [[ ! $changelog_confirm =~ ^[Yy]$ ]]; then
        echo "Release cancelled"
        exit 0
    fi
else
    echo -e "${GREEN}Found changelog entry for v${NEW_VERSION}:${NC}"
    echo "----------------------------------------"
    echo "$CHANGELOG_ENTRY"
    echo "----------------------------------------"
fi

# Check if there are uncommitted changes
if ! git diff --quiet powertools-cli/Cargo.toml CHANGELOG.md; then
    echo -e "${YELLOW}Committing version bump...${NC}"
    git add powertools-cli/Cargo.toml CHANGELOG.md
    git commit -m "chore: Bump version to v${NEW_VERSION}"
else
    echo -e "${YELLOW}No version changes to commit${NC}"
fi

# Push to main
echo -e "${BLUE}Pushing to main...${NC}"
git push origin main

# Check if tag already exists
TAG_NAME="v${NEW_VERSION}"
if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
    echo -e "${YELLOW}Tag ${TAG_NAME} already exists locally${NC}"
    echo -n "Delete and recreate? [y/N]: "
    read -r delete_confirm
    if [[ $delete_confirm =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Deleting local tag...${NC}"
        git tag -d "$TAG_NAME"
    else
        echo -e "${RED}Release cancelled${NC}"
        exit 1
    fi
fi

# Check if tag exists remotely
if git ls-remote --tags origin | grep -q "refs/tags/${TAG_NAME}$"; then
    echo -e "${YELLOW}Tag ${TAG_NAME} already exists remotely${NC}"
    echo -n "Delete remote tag and recreate? [y/N]: "
    read -r delete_remote_confirm
    if [[ $delete_remote_confirm =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Deleting remote tag...${NC}"
        git push origin ":refs/tags/${TAG_NAME}"
    else
        echo -e "${RED}Release cancelled${NC}"
        exit 1
    fi
fi

# Create and push tag with changelog as message
echo -e "${BLUE}Creating tag ${TAG_NAME}...${NC}"

if [ -n "$CHANGELOG_ENTRY" ]; then
    # Create annotated tag with changelog entry as message
    # Remove the version header line from the changelog entry for the tag message
    TAG_MESSAGE=$(echo "$CHANGELOG_ENTRY" | tail -n +2 | sed '/^$/d')

    if [ -n "$TAG_MESSAGE" ]; then
        echo -e "${GREEN}Using changelog entry as tag message${NC}"
        git tag -a "$TAG_NAME" -m "Release $NEW_VERSION

$TAG_MESSAGE"
    else
        # Fallback to simple tag if changelog entry is empty
        git tag "$TAG_NAME"
    fi
else
    # No changelog entry, create simple tag
    git tag "$TAG_NAME"
fi

echo -e "${BLUE}Pushing tag ${TAG_NAME}...${NC}"
git push origin "$TAG_NAME"

echo ""
echo -e "${GREEN}✅ Release ${TAG_NAME} initiated successfully!${NC}"
echo ""
echo "GitHub Actions will now:"
echo "  1. Build binaries for macOS (ARM64 & x86_64) and Linux"
echo "  2. Create GitHub release with binaries and checksums"
echo "  3. Update Homebrew formula automatically"
echo ""
echo "Monitor progress at:"
echo "  https://github.com/zachswift615/agent-power-tools/actions"
echo ""
echo "Release will be available at:"
echo "  https://github.com/zachswift615/agent-power-tools/releases/tag/${TAG_NAME}"
