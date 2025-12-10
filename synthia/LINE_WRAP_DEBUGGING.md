# Line Wrap Duplication Bug - Debugging Handoff

## Problem Description

When typing a message that exceeds the terminal width, the first line's content duplicates on every character typed after the wrap point. This makes long messages completely unreadable.

**Status**: NOT FIXED - Multiple approaches tried, none successful yet.

## What Works

✅ **Pasting** - Fixed! Text can now be pasted without duplication.
✅ **Pastel colors** - Working beautifully
✅ **No compiler warnings** - Clean build

## What Doesn't Work

❌ **Line wrapping** - Still duplicating content when lines exceed terminal width

---

## Approaches Tried (All Failed)

### Attempt 1: Manual Line Wrapping
**Commit**: Initial fix attempt
**File**: `synthia/src/ui/app.rs` lines 843-868 (old version)

**Approach**:
- Manually chunk lines to terminal width
- Insert `\r\n` at wrap boundaries
- Prevent terminal from auto-wrapping

**Result**: FAILED - Still had duplication issues

**Code**:
```rust
// Manually wrap first line
let first_line_budget = (term_width as usize).saturating_sub(prompt_len);
let first_line_chars: Vec<char> = lines[0].chars().collect();

for (i, chunk) in first_line_chars.chunks(first_line_budget).enumerate() {
    if i > 0 {
        queue!(stdout, Print("\r\n"))?;
    }
    let chunk_str: String = chunk.iter().collect();
    queue!(stdout, Print(&chunk_str))?;
}
```

**Why it failed**: Too complex, edge cases with exact terminal width, cursor positioning became a nightmare

---

### Attempt 2: Calculate and Clear All Lines
**Commit**: Second attempt
**File**: `synthia/src/ui/app.rs` lines 822-842 (old version)

**Approach**:
- Calculate how many screen lines input will occupy
- Clear each line individually
- Let terminal handle wrapping naturally

**Result**: FAILED - Still duplicating

**Code**:
```rust
let mut total_screen_lines = 0;
for (idx, line) in lines.iter().enumerate() {
    let line_len = if idx == 0 {
        prompt_len + line.chars().count()
    } else {
        line.chars().count()
    };
    total_screen_lines += if line_len == 0 { 1 } else { (line_len + term_width as usize - 1) / term_width as usize };
}

for _ in 0..total_screen_lines {
    execute!(stdout, Clear(ClearType::CurrentLine), cursor::MoveDown(1))?;
}
```

**Why it failed**: Calculating exact wrapped lines is error-prone, cursor positioning issues

---

### Attempt 3: Clear From Cursor Down
**Commit**: Third attempt (CURRENT STATE)
**File**: `synthia/src/ui/app.rs` lines 825-837

**Approach**:
- Use `Clear(ClearType::FromCursorDown)` instead of calculating lines
- This clears everything below cursor in one operation
- Simpler and more robust

**Result**: FAILED - STILL duplicating

**Current Code**:
```rust
// Get current cursor position and move to start of line
let (_, cursor_y) = cursor::position()?;

// Clear everything from current position to end of screen
execute!(
    stdout,
    cursor::MoveTo(0, cursor_y),
    Clear(ClearType::FromCursorDown),
    SetForegroundColor(PastelColors::SUCCESS),
    Print("You: "),
    ResetColor
)?;

// Now queue the input text
queue!(stdout, Print(lines[0]))?;

// Print remaining lines (if any)
for line in &lines[1..] {
    queue!(stdout, Print("\r\n"), Print(line))?;
}
```

**Why it failed**: Unknown - should theoretically work

---

### Attempt 4: Block Rendering During Paste
**Commit**: Paste fix attempt
**File**: `synthia/src/ui/app.rs` line 411

**Approach**:
- Skip rendering when `is_pasting` flag is set
- Only render once when paste completes

**Result**: BROKE PASTING ENTIRELY - Reverted

**Code** (reverted):
```rust
&& !self.is_pasting  // Skip rendering during paste to avoid duplication
```

**Why it failed**: Paste detection's 100ms timeout wasn't triggering properly, so text never appeared

---

## Observations

1. **Clean build verified** - `md5 target/release/synthia` shows recent build
2. **Code changes confirmed** - `grep "FromCursorDown"` shows current approach in source
3. **Paste works** - This proves rendering CAN work without duplication in some cases
4. **Only line wrap duplicates** - Normal typing works fine until wrap point

## Current Code Location

**File**: `synthia/src/ui/app.rs`
**Function**: `render_input_line()` starting at line 811

Key sections:
- **Lines 825-837**: Clear screen and print prompt
- **Lines 842-850**: Print input lines
- **Lines 852-897**: Calculate cursor position (complex!)
- **Lines 900**: Flush output

## Hypotheses (Untested)

### Hypothesis 1: Cursor Position Calculation Bug
The cursor position calculation (lines 852-897) is extremely complex. It might be:
- Moving cursor to wrong position
- Causing re-render at wrong Y coordinate
- Not accounting for terminal auto-wrap properly

