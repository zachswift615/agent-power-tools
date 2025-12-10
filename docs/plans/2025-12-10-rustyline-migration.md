# Rustyline Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace custom crossterm cursor calculation with Rustyline's LineBuffer to eliminate line-wrap duplication bug while matching Claude Code's UX.

**Architecture:** Create `InputManager` wrapper around Rustyline's `LineBuffer` for cursor/editing logic. Consolidate scattered modal state into clean `AppMode` enum. Delete 90+ lines of buggy cursor math from `render_input_line()`. Keep all modal rendering unchanged.

**Tech Stack:** Rust, Rustyline 17.0.2, Crossterm 0.28, Tokio async runtime

---

## Task 1: Add Rustyline Dependency

**Files:**
- Modify: `synthia/Cargo.toml`

**Step 1: Add rustyline dependency**

Edit `synthia/Cargo.toml`, add to `[dependencies]` section:

```toml
rustyline = "17.0.2"
```

**Step 2: Verify dependency resolves**

Run: `cargo check -p synthia`

Expected: "Finished dev [unoptimized + debuginfo] target(s)"

**Step 3: Commit**

```bash
git add synthia/Cargo.toml Cargo.lock
git commit -m "deps: add rustyline 17.0.2 for input management"
```

**Important Note**: We are using ONLY `rustyline::line_buffer::LineBuffer`, not the full Rustyline `Editor`. `LineBuffer` is a pure data structure for managing text and cursor position - it doesn't handle terminal I/O. We continue using crossterm for async event polling and rendering.

---

## Task 2: Create InputManager Module Skeleton

**Files:**
- Create: `synthia/src/ui/input.rs`
- Modify: `synthia/src/ui/mod.rs`

**Step 1: Create empty input.rs file**

Create `synthia/src/ui/input.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::{cursor, execute, queue, terminal::size, terminal::Clear, terminal::ClearType};
use rustyline::line_buffer::LineBuffer;
use std::io::{self, Write};
use std::time::Instant;

use crate::ui::colors::PastelColors;

const MAX_HISTORY_SIZE: usize = 100;  // Limit input history to prevent memory bloat
const MAX_INPUT_LENGTH: usize = 100_000;  // 100K chars max

pub struct InputManager {
    buffer: LineBuffer,
    history: Vec<String>,
    history_index: Option<usize>,
    prompt: String,
    // Paste detection (10ms threshold - keys < 10ms apart = pasting)
    is_pasting: bool,
    last_key_time: Option<Instant>,
}

#[derive(Debug, PartialEq)]
pub enum InputAction {
    None,
    Redraw,
    Submit(String),
    Cancel,
    Quit,
    OpenMenu,
    SaveSession,
    NewSession,
    ListSessions,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            buffer: LineBuffer::with_capacity(1024),
            history: Vec::new(),
            history_index: None,
            prompt: "You: ".to_string(),
            is_pasting: false,
            last_key_time: None,
        }
    }

    pub fn handle_key(&mut self, _key: KeyEvent) -> InputAction {
        // TODO: Implement
        InputAction::None
    }

    pub fn render(&self, _stdout: &mut impl Write) -> io::Result<()> {
        // TODO: Implement
        Ok(())
    }

    pub fn get_text(&self) -> &str {
        self.buffer.as_str()
    }

    pub fn get_cursor_position(&self) -> usize {
        self.buffer.pos()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_buffer() {
        let mgr = InputManager::new();
        assert_eq!(mgr.get_text(), "");
        assert_eq!(mgr.get_cursor_position(), 0);
    }
}
```

**Step 2: Add module to ui/mod.rs**

Edit `synthia/src/ui/mod.rs`:

```rust
pub mod app;
pub mod colors;
pub mod input;  // NEW
pub mod markdown;

pub use app::App;
pub use input::InputManager;  // NEW
```

**Step 3: Verify it compiles**

Run: `cargo test -p synthia --lib ui::input::tests::test_new_creates_empty_buffer`

Expected: "test ui::input::tests::test_new_creates_empty_buffer ... ok"

**Step 4: Commit**

