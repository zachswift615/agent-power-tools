# UI Issues - Discovered During P1 Testing

## Issue #1: Tool Output Alignment Problem

**Severity:** Minor (Cosmetic)
**Discovered:** 2025-10-18 (P1 testing)

### Description
Tool output (Command, Output labels) appears offset/misaligned from the left margin, creating excessive indentation.

### Visual Example
**Current (broken):**
```
[Tool: bash] ✓ 426ms
                Command: python app.py

Output: stdout:
                * Serving Flask app
```

**Expected:**
```
[Tool: bash] ✓ 426ms
Command: python app.py

Output: stdout:
  * Serving Flask app
```

### Root Cause Hypothesis
The tool output formatting in `ui/app.rs` `UIUpdate::ToolResult` handler is adding extra indentation.

### Files to Investigate
- `synthia/src/ui/app.rs` - `UIUpdate::ToolResult` handler (around line 240-270)

### Fix Approach
1. Review how we print `Command:` and `Output:` labels
2. Remove or reduce indentation
3. Ensure consistent left margin alignment

---

## Issue #2: Input Line Duplication While Typing

**Severity:** High (UX Problem)
**Discovered:** 2025-10-18 (P1 testing)

### Description
While typing a prompt, the input line repeats itself multiple times (~20 times), filling the screen with duplicate green "You:" prompts.

### Visual Example
**Current (broken):**
```
You: Create a Flask TODO app with HTML templates and explain the code. Please create the app in
You: Create a Flask TODO app with HTML templates and explain the code. Please create the app in
You: Create a Flask TODO app with HTML templates and explain the code. Please create the app in
[... repeats ~20 times ...]
this directory: /Users/zachswift/projects/test-project-synthia
```

**Expected:**
```
You: Create a Flask TODO app with HTML templates and explain the code. Please create the app in this directory: /Users/zachswift/projects/test-project-synthia
```

### Root Cause Hypothesis
The `render_input()` method is being called multiple times without clearing the previous render, or terminal cursor positioning is incorrect.

**Possible causes:**
1. `render_input()` called on every character without clearing line
2. Event batching causing duplicate renders
3. Missing `clear_input_line()` call before re-rendering
4. Cursor not returning to correct position

### Files to Investigate
- `synthia/src/ui/app.rs` - `render_input()` method
- `synthia/src/ui/app.rs` - `clear_input_line()` method
- `synthia/src/ui/app.rs` - Event loop in `run()` method

### Fix Approach
1. Ensure `clear_input_line()` is called before **every** `render_input()`
2. Check if we're rendering too frequently (on every key event vs batched)
3. Verify terminal cursor positioning logic:
   ```rust
   // Before rendering:
   clear_input_line()?;

   // Move to beginning of line
   queue!(stdout, MoveTo(0, cursor_y))?;

   // Render once
   queue!(stdout, Print(format!("You: {}", input)))?;
   stdout.flush()?;
   ```

### Questions for User
1. Does this happen every time you type, or only sometimes?
2. What terminal emulator are you using? (iTerm2, Terminal.app, Alacritty, etc.)
3. Can you reproduce it consistently by typing a new prompt?
4. What's your terminal width? (Run `tput cols`)

---

## Testing Checklist (After Fixes)

### Issue #1 Testing
- [ ] Run Synthia and execute a bash tool
- [ ] Verify `Command:` label is left-aligned (no offset)
- [ ] Verify `Output:` sections are minimally indented
- [ ] Test with long command strings
- [ ] Test with multi-line output

### Issue #2 Testing
- [ ] Start Synthia
- [ ] Type a long prompt (100+ characters)
- [ ] Verify input line appears **only once**
- [ ] Verify no duplicate "You:" prompts
- [ ] Test backspace/delete behavior
- [ ] Test paste behavior (large paste)

---

## Priority & Scheduling

**Not blocking P2 work** - These are UI polish issues that don't affect core functionality:
- ✅ Tools execute correctly
- ✅ Word wrapping works
- ✅ No crashes or data loss

**Recommendation:**
- Document for tracking (this file)
- Continue with P2 architecture improvements
- Schedule UI polish fixes for P3 or dedicated UI sprint

---

## Related Work

### P1 Tasks (Completed)
- ✅ Task 1: Word wrapping fix (no mid-word breaks observed in testing)
- ✅ Task 2: JSON parsing (tools execute successfully)
- ✅ Task 3: Event batching (no hangs)

### Future Tasks
- P3 Task 8: Streaming progress indicator - May touch similar rendering code
- P4 Task 9: Integration tests - Should include UI rendering tests

---

## Notes from Screenshots

**Screenshot #1 (Tool Output Alignment):**
- Shows Flask app execution
- Tool output clearly offset with excessive whitespace
- Otherwise functional (app runs, output displayed)

**Screenshot #2 (Input Duplication):**
- Shows ~20 duplicate "You:" lines while typing
- Final prompt eventually completes correctly
- Suggests render timing issue, not data corruption