**Evidence**: This code hasn't been simplified or revised

### Hypothesis 2: Terminal Auto-Wrap Interference
When terminal auto-wraps at exactly terminal width, it might:
- Not actually be at column 0 of new line
- Have cursor at position `term_width` instead of `0`
- Need explicit `\r` before newline

**Test**: Add explicit `\r` before wrapped content

### Hypothesis 3: Screen Clear Not Actually Clearing
`Clear(ClearType::FromCursorDown)` might not work as expected because:
- Cursor Y position is wrong
- Terminal hasn't scrolled yet
- Need to clear BEFORE getting cursor position

**Test**: Get cursor position, move up a line, then clear

### Hypothesis 4: Rendering Twice Per Keystroke
The render might be called multiple times:
- Once on keystroke
- Once from event loop
- Guard `is_rendering_input` might not be working

**Test**: Add logging to see how many times render is called

### Hypothesis 5: Terminal-Specific Behavior
Different terminals handle wrapping differently:
- iTerm2 vs Terminal.app vs Alacritty
- Some terminals add implicit newline at column `width`
- Some terminals move cursor differently

**Need Info**:
- What terminal is being used?
- What is `$COLUMNS` value?
- Does it happen in different terminals?

---

## Debugging Steps to Try Next

### Step 1: Add Logging
Add trace logging to understand what's happening:

```rust
tracing::debug!(
    "render_input_line: cursor_y={}, term_width={}, input_len={}, lines={}",
    cursor_y, term_width, self.input.chars().count(), lines.len()
);
```

### Step 2: Test Simple Case
Simplify render to absolute minimum:

```rust
fn render_input_line(&mut self, stdout: &mut impl Write) -> io::Result<()> {
    execute!(
        stdout,
        cursor::MoveTo(0, cursor::position()?.1),
        Clear(ClearType::FromCursorDown),
        Print("You: "),
        Print(&self.input)
    )?;
    stdout.flush()
}
```

If this works, add back complexity piece by piece.

### Step 3: Check for Double-Render
Add counter to see if render is called multiple times per keystroke:

```rust
static RENDER_COUNT: AtomicU64 = AtomicU64::new(0);
let count = RENDER_COUNT.fetch_add(1, Ordering::Relaxed);
tracing::warn!("RENDER #{}", count);
```

### Step 4: Terminal Info
Get terminal-specific info:

```bash
echo $TERM
echo $COLUMNS
tput cols
```

### Step 5: Try Different Terminal
Test in:
- Terminal.app
- iTerm2
- Alacritty
- Kitty

See if behavior differs.

### Step 6: Inspect Terminal State
After duplication occurs, check:
- Actual cursor position: `cursor::position()?`
- Expected vs actual Y coordinate
- Number of lines on screen

---

## Files Modified

- `synthia/src/ui/app.rs` - Main rendering logic
- `synthia/src/ui/colors.rs` - Pastel color palette (working)
- `synthia/src/ui/mod.rs` - Added colors module

## Related Issues

- Context shows 0 tokens - Separate issue, not investigated deeply
- Token tracking in context_manager.rs needs investigation

---

## Questions for User

1. **Terminal**: What terminal app? (iTerm2, Terminal.app, Alacritty, etc.)
2. **Width**: What is your terminal width? (`echo $COLUMNS`)
3. **Pattern**: Describe exactly what duplicates:
   - Does the ENTIRE first line repeat?
   - Or just some characters?
   - Does it happen on second line too?
4. **When**: Does it duplicate:
   - Every keystroke after wrap?
   - Only when cursor is at end?
   - When editing middle of wrapped line?

---

## Recommended Next Steps

1. **Simplify first** - Try the absolute minimal render (Step 2 above)
2. **Add logging** - Understand what's actually happening
3. **Test terminals** - See if it's terminal-specific
4. **Check for double-render** - Verify render isn't called 2x per key
5. **Investigate cursor calc** - That code is super complex and error-prone

---

## Code Smell Alert 🚨

The cursor position calculation (lines 852-897) is **extremely complex**:
- Nested loops
- Multiple conditionals
- Modulo arithmetic
- Manually calculating wrapped lines

This is likely where the bug is hiding. A simpler approach might work better.

**Suggested rewrite**: Let terminal handle everything, just track logical cursor position in string, don't try to calculate screen coordinates.

---

## Build Instructions

```bash
# Clean build
cargo clean -p synthia
cargo build -p synthia --release

# Run
cargo run -p synthia --release

# Check binary
ls -lh target/release/synthia
md5 target/release/synthia
```

---

## Success Criteria

✅ Type a message longer than terminal width
✅ See text wrap to next line cleanly
✅ No duplication of any characters
✅ Cursor positioned correctly
✅ Backspace works across wrap boundary
✅ Can edit in middle of wrapped line

**Current Status**: All criteria FAIL except paste

---

Good luck! This is a tricky one. The fact that paste works suggests the rendering CAN work correctly, so there's something specific about the incremental rendering during typing that's causing issues.
