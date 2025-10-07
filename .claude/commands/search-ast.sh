#!/bin/bash
# Search for patterns in AST using tree-sitter queries

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROJECT_ROOT/powertools-cli/target/release/powertools"

if [ ! -f "$BINARY" ]; then
    echo "Error: powertools binary not found. Run: cd powertools-cli && cargo build --release" >&2
    exit 1
fi

# Execute the search with JSON output
"$BINARY" search-ast "$@" --format json