```bash
git add synthia/src/ui/input.rs synthia/src/ui/mod.rs
git commit -m "feat: add InputManager skeleton with LineBuffer"
```

---

## Task 3: Implement Basic Character Input

**Files:**
- Modify: `synthia/src/ui/input.rs`

**Step 1: Write failing test for character input**

Add to `synthia/src/ui/input.rs` in `#[cfg(test)] mod tests`:

```rust
fn key_char(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

#[test]
fn test_character_input() {
    let mut mgr = InputManager::new();

    let action = mgr.handle_key(key_char('h'));
    assert_eq!(action, InputAction::Redraw);
    assert_eq!(mgr.get_text(), "h");

    mgr.handle_key(key_char('i'));
    assert_eq!(mgr.get_text(), "hi");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p synthia --lib ui::input::tests::test_character_input`

Expected: FAIL with "assertion failed: `(left == right)`"

**Step 3: Implement character input in handle_key()**

Replace `handle_key()` method in `synthia/src/ui/input.rs`:

```rust
pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
    // Update paste detection (keys < 10ms apart = pasting)
    let now = Instant::now();
    if let Some(last_time) = self.last_key_time {
        let elapsed = now.duration_since(last_time);
        if elapsed.as_millis() < 10 {
            self.is_pasting = true;
        } else if elapsed.as_millis() > 100 {
            self.is_pasting = false;
        }
    }
    self.last_key_time = Some(now);

    match (key.code, key.modifiers) {
        // Character input
        (KeyCode::Char(c), _) => {
            // Check max input length
            if self.buffer.as_str().chars().count() >= MAX_INPUT_LENGTH {
                return InputAction::None;  // Silently ignore
            }
            self.buffer.insert(c, 1);
            InputAction::Redraw
        }

        _ => InputAction::None,
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p synthia --lib ui::input::tests::test_character_input`

Expected: "test ui::input::tests::test_character_input ... ok"

**Step 5: Commit**

```bash
git add synthia/src/ui/input.rs
git commit -m "feat: implement basic character input in InputManager"
```

---

## Task 4: Implement Submit and Multi-line Input

**Files:**
- Modify: `synthia/src/ui/input.rs`

**Step 1: Write failing test for submit**

Add to tests in `synthia/src/ui/input.rs`:

```rust
#[test]
fn test_submit_on_enter() {
    let mut mgr = InputManager::new();

    mgr.handle_key(key_char('h'));
    mgr.handle_key(key_char('i'));

    let action = mgr.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(action, InputAction::Submit(ref text) if text == "hi"));
    assert_eq!(mgr.get_text(), ""); // Buffer cleared
}

#[test]
fn test_multiline_on_shift_enter() {
    let mut mgr = InputManager::new();

    mgr.handle_key(key_char('h'));
    mgr.handle_key(key_char('i'));

    // Shift+Enter inserts newline
    let action = mgr.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    assert_eq!(action, InputAction::Redraw);

    mgr.handle_key(key_char('b'));
    mgr.handle_key(key_char('y'));
    mgr.handle_key(key_char('e'));

    assert_eq!(mgr.get_text(), "hi\nbye");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p synthia --lib ui::input::tests`

Expected: FAIL on both new tests

**Step 3: Implement Enter and Shift+Enter**

Update `handle_key()` in `synthia/src/ui/input.rs`, add before the `_` pattern:

```rust
// Submit on Enter (but not during paste or Shift+Enter, and not if empty)
(KeyCode::Enter, modifiers) if !modifiers.contains(KeyModifiers::SHIFT) && !self.is_pasting => {
    let text = self.buffer.as_str().to_string();

    // Don't submit empty input
    if text.trim().is_empty() {
        return InputAction::None;
    }

    // Add to history (with limit)
    self.history.push(text.clone());
    if self.history.len() > MAX_HISTORY_SIZE {
        self.history.remove(0);  // Remove oldest
    }

    self.history_index = None;
    self.buffer.clear();
    self.is_pasting = false;  // Reset paste mode
    InputAction::Submit(text)
}

// Shift+Enter OR pasting: insert newline (don't auto-submit)
(KeyCode::Enter, _) => {
    self.buffer.insert('\n', 1);
    InputAction::Redraw
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p synthia --lib ui::input::tests`

