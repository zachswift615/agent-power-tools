#!/bin/bash
# Setup script for Agent Power Tools

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/powertools-cli/target/release/powertools"

echo "🛠️  Agent Power Tools Setup"
echo "=========================="
echo ""

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "⚠️  Binary not found. Building powertools..."
    cd "$SCRIPT_DIR/powertools-cli"
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "❌ Build failed. Please install Rust first: https://rustup.rs"
        exit 1
    fi
    echo "✅ Build complete!"
fi

# Create convenience alias
echo ""
echo "To use powertools from anywhere, add this to your ~/.zshrc or ~/.bashrc:"
echo ""
echo "  alias powertools='$BINARY'"
echo ""
echo "Or run this command to add it now:"
echo ""
echo "  echo \"alias powertools='$BINARY'\" >> ~/.zshrc"
echo ""

# Test the binary
echo "Testing powertools..."
"$BINARY" --version
echo ""
echo "✅ Power Tools ready to use!"
echo ""
echo "Quick start:"
echo "  $BINARY search-ast '(function_item) @func' -p src/"
echo "  $BINARY stats"
echo "  $BINARY --help"