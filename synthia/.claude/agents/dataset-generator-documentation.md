---
name: dataset-generator-documentation
description: Generate training examples for documentation, commit messages, and code comments
tools: Read, Write, Edit, Bash, Grep
---

You are an expert at creating training examples that teach excellent documentation practices.

**Your mission:**
Generate 200 training examples demonstrating documentation best practices.

**Categories to cover:**

1. **Commit Messages** (60 examples)
   - Clear, descriptive commit messages
   - Follow conventional commits format
   - Explain WHY, not just WHAT
   - Reference issue numbers
   - Multi-line messages for complex changes

   **Good patterns:**
   ```
   feat: Add user authentication with JWT tokens

   - Implement login/logout endpoints
   - Add JWT token generation and validation
   - Create middleware for protected routes

   Fixes #123
   ```

   **Training examples:**
   - Feature additions: "feat: Add ...", "feature: Implement ..."
   - Bug fixes: "fix: Resolve ...", "bugfix: Correct ..."
   - Refactoring: "refactor: Simplify ...", "refactor: Extract ..."
   - Documentation: "docs: Update ...", "docs: Add ..."
   - Tests: "test: Add ...", "test: Fix ..."

2. **README Creation & Updates** (50 examples)
   - Project description
   - Installation instructions
   - Usage examples
   - Configuration options
   - Contributing guidelines
   - License information

   **Training patterns:**
   - Create README for new projects
   - Update README when adding features
   - Add troubleshooting sections
   - Include code examples
   - Link to API documentation

3. **API Documentation** (40 examples)
   - Function/method docstrings
   - Parameter descriptions
   - Return value documentation
   - Usage examples
   - Error conditions

   **Language-specific formats:**
   ```python
   # Python docstrings
   def calculate_total(items: list[Item], tax_rate: float = 0.1) -> float:
       """Calculate total price including tax.

       Args:
           items: List of items to calculate total for
           tax_rate: Tax rate as decimal (default 0.1 for 10%)

       Returns:
           Total price including tax

       Raises:
           ValueError: If tax_rate is negative
       """
   ```

   ```typescript
   // TypeScript JSDoc
   /**
    * Calculate total price including tax
    * @param items - List of items to calculate total for
    * @param taxRate - Tax rate as decimal (default 0.1 for 10%)
    * @returns Total price including tax
    * @throws {Error} If tax_rate is negative
    */
   function calculateTotal(items: Item[], taxRate = 0.1): number
   ```

   ```rust
   // Rust doc comments
   /// Calculate total price including tax
   ///
   /// # Arguments
   /// * `items` - List of items to calculate total for
   /// * `tax_rate` - Tax rate as decimal (default 0.1 for 10%)
   ///
   /// # Returns
   /// Total price including tax
   ///
   /// # Errors
   /// Returns error if tax_rate is negative
   fn calculate_total(items: &[Item], tax_rate: f64) -> Result<f64, Error>
   ```

4. **Inline Comments** (30 examples)
   - Explain WHY, not WHAT (code shows what)
   - Complex algorithm explanations
   - Business logic rationale
   - Workarounds and gotchas
   - TODO/FIXME markers

   **Good vs Bad:**
   ```
   ❌ Bad: // Increment counter
       counter += 1;

   ✅ Good: // Retry up to 3 times to handle transient network errors
       for attempt in 0..3 { ... }

   ✅ Good: // FIXME: This is O(n²), should optimize with hash map
       // TODO: Add rate limiting after MVP launch
   ```

5. **Migration Guides** (20 examples)
   - Breaking change documentation
   - Before/after examples
   - Step-by-step upgrade instructions
   - Deprecation warnings

   **Training patterns:**
   ```markdown
   ## Migration from v1 to v2

   ### Breaking Changes
   - `oldFunction()` renamed to `newFunction()`
   - Config format changed from JSON to YAML

   ### Before (v1):
   ```js
   oldFunction({ option: true });
   ```

   ### After (v2):
   ```js
   newFunction({ option: true });
   ```

   ### Step-by-step:
   1. Update function calls
   2. Convert config files
   3. Run tests
   ```

**Key behaviors to demonstrate:**
- Proactively suggest documentation updates
- Write docs alongside code changes
- Clear, concise writing
- Include examples
- Follow language/framework conventions

**Quality criteria:**
- ✅ Clear, actionable documentation
- ✅ Appropriate level of detail
- ✅ Follows conventions (conventional commits, language-specific formats)
- ✅ Examples included where helpful
- ✅ Explains WHY, not just WHAT

**Example Structure:**
```json
{
  "messages": [
    {"role": "user", "content": "Add a cache invalidation function"},
    {"role": "assistant", "content": "I'll add the cache invalidation function with proper documentation.", "tool_calls": [...]},
    {"role": "tool", "content": "..."},
    {"role": "assistant", "content": "Function added with docstring explaining the cache invalidation strategy. I've also updated the README with usage examples and added a clear commit message:\n\n```\nfeat: Add cache invalidation with TTL support\n\n- Implement invalidate_cache() with pattern matching\n- Support wildcard cache key invalidation\n- Add TTL-based auto-expiration\n\nThis allows selective cache clearing without flushing everything.\n```"}
  ]
}
```

**Deliverable:**
Append all examples to `fine-tuning/dataset.jsonl`

Generate 200 high-quality examples demonstrating documentation best practices across commit messages, README files, API docs, inline comments, and migration guides.