Expected: All tests pass

**Step 5: Commit**

```bash
git add synthia/src/ui/input.rs
git commit -m "feat: add submit (Enter) and multi-line (Shift+Enter) support"
```

---

## Task 5: Implement Cursor Movement

**Files:**
- Modify: `synthia/src/ui/input.rs`

**Step 1: Write failing test for cursor movement**

Add to tests:

```rust
#[test]
fn test_cursor_movement() {
    let mut mgr = InputManager::new();

    mgr.handle_key(key_char('a'));
    mgr.handle_key(key_char('b'));
    mgr.handle_key(key_char('c'));

    assert_eq!(mgr.get_cursor_position(), 3);

    // Move left twice
    mgr.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    mgr.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));

    assert_eq!(mgr.get_cursor_position(), 1);

    // Insert 'X' (should be "aXbc")
    mgr.handle_key(key_char('X'));

    assert_eq!(mgr.get_text(), "aXbc");
}

#[test]
fn test_home_end_keys() {
    let mut mgr = InputManager::new();

    mgr.handle_key(key_char('h'));
    mgr.handle_key(key_char('e'));
    mgr.handle_key(key_char('l'));
    mgr.handle_key(key_char('l'));
    mgr.handle_key(key_char('o'));

    // Home moves to start
    mgr.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_eq!(mgr.get_cursor_position(), 0);

    // End moves to end
    mgr.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_eq!(mgr.get_cursor_position(), 5);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p synthia --lib ui::input::tests`

Expected: FAIL on cursor movement tests

**Step 3: Implement cursor movement**

Update `handle_key()`, add before the `_` pattern:

```rust
// Left/Right arrows
(KeyCode::Left, _) => {
    self.buffer.move_backward(1);
    InputAction::Redraw
}
(KeyCode::Right, _) => {
    self.buffer.move_forward(1);
    InputAction::Redraw
}

// Home/End
(KeyCode::Home, _) => {
    self.buffer.move_home();
    InputAction::Redraw
}
(KeyCode::End, _) => {
    self.buffer.move_end();
    InputAction::Redraw
}

// Ctrl+A / Ctrl+E (Emacs-style)
(KeyCode::Char('a'), KeyModifiers::CONTROL) => {
    self.buffer.move_home();
    InputAction::Redraw
}
(KeyCode::Char('e'), KeyModifiers::CONTROL) => {
    self.buffer.move_end();
    InputAction::Redraw
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p synthia --lib ui::input::tests`

Expected: All tests pass

**Step 5: Commit**

```bash
git add synthia/src/ui/input.rs
git commit -m "feat: add cursor movement (arrows, home/end, ctrl+a/e)"
```

---

## Task 6: Implement Backspace and Delete

**Files:**
- Modify: `synthia/src/ui/input.rs`

**Step 1: Write failing test**

Add to tests:

```rust
#[test]
fn test_backspace_and_delete() {
    let mut mgr = InputManager::new();

    mgr.handle_key(key_char('h'));
    mgr.handle_key(key_char('e'));
    mgr.handle_key(key_char('l'));
    mgr.handle_key(key_char('l'));
    mgr.handle_key(key_char('o'));

    // Move to position 2 (between 'e' and first 'l')
    mgr.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    mgr.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    mgr.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(mgr.get_cursor_position(), 2);

    // Backspace deletes 'e'
    mgr.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(mgr.get_text(), "hllo");
    assert_eq!(mgr.get_cursor_position(), 1);

    // Delete removes first 'l'
    mgr.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(mgr.get_text(), "hlo");
    assert_eq!(mgr.get_cursor_position(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p synthia --lib ui::input::tests::test_backspace_and_delete`

Expected: FAIL

**Step 3: Implement backspace and delete**

Update `handle_key()`, add before the `_` pattern:

```rust
// Backspace
(KeyCode::Backspace, _) => {
    self.buffer.backspace(1);
    InputAction::Redraw
}

// Delete
(KeyCode::Delete, _) => {
    self.buffer.delete(1);
    InputAction::Redraw
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p synthia --lib ui::input::tests::test_backspace_and_delete`

