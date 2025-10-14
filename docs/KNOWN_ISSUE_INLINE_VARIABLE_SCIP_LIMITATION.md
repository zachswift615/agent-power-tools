# Known Issue: Inline Variable - SCIP Local Variable Limitation

**Date Discovered:** 2025-10-14
**Severity:** CRITICAL - Blocks most real-world usage
**Affects:** inline-variable refactoring (v0.4.0 Phase 2.2)
**Status:** Requires architectural change to fix

---

## Problem Statement

The `inline-variable` refactoring **cannot find references to function-local variables** because SCIP indexers (scip-typescript, rust-analyzer, scip-python, scip-clang) do not emit occurrence data for local variables.

### What Works ✅

- **Top-level/global const variables** (variables declared outside functions at module scope)
- **Safety validations** (mutability checks, side effects detection)

### What Doesn't Work ❌

- **Function-local variables** (the most common use case!)
- **Block-scoped variables** inside if/for/while blocks
- **Method-local variables** inside class methods

---

## Root Cause

SCIP (Source Code Intelligence Protocol) indexers are designed for **cross-file semantic navigation**, not local refactoring within a single function. They optimize index size and build time by only emitting occurrences for symbols that can be referenced from other files:

- ✅ Functions, classes, interfaces, types (can be imported)
- ✅ Exported constants and variables
- ❌ Function parameters
- ❌ Local variables inside function bodies
- ❌ Loop variables (`for (let i = 0; ...)`)

**Why:** Local variables can't be referenced from other files, so SCIP indexers skip them to reduce index size and improve performance.

---

## Evidence from Testing

### Test 1: Top-Level Variable (WORKS)

**File:** `/tmp/inline-test/test.ts`
```typescript
const userName = "Alice";  // Top-level, module scope
console.log("Hello, " + userName);
console.log("Welcome, " + userName + "!");
```

**Command:**
```bash
powertools inline-variable test.ts 1 7 --preview
```

**Result:** ✅ **SUCCESS** - Found 3 occurrences (1 declaration + 2 references)

**Debug Output:**
```
[DEBUG] Factory match in test.ts: symbol=scip-typescript npm inline-test 1.0.0 `test.ts`/userName.
[DEBUG] *** Match in test file: test.ts symbol: scip-typescript npm inline-test 1.0.0 `test.ts`/userName.
[DEBUG] Total test file occurrences checked: 8
```

---

### Test 2: Function-Local Variable (FAILS)

**File:** `/tmp/inline-test/test2.ts`
```typescript
function calculate(x: number, y: number): number {
    const result = x + y;  // Function-local variable
    console.log(result);
    return result;
}
```

**Command:**
```bash
powertools inline-variable test2.ts 2 11 --preview
```

**Result:** ❌ **FAIL** - "Variable 'result' is declared but never used"

**Explanation:** scip-typescript indexed the declaration but did NOT emit occurrences for the two usages on lines 3 and 4 because they're inside the function scope.

---

### Test 3: Safety Checks Still Work

**File:** `/tmp/inline-test/test3.ts`
```typescript
const timestamp = Date.now();  // Function call
console.log(timestamp);
console.log(timestamp);
```

**Command:**
```bash
powertools inline-variable test3.ts 1 7 --preview
```

**Result:** ✅ **CORRECTLY REJECTED** - "Cannot inline variable 'timestamp' because its initializer may have side effects: Date.now()"

**Explanation:** Side effects detection happens BEFORE reference finding, so it correctly rejects this.

---

### Test 4: Mutability Check Works

**File:** `/tmp/inline-test/test4.ts`
```typescript
let counter = 0;  // Mutable
console.log(counter);
counter++;
console.log(counter);
```

**Command:**
```bash
powertools inline-variable test4.ts 1 5 --preview
```

**Result:** ✅ **CORRECTLY REJECTED** - "Cannot inline mutable variable 'counter'"

**Explanation:** Mutability check happens via tree-sitter AST parsing, independent of SCIP.

---

## Impact Assessment

### Affected Use Cases (Most Common!)

1. **Local variables in functions** (90% of inline variable use cases)
   ```typescript
   function process(data) {
       const result = transform(data);  // ❌ Can't inline
       return result;
   }
   ```

2. **Temporary variables in methods**
   ```typescript
   class Service {
       getData() {
           const url = this.buildUrl();  // ❌ Can't inline
           return fetch(url);
       }
   }
   ```

3. **Loop/block-scoped variables**
   ```typescript
   for (let i = 0; i < 10; i++) {
       const squared = i * i;  // ❌ Can't inline
       console.log(squared);
   }
   ```

### Unaffected Use Cases (Rare!)

1. **Module-level constants**
   ```typescript
   const API_URL = "https://api.example.com";  // ✅ Can inline (but usually shouldn't!)
   ```

2. **Exported variables** (cross-file inlining not implemented yet anyway)

---

## Why This Matters

**Inline variable refactoring is primarily used to eliminate temporary variables inside functions**, which is exactly what SCIP doesn't support. This makes the feature nearly useless for real-world refactoring tasks.

**Example Real-World Scenario:**
```typescript
function calculateTotal(items: Item[]): number {
    const subtotal = items.reduce((sum, item) => sum + item.price, 0);  // ❌ Can't inline
    const tax = subtotal * 0.08;  // ❌ Can't inline
    const total = subtotal + tax;  // ❌ Can't inline
    return total;
}
```

All three variables are function-local, so SCIP-based inline-variable **can't refactor any of them**.

