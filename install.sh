#!/bin/bash
set -e

# Powertools installation script
# Usage: curl -fsSL https://raw.githubusercontent.com/YOUR_USERNAME/agent-powertools/main/install.sh | sh

REPO="YOUR_USERNAME/agent-powertools"
BINARY_NAME="powertools"
INSTALL_DIR="${HOME}/.local/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print colored message
info() {
    echo -e "${GREEN}==>${NC} $1"
}

error() {
    echo -e "${RED}Error:${NC} $1" >&2
    exit 1
}

warn() {
    echo -e "${YELLOW}Warning:${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    local os="$(uname -s)"
    local arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                arm64|aarch64)
                    echo "macos-arm64"
                    ;;
                x86_64)
                    echo "macos-x86_64"
                    ;;
                *)
                    error "Unsupported macOS architecture: $arch"
                    ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64)
                    echo "linux-x86_64"
                    ;;
                *)
                    error "Unsupported Linux architecture: $arch"
                    ;;
            esac
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac
}

# Get latest release version
get_latest_version() {
    curl -s "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep '"tag_name":' | \
        sed -E 's/.*"v([^"]+)".*/\1/'
}

# Main installation
main() {
    info "Installing ${BINARY_NAME}..."

    # Detect platform
    local platform=$(detect_platform)
    info "Detected platform: $platform"

    # Get latest version
    info "Fetching latest release..."
    local version=$(get_latest_version)

    if [ -z "$version" ]; then
        error "Failed to get latest version. Please check your internet connection."
    fi

    info "Latest version: v${version}"

    # Construct download URL
    local archive_name="${BINARY_NAME}-${platform}.tar.gz"
    local download_url="https://github.com/${REPO}/releases/download/v${version}/${archive_name}"

    info "Downloading from: $download_url"

    # Create temporary directory
    local tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    # Download binary
    if ! curl -fsSL "$download_url" -o "${tmp_dir}/${archive_name}"; then
        error "Failed to download ${archive_name}"
    fi

    # Extract binary
    info "Extracting..."
    tar -xzf "${tmp_dir}/${archive_name}" -C "$tmp_dir"

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Install binary
    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
    mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    # Check if install dir is in PATH
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        warn "${INSTALL_DIR} is not in your PATH"
        echo ""
        echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "    export PATH=\"\${HOME}/.local/bin:\$PATH\""
        echo ""
        echo "Or move the binary to /usr/local/bin:"
        echo "    sudo mv ${INSTALL_DIR}/${BINARY_NAME} /usr/local/bin/"
    fi

    # Success message
    info "Installation complete! 🎉"
    echo ""
    echo "Run '${BINARY_NAME} --help' to get started"
    echo ""
    echo "For MCP integration with Claude Code, see:"
    echo "https://github.com/${REPO}#mcp-integration"
}

main "$@"