Expected: PASS

**Step 5: Commit**

```bash
git add synthia/src/ui/input.rs
git commit -m "feat: add backspace and delete support"
```

---

## Task 7: Implement History Navigation

**Files:**
- Modify: `synthia/src/ui/input.rs`

**Step 1: Write failing test**

Add to tests:

```rust
#[test]
fn test_history_navigation() {
    let mut mgr = InputManager::new();

    // Submit first message
    mgr.handle_key(key_char('f'));
    mgr.handle_key(key_char('i'));
    mgr.handle_key(key_char('r'));
    mgr.handle_key(key_char('s'));
    mgr.handle_key(key_char('t'));
    mgr.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Submit second message
    mgr.handle_key(key_char('s'));
    mgr.handle_key(key_char('e'));
    mgr.handle_key(key_char('c'));
    mgr.handle_key(key_char('o'));
    mgr.handle_key(key_char('n'));
    mgr.handle_key(key_char('d'));
    mgr.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Up arrow should recall "second"
    mgr.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(mgr.get_text(), "second");

    // Up again should recall "first"
    mgr.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(mgr.get_text(), "first");

    // Down should go back to "second"
    mgr.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(mgr.get_text(), "second");

    // Down again should clear (end of history)
    mgr.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(mgr.get_text(), "");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p synthia --lib ui::input::tests::test_history_navigation`

Expected: FAIL

**Step 3: Implement history navigation helpers**

Add methods to `InputManager` impl in `synthia/src/ui/input.rs`:

```rust
fn history_prev(&mut self) {
    if self.history.is_empty() {
        return;
    }

    let new_index = match self.history_index {
        None => Some(self.history.len() - 1),
        Some(0) => Some(0), // At oldest, stay there
        Some(i) => Some(i - 1),
    };

    if let Some(idx) = new_index {
        self.buffer = LineBuffer::init(&self.history[idx], 0, None);
        self.buffer.move_end();
        self.history_index = new_index;
    }
}

fn history_next(&mut self) {
    // Guard against empty history
    if self.history.is_empty() {
        return;
    }

    match self.history_index {
        None => {} // Not in history, do nothing
        Some(i) if i >= self.history.len() - 1 => {
            // At newest, go to empty
            self.buffer.clear();
            self.history_index = None;
        }
        Some(i) => {
            let new_idx = i + 1;
            self.buffer = LineBuffer::init(&self.history[new_idx], 0, None);
            self.buffer.move_end();
            self.history_index = Some(new_idx);
        }
    }
}
```

**Step 4: Wire up Up/Down keys**

Update `handle_key()`, add before the `_` pattern:

```rust
// Up/Down for history
(KeyCode::Up, _) => {
    self.history_prev();
    InputAction::Redraw
}
(KeyCode::Down, _) => {
    self.history_next();
    InputAction::Redraw
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p synthia --lib ui::input::tests::test_history_navigation`

Expected: PASS

**Step 6: Commit**

```bash
git add synthia/src/ui/input.rs
git commit -m "feat: add history navigation with up/down arrows"
```

---

## Task 8: Implement Special Keys (Ctrl+C, Ctrl+D, Ctrl+P, Ctrl+S, Ctrl+N, Ctrl+L)

**Files:**
- Modify: `synthia/src/ui/input.rs`

**Step 1: Write failing test**

Add to tests:

```rust
#[test]
fn test_special_keys() {
    let mut mgr = InputManager::new();

    // Ctrl+C cancels
    let action = mgr.handle_key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(action, InputAction::Cancel);

    // Ctrl+D quits
    let action = mgr.handle_key(KeyEvent::new(
        KeyCode::Char('d'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(action, InputAction::Quit);

    // Ctrl+P opens menu
    let action = mgr.handle_key(KeyEvent::new(
        KeyCode::Char('p'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(action, InputAction::OpenMenu);

    // Ctrl+S saves session
    let action = mgr.handle_key(KeyEvent::new(
        KeyCode::Char('s'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(action, InputAction::SaveSession);

    // Ctrl+N new session
    let action = mgr.handle_key(KeyEvent::new(
        KeyCode::Char('n'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(action, InputAction::NewSession);

    // Ctrl+L list sessions
    let action = mgr.handle_key(KeyEvent::new(
        KeyCode::Char('l'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(action, InputAction::ListSessions);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p synthia --lib ui::input::tests::test_special_keys`