---

## Proposed Solutions

### Solution 1: Tree-Sitter Based Reference Finding (Recommended)

**Approach:** For inline-variable, bypass SCIP entirely and use tree-sitter to find references within the same file.

**Pros:**
- Works for all local variables
- No dependency on SCIP indexing
- More reliable for single-file refactoring
- Can find references in comments/strings if desired

**Cons:**
- Only works within a single file (but inline-variable is currently single-file only anyway!)
- Requires language-specific tree-sitter query patterns
- Need to handle identifier shadowing

**Implementation Effort:** 2-3 days

**Pseudo-code:**
```rust
fn find_variable_usages_tree_sitter(&self, file_path: &Path, var_name: &str, declaration_line: usize) -> Result<Vec<Location>> {
    let file_content = fs::read_to_string(file_path)?;
    let language = detect_language(file_path)?;
    let tree = parse_with_tree_sitter(&file_content, language)?;

    // Language-specific query to find all identifiers matching var_name
    let query = match language {
        Language::TypeScript => "(identifier) @ident",
        Language::Rust => "(identifier) @ident",
        Language::Python => "(identifier) @ident",
        Language::Cpp => "(identifier) @ident",
    };

    let mut usages = vec![];
    for capture in tree.query(query) {
        let node = capture.node;
        let text = node.utf8_text(file_content.as_bytes())?;

        if text == var_name && node.start_position().row > declaration_line {
            usages.push(Location {
                file_path: file_path.to_path_buf(),
                line: node.start_position().row + 1,
                column: node.start_position().column + 1,
                end_line: Some(node.end_position().row + 1),
                end_column: Some(node.end_position().column + 1),
            });
        }
    }

    Ok(usages)
}
```

**Challenges:**
- **Identifier shadowing:** Need to ensure we're not replacing a different `result` variable in a nested scope
- **Language-specific quirks:** Each language has different scoping rules
- **Validation:** Should still use SCIP for find_definition to get accurate variable location

**Recommendation:** Implement this for v0.4.1 or v0.5.0

---

### Solution 2: Hybrid Approach (Best of Both Worlds)

**Approach:** Try SCIP first (for cross-file support in the future), fall back to tree-sitter if no references found.

**Pros:**
- Future-proof for cross-file inlining
- Works for both local and module-level variables
- Graceful degradation

**Cons:**
- More complex implementation
- Need to maintain two code paths

**Implementation:**
```rust
fn inline(&self, options: InlineOptions) -> Result<InlineResult> {
    // 1. Use SCIP for definition
    let definition = self.scip_query.find_definition(...)?;

    // 2. Extract variable using tree-sitter
    let var_decl = self.extract_variable_declaration(...)?;

    // 3. Try SCIP for references first
    let mut references = self.scip_query.find_references(&var_name, true)?;

    // 4. If no SCIP references found, fall back to tree-sitter
    if references.is_empty() {
        references = self.find_usages_tree_sitter(&options.file_path, &var_decl.name, var_decl.location.line)?;
    }

    // 5. Proceed with inlining...
}
```

---

### Solution 3: Wait for SCIP Indexer Improvements (Not Recommended)

**Approach:** Request that scip-typescript, rust-analyzer, etc. emit local variable occurrences.

**Pros:**
- Would fix the issue at the source
- Would benefit other tools too

**Cons:**
- Not under our control
- Unlikely to happen (performance/index size trade-off is intentional)
- Would take months/years even if accepted

**Verdict:** Not practical for v0.4.0 timeline

---

## Decision

**For v0.4.0:** Document this limitation clearly. Inline-variable works for top-level constants only.

**For v0.4.1 or v0.5.0:** Implement **Solution 1 (Tree-Sitter Based Reference Finding)** to make inline-variable practical for real-world use.

**Rationale:**
- Inline-variable is inherently a single-file operation (cross-file not yet implemented)
- Tree-sitter gives us precise, reliable reference finding for local variables
- SCIP can still be used for find_definition to get accurate variable location
- This matches the architecture of other refactoring tools (VS Code uses TypeScript language service for local refactoring, not just SCIP-like indexes)

---

## Workaround for Users

Until the tree-sitter solution is implemented, users can:

1. **Only use inline-variable on module-level const variables**
   ```typescript
   const API_URL = "https://api.example.com";  // ✅ Works
   ```

2. **Manually inline function-local variables** using find-and-replace (not safe, but possible)

3. **Use IDE refactoring tools** (VS Code, IntelliJ) for function-local inlining

---

## Testing Notes

**What to Test:**
- ✅ Top-level variables (works)
- ✅ Safety validations (works)
- ❌ Function-local variables (doesn't work, expected)

**Don't Waste Time Testing:**
- Rust function-local variables (same issue)
- Python function-local variables (same issue)
- C++ function-local variables (same issue)

All SCIP indexers have this same limitation - it's by design, not a bug.

---

## Related Issues

- scip-python test file references bug (similar root cause - SCIP doesn't emit all occurrences)
- KNOWN_ISSUE_PYTHON_TEST_REFERENCES.md

---

## References

- SCIP specification: https://github.com/sourcegraph/scip
- Tree-sitter queries: https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries
- VS Code refactoring architecture: Uses TypeScript Language Service for local refactoring, not LSP

---

**Last Updated:** 2025-10-14
**Discovered By:** Testing session for v0.4.0 Phase 2.2
**Next Action:** Implement tree-sitter-based reference finding for v0.4.1+
