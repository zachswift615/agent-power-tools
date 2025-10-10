# Known Issue: Python Test File References Not Found

## Summary
When using `powertools references` or `powertools rename-symbol` on Python projects, **references from test files are not found**. This is due to an upstream bug in scip-python where the TreeVisitor doesn't emit reference occurrences for imported symbols.

## Impact
- **`powertools references <symbol>`**: Returns results only from `src/` files, omits all test file usages
- **`powertools rename-symbol`**: Renames symbol in source files but leaves test files unchanged
- **Scope**: Affects all Python projects indexed with scip-python

## Quick Reproduction

### Using poetry-core as test case:

```bash
# 1. Clone test repository
git clone https://github.com/python-poetry/poetry-core.git
cd poetry-core

# 2. Index with powertools
powertools index --auto-install --languages python

# 3. Try finding references to Factory class
powertools references Factory --format json -p .

# Expected: 500+ references including from tests/conftest.py, tests/test_factory.py
# Actual: 398 references, ALL from src/, ZERO from tests/
```

### Verification
```bash
# Confirm test files ARE indexed
powertools functions --format json | grep -c '"file_path".*test'
# Shows: Many test files indexed (e.g., 1726 functions found, many in tests/)

# Confirm test files use Factory
grep -r "Factory" tests/conftest.py tests/test_factory.py | wc -l
# Shows: 15+ usages in test files

# But references query returns 0 from tests
powertools references Factory --format json -p . | grep -c 'tests/'
# Shows: 0
```

## Root Cause

**This is an upstream scip-python bug, not a powertools bug.**

The issue is in scip-python's TreeVisitor implementation:
- Test files ARE processed and indexed
- Test files DO have occurrences written to SCIP index (e.g., 182 occurrences in conftest.py)
- But occurrences only contain module-level symbols (`tests/__init__:`, `pytest/__init__:`)
- References to imported symbols (like `Factory`) are NOT emitted as occurrences

When test code does:
```python
from poetry.core.factory import Factory

def test_something():
    factory = Factory()  # ← This reference is NOT in SCIP index
    factory.create_poetry()  # ← This reference is NOT in SCIP index
```

The SCIP index should contain occurrences with symbol `scip-python python poetry-core 2.2.1 \`src.poetry.core.factory\`/Factory#`, but it doesn't.

## Detailed Investigation

See the comprehensive bug report in the scip-python repository:
**[scip-python/BUG_REPORT_TEST_FILE_REFERENCES.md](https://github.com/sourcegraph/scip-python/blob/main/BUG_REPORT_TEST_FILE_REFERENCES.md)**
*(Or in your local fork: `/path/to/scip-python/BUG_REPORT_TEST_FILE_REFERENCES.md`)*

That document includes:
- Complete step-by-step reproduction
- Analysis of what's working vs what's broken
- Exact location in scip-python code where the bug likely exists
- Test case for verifying a fix

## Statistics from Investigation

Testing on poetry-core repository:
- **Total files indexed**: 345 (186 in tests/, 142 in src/)
- **Test file occurrences**: 33,934 total
- **Test documents in SCIP**: 203 (confirmed loaded by powertools)
- **Factory references found**: 398 (should be 500+)
- **Factory references from tests**: 0 (should be 100+)

## Workaround

**None currently available.**

When renaming symbols, you must manually update test files:
```bash
# 1. Rename in source files with powertools
powertools rename-symbol src/module.py 10 5 NewName --preview=false

# 2. Manually update test files
grep -r "OldName" tests/ | # ... manual editing required
```

## Status

- **Powertools**: ✅ Working correctly - loads all documents from SCIP index, queries them properly
- **scip-python indexing**: ⚠️ Bug - TreeVisitor emits occurrences with wrong symbols (import module instead of source module)
- **Root cause**: `TypeEvaluator.resolveAliasDeclaration()` returns `null` for test file imports, preventing proper alias resolution
- **Fix required in**: `scip-python` repository, files:
  - `packages/pyright-scip/src/treeVisitor.ts` (symptom)
  - `packages/pyright-internal/*/typeEvaluator.ts` (root cause - Pyright's alias resolution)

## Fix Attempts

A partial fix was attempted in December 2024:
- ✅ Modified `emitDeclaration()` to skip early return for alias declarations
- ❌ Failed because `resolveAliasDeclaration()` returns null for test file imports
- The issue is deeper than TreeVisitor - it's in Pyright's type resolution system

See full details in: [scip-python/BUG_REPORT_TEST_FILE_REFERENCES.md](https://github.com/sourcegraph/scip-python/blob/main/BUG_REPORT_TEST_FILE_REFERENCES.md) (section: "Fix Attempt Results")

## Testing After Fix

Once scip-python is fixed, verify with:

```bash
# Re-index with fixed scip-python
powertools index --auto-install --languages python

# Query should now return test references
powertools references Factory --format json -p . | jq '. | map(select(.location.file_path | contains("test"))) | length'
# Should show: 100+ (not 0)

# Rename should update both src and tests
powertools rename-symbol src/poetry/core/factory.py 51 7 MyFactory --preview
# Should show changes in both src/ and tests/
```

## Related Issues

- TypeScript/JavaScript: Test references work correctly (scip-typescript doesn't have this bug)
- Rust: Test references work correctly (rust-analyzer/scip integration works)
- C++: Test references work correctly (scip-clang works)
- **Only Python affected** due to scip-python's TreeVisitor implementation

## Debug Information

If investigating locally, you can add debug logging to powertools:

```rust
// In powertools-cli/src/indexers/scip_query_simple.rs, find_references()
eprintln!("[DEBUG] Searching {} indexes", self.indexes.len());
for (i, index) in self.indexes.iter().enumerate() {
    let test_docs = index.documents.iter()
        .filter(|d| d.relative_path.contains("test"))
        .count();
    eprintln!("[DEBUG] Index {}: {} test docs", i, test_docs);
}
```

This confirms test documents are loaded (you'll see 200+ test docs), but queries return no matches.

## Contact

For questions about this issue:
- **scip-python bugs**: File at https://github.com/sourcegraph/scip-python/issues
- **Powertools integration**: File at https://github.com/your-org/agent-power-tools/issues