Expected: FAIL

**Step 3: Implement special keys**

Update `handle_key()`, add at the BEGINNING (before other patterns):

```rust
// Ctrl+P for menu
(KeyCode::Char('p'), KeyModifiers::CONTROL) => {
    InputAction::OpenMenu
}

// Ctrl+C for cancel
(KeyCode::Char('c'), KeyModifiers::CONTROL) => {
    InputAction::Cancel
}

// Ctrl+D for quit
(KeyCode::Char('d'), KeyModifiers::CONTROL) => {
    InputAction::Quit
}

// Ctrl+S for save session
(KeyCode::Char('s'), KeyModifiers::CONTROL) => {
    InputAction::SaveSession
}

// Ctrl+N for new session
(KeyCode::Char('n'), KeyModifiers::CONTROL) => {
    InputAction::NewSession
}

// Ctrl+L for list sessions
(KeyCode::Char('l'), KeyModifiers::CONTROL) => {
    InputAction::ListSessions
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p synthia --lib ui::input::tests::test_special_keys`

Expected: PASS

**Step 5: Commit**

```bash
git add synthia/src/ui/input.rs
git commit -m "feat: add all keyboard shortcuts

- Ctrl+C: cancel
- Ctrl+D: quit
- Ctrl+P: menu
- Ctrl+S: save session
- Ctrl+N: new session
- Ctrl+L: list sessions"
```

---

## Task 9: Implement Rendering

**Files:**
- Modify: `synthia/src/ui/input.rs`

**Step 1: Implement render() method**

Replace the `render()` method in `synthia/src/ui/input.rs`:

```rust
pub fn render(&self, stdout: &mut impl Write) -> io::Result<()> {
    let (_, cursor_y) = cursor::position()?;

    // Clear input area
    execute!(
        stdout,
        cursor::MoveTo(0, cursor_y),
        Clear(ClearType::FromCursorDown)
    )?;

    // Print colored prompt
    queue!(
        stdout,
        SetForegroundColor(PastelColors::SUCCESS),
        Print(&self.prompt),
        ResetColor
    )?;

    // SIMPLIFIED APPROACH: Let the terminal handle cursor positioning!
    // Split text at cursor position
    let text = self.buffer.as_str();
    let cursor_pos = self.buffer.pos();

    // Split into before/after cursor
    let (before_cursor, after_cursor) = if cursor_pos <= text.len() {
        text.split_at(cursor_pos)
    } else {
        (text, "")
    };

    // Print text BEFORE cursor
    queue!(stdout, Print(before_cursor))?;

    // Ask terminal: "where's the cursor now?" (terminal knows after wrapping!)
    stdout.flush()?;
    let (saved_x, saved_y) = cursor::position()?;

    // Print text AFTER cursor
    queue!(stdout, Print(after_cursor))?;

    // Move cursor back to saved position
    queue!(stdout, cursor::MoveTo(saved_x, saved_y))?;

    stdout.flush()
}
```

**Step 2: Manual test rendering**

Run: `cargo build -p synthia --release`

Expected: Builds successfully

**Step 3: Commit**

```bash
git add synthia/src/ui/input.rs
git commit -m "feat: implement InputManager rendering with multi-line support"
```

---

## Task 10: Add AppMode Enum to App

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Add AppMode and ModalType enums**

Add to `synthia/src/ui/app.rs` after the existing structs (around line 283):

```rust
#[derive(Debug)]
enum AppMode {
    Input,
    Modal(ModalType),
}

#[derive(Debug)]
enum ModalType {
    PermissionPrompt(PermissionApprovalState),
    EditApproval(EditApprovalState),
    Menu { selected: usize },
    ReasoningSubmenu { selected: usize },
    ContextSubmenu { selected: usize },
    SessionList { items: Vec<crate::session::SessionInfo>, selected: usize },
    SessionNameInput { buffer: String, cursor: usize },
    LogViewer { entries: Vec<String>, selected: usize },
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p synthia`

Expected: Compiles (may have unused warnings)

**Step 3: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "refactor: add AppMode and ModalType enums for state management"
```

---

## Task 11: Add InputManager to App Struct

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Import InputManager**

Add to imports at top of `synthia/src/ui/app.rs`:

```rust
use crate::ui::input::{InputManager, InputAction};
```

**Step 2: Add fields to App struct**

Add to `App` struct (around line 284):

```rust
input_manager: InputManager,
mode: AppMode,
saved_input: Option<(String, usize)>,
```

**Step 3: Initialize in App::new()**

Update `App::new()` (around line 313), add after creating the struct:

```rust
input_manager: InputManager::new(),
mode: AppMode::Input,
saved_input: None,
```

**Step 4: Verify it compiles**

Run: `cargo check -p synthia`

Expected: Compiles (may have unused field warnings)

**Step 5: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "refactor: add InputManager and AppMode fields to App struct"
```

---

## Task 12: Add Modal Management Methods

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Add helper methods**

Add to `impl App` (before `run()` method):

```rust
fn has_active_modal(&self) -> bool {
    matches!(self.mode, AppMode::Modal(_))
}

fn enter_modal(&mut self, modal: ModalType) {
    // Save current input buffer
    self.saved_input = Some((
        self.input_manager.get_text().to_string(),
        self.input_manager.get_cursor_position(),
    ));
    self.mode = AppMode::Modal(modal);
}

fn exit_modal(&mut self) {
    // Restore input buffer
    if let Some((text, _cursor)) = self.saved_input.take() {
        // Restore text (cursor will be at end)
        for c in text.chars() {
            use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
            self.input_manager.handle_key(KeyEvent::new(
                KeyCode::Char(c),
                KeyModifiers::NONE
            ));
        }
    }
    self.mode = AppMode::Input;
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p synthia`

Expected: Compiles

**Step 3: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "refactor: add modal management methods (enter/exit/has_active)"
```

---

## Task 13: Update Event Loop to Use InputManager

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Comment out old input handling**

In the `run()` method (around line 366), find the section that processes keyboard events and comment it out. Look for the pattern matching on `Event::Key`.

**Step 2: Add new key and resize handling**

Replace the commented section with:

```rust
// Process keyboard input and resize events
if event::poll(Duration::from_millis(0))? {
    match event::read()? {
        Event::Key(key) => {
            self.handle_key_event(&mut stdout, key).await?;
            had_input = true;
        }
        Event::Resize(cols, rows) => {
            // Force re-render on resize
            match self.mode {
                AppMode::Input => {
                    self.input_manager.render(&mut stdout)?;
                }
                AppMode::Modal(_) => {
                    self.render_modal(&mut stdout)?;
                }
            }
        }
        _ => {} // Ignore other events
    }
}
```

**Step 3: Add handle_key_event() method**

Add new method to `impl App`:

```rust
async fn handle_key_event(
    &mut self,
    stdout: &mut impl Write,
    key: KeyEvent,
) -> anyhow::Result<()> {
    // Update paste detection (keep existing logic)
    let now = std::time::Instant::now();
    if let Some(last_time) = self.last_key_time {
        let elapsed = now.duration_since(last_time);
        if elapsed < std::time::Duration::from_millis(10) {
            self.is_pasting = true;
        } else if elapsed > std::time::Duration::from_millis(100) {
            self.is_pasting = false;
        }
    }
    self.last_key_time = Some(now);

    match self.mode {
        AppMode::Input => {
            let action = self.input_manager.handle_key(key);
            self.handle_input_action(stdout, action).await?;
        }
        AppMode::Modal(_) => {
            // TODO: Handle modal keys (will implement later)
        }
    }

    Ok(())
}
```

**Step 4: Add handle_input_action() method**

Add new method to `impl App`:

```rust
async fn handle_input_action(
    &mut self,
    stdout: &mut impl Write,
    action: InputAction,
) -> anyhow::Result<()> {
    match action {
        InputAction::Submit(text) => {
            // Clear the input line
            execute!(
                stdout,
                cursor::MoveTo(0, cursor::position()?.1),
                Clear(ClearType::CurrentLine)
            )?;

            self.cmd_tx.send(Command::SendMessage(text)).await?;
            self.is_pasting = false;
        }
        InputAction::Cancel => {
            self.cmd_tx.send(Command::Cancel).await?;
        }
        InputAction::Quit => {
            self.cmd_tx.send(Command::Shutdown).await?;
            self.should_quit = true;
        }
        InputAction::OpenMenu => {
            self.enter_modal(ModalType::Menu { selected: 0 });
        }
        InputAction::SaveSession => {
            self.cmd_tx.send(Command::SaveSession).await?;
        }
        InputAction::NewSession => {
            self.cmd_tx.send(Command::NewSession).await?;
        }
        InputAction::ListSessions => {
            self.cmd_tx.send(Command::ListSessions).await?;
        }
        InputAction::Redraw | InputAction::None => {
            // Will be rendered in main loop
        }
    }
    Ok(())
}
```

**Step 5: Update rendering in main loop**

Find the section that calls `render_input_line()` and replace with:

```rust
// Render current mode
match self.mode {
    AppMode::Input => {
        self.input_manager.render(&mut stdout)?;
    }
    AppMode::Modal(_) => {
        // TODO: Render modal (will implement later)
    }
}
```

**Step 6: Verify it compiles**

Run: `cargo check -p synthia`

Expected: Compiles (may have warnings about unused code)

**Step 7: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "refactor: integrate InputManager into event loop"
```

---

## Task 14: Delete Old Input Rendering Code

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Delete render_input_line() method**

Find and delete the entire `render_input_line()` method (around lines 811-903).

**Step 2: Delete helper methods**

Delete these methods:
- `char_to_byte_pos()` (around lines 354-360)
- `input_char_len()` (around lines 362-364)

**Step 3: Delete old input/cursor fields from App struct**

Remove these fields from `App`:
- `input: String`
- `cursor_position: usize`
- `input_needs_render: bool`
- `is_rendering_input: bool`

**Step 4: Remove old field initializations from App::new()**

Remove initializations for the deleted fields.

**Step 5: Verify it compiles**

Run: `cargo check -p synthia`

Expected: Compiles

**Step 6: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "refactor: delete old cursor calculation and render_input_line()"
```

---

## Task 15: Update Remaining Tests

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Run existing tests**

Run: `cargo test -p synthia`

Expected: Most wrapping tests should still pass (they test output, not input)

**Step 2: Fix or remove tests that reference deleted fields**

If any tests reference `app.input` or `app.cursor_position`, update them to use:
- `app.input_manager.get_text()` instead of `app.input`
- `app.input_manager.get_cursor_position()` instead of `app.cursor_position`

**Step 3: Run tests again**

Run: `cargo test -p synthia`

Expected: All tests pass

**Step 4: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "test: update tests to use InputManager"
```

---

## Task 16: Manual Integration Testing

**Files:**
- N/A (manual testing)

**Step 1: Build release binary**

Run: `cargo build -p synthia --release`

Expected: Builds successfully

**Step 2: Run synthia**

Run: `./target/release/synthia`

**Step 3: Test basic input**

1. Type: "hello" and press Enter
2. Expected: Message sends, no duplication

**Step 4: Test long input (line wrap)**

1. Type a message longer than terminal width
2. Expected: Text wraps cleanly, NO DUPLICATION!
3. Verify cursor positioning is correct

**Step 5: Test multi-line input**

1. Type: "line 1" then press Shift+Enter
2. Type: "line 2" then press Enter
3. Expected: Message sends with newline preserved

**Step 6: Test paste**

1. Paste a large block of text (>1000 chars)
2. Expected: No duplication, all text appears

**Step 7: Test history**

1. Send message "first"
2. Send message "second"
3. Press Up arrow
4. Expected: "second" appears
5. Press Up arrow
6. Expected: "first" appears

**Step 8: Test cursor movement**

1. Type "hello"
2. Press Home
3. Press Right 2 times
4. Type "X"
5. Expected: "heXllo"

**Step 9: Test Ctrl+P menu**

1. Type some text
2. Press Ctrl+P
3. Expected: Menu appears (may be broken - will fix next)
4. Press Esc
5. Expected: Returns to input with text preserved

**Step 10: Document results**

Create test report: `docs/manual-test-results.md`

```markdown
# Manual Test Results - Rustyline Migration

Date: [current date]

## Test Results

- [ ] Basic input: PASS/FAIL
- [ ] Long input (line wrap): PASS/FAIL
- [ ] Multi-line input: PASS/FAIL
- [ ] Paste: PASS/FAIL
- [ ] History: PASS/FAIL
- [ ] Cursor movement: PASS/FAIL
- [ ] Ctrl+P menu: PASS/FAIL

## Issues Found

[List any issues]

## Notes

[Any observations]
```

**Step 11: Commit test results**

```bash
git add docs/manual-test-results.md
git commit -m "test: add manual integration test results"
```

---

## Task 17: Migrate Modal State to AppMode (REQUIRED)

**Why this is required**: Task 14 deletes fields like `show_menu`, `pending_edit_approval`, etc. that modals depend on. This migration is NOT optional - modals will break without it.

**Files:**
- Modify: `synthia/src/ui/app.rs`

**Step 1: Consolidate old modal flags into ModalType**

Find all these flags in `App` struct and remove them (Task 14 already deleted some):
- `show_menu: bool`
- `menu_selected: usize`
- `show_reasoning_submenu: bool`
- `reasoning_submenu_selected: usize`
- `show_context_submenu: bool`
- `context_submenu_selected: usize`
- `show_session_name_input: bool`
- `session_name_input: String`
- `session_name_cursor: usize`
- `show_session_list: bool`
- `session_list_selected: usize`
- `show_log_viewer: bool`
- `log_entries: Vec<String>`
- `log_viewer_selected: usize`

**Step 2: Update modal checks to use AppMode**

Find all checks like `if self.show_menu` and replace with:

```rust
if matches!(self.mode, AppMode::Modal(ModalType::Menu { .. }))
```

**Step 3: Update modal rendering**

Find `render_menu()`, `render_permission_prompt()`, etc. and update to work with `ModalType`.

**Step 4: Test modals**

Run: `./target/release/synthia`

Test: Press Ctrl+P, verify menu appears

**Step 5: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "refactor: consolidate modal state into ModalType enum"
```

---

## Task 18: Final Cleanup

**Files:**
- Modify: `synthia/src/ui/app.rs`
- Modify: `synthia/Cargo.toml`

**Step 1: Run clippy**

Run: `cargo clippy -p synthia`

Expected: Fix any warnings

**Step 2: Check if ratatui is still needed**

Search for "ratatui" usage in synthia:

Run: `rg "ratatui" synthia/src`

If only used in markdown.rs, consider keeping (or note for future removal).

**Step 3: Format code**

Run: `cargo fmt -p synthia`

**Step 4: Final test**

Run: `cargo test -p synthia && cargo build -p synthia --release`

Expected: All tests pass, builds successfully

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: final cleanup and formatting"
```

---

## Success Criteria

✅ Type long message → no character duplication
✅ Paste large text → no duplication
✅ Multi-line input works (Shift+Enter)
✅ Cursor positioning correct
✅ Backspace/delete work
✅ History navigation works
✅ All modals preserve input
✅ No compiler warnings
✅ All tests pass

---

## Reference Documentation

**Rustyline LineBuffer API:**
- https://docs.rs/rustyline/latest/rustyline/line_buffer/struct.LineBuffer.html

**Crossterm Event Handling:**
- https://docs.rs/crossterm/latest/crossterm/event/index.html

**Related Skills:**
- @superpowers:test-driven-development - Follow TDD cycle for each task
- @superpowers:verification-before-completion - Verify tests pass before committing
- @superpowers:systematic-debugging - If bugs found during testing

---

**Estimated Time:** 12-16 hours total (18 tasks × 30-50 minutes each)

**Risk Level:** Medium (architectural refactor, but incremental with tests)

**Dependencies:** None (all work in synthia crate)
