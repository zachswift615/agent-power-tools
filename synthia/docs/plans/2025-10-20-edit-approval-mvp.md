# Edit Approval MVP Implementation Plan

> **For Claude:** Use `${SUPERPOWERS_SKILLS_ROOT}/skills/collaboration/executing-plans/SKILL.md` to implement this plan task-by-task.

**Goal:** Add user approval prompts for edit operations with diff preview, allowing users to accept/reject edits before they execute.

**Architecture:** UI-only implementation that intercepts edit tool calls in ToolRegistry, shows diff preview, waits for user input, then executes or cancels. Assistant never sees the approval step - tool returns either success or "Edit cancelled by user".

**Tech Stack:** Rust, tokio async, crossterm (TUI), similar-rs (diff computation)

---

## Phase 1: Foundation - Diff Computation

### Task 1: Add diff computation dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add similar crate for diff computation**

Add to dependencies:
```toml
similar = { version = "2.3", features = ["inline"] }
```

**Step 2: Build to verify dependency**

Run: `cargo build`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add similar crate for diff computation"
```

---

## Phase 2: Edit Preview UI Update

### Task 2: Add EditPreview UI update variant

**Files:**
- Modify: `src/agent/messages.rs:13-32`
- Test: Manual verification (no existing tests for messages.rs)

**Step 1: Write the test first**

This is an enum variant addition - no test needed, but we'll verify in integration testing.

**Step 2: Add EditPreview variant to UIUpdate enum**

In `src/agent/messages.rs`, add new variant after line 31:

```rust
    EditPreview {
        file_path: String,
        old_string: String,
        new_string: String,
        diff: String,
    },
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/agent/messages.rs
git commit -m "feat: add EditPreview UI update variant"
```

---

## Phase 3: Approval Response Mechanism

### Task 3: Add approval response channel

**Files:**
- Modify: `src/agent/messages.rs:1-10`

**Step 1: Add ApprovalResponse enum**

Add to top of file after line 10:

```rust
#[derive(Debug, Clone)]
pub enum ApprovalResponse {
    Approve,
    Reject,
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/agent/messages.rs
git commit -m "feat: add ApprovalResponse enum for edit approval"
```

---

## Phase 4: Diff Computation in Edit Tool

### Task 4: Add diff computation utility

**Files:**
- Create: `src/tools/diff.rs`
- Modify: `src/tools/mod.rs:1-13`

**Step 1: Write the failing test**

Create `src/tools/diff.rs`:

```rust
use similar::{ChangeTag, TextDiff};

pub fn compute_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut result = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        result.push_str(&format!("{}{}", sign, change));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff_addition() {
        let old = "line 1\nline 2\n";
        let new = "line 1\nline 2\nline 3\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains(" line 1"));
        assert!(diff.contains(" line 2"));
        assert!(diff.contains("+line 3"));
    }
}
```

**Step 2: Run test to verify it fails (or passes if already correct)**

Run: `cargo test test_compute_diff_addition`
Expected: Test should pass (diff computation is correct)

**Step 3: Add more tests**

Add to tests module:

```rust
    #[test]
    fn test_compute_diff_deletion() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 3\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains(" line 1"));
        assert!(diff.contains("-line 2"));
        assert!(diff.contains(" line 3"));
    }

    #[test]
    fn test_compute_diff_modification() {
        let old = "hello world\n";
        let new = "hello Synthia\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains("-hello world"));
        assert!(diff.contains("+hello Synthia"));
    }

    #[test]
    fn test_compute_diff_no_change() {
        let old = "same\n";
        let new = "same\n";
        let diff = compute_diff(old, new);

        assert!(diff.contains(" same"));
        assert!(!diff.contains("+"));
        assert!(!diff.contains("-"));
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test diff`
Expected: All tests pass

**Step 5: Add module to tools/mod.rs**

Add to `src/tools/mod.rs` after line 3:

```rust
pub mod diff;
```

**Step 6: Commit**

```bash
git add src/tools/diff.rs src/tools/mod.rs
git commit -m "feat: add diff computation utility with tests"
```

---

## Phase 5: Edit Approval UI Handler

### Task 5: Add edit approval prompt to TUI

**Files:**
- Modify: `src/ui/app.rs:82-96` (App struct)
- Modify: `src/ui/app.rs:195-404` (handle_ui_update method)

**Step 1: Add approval state to App struct**

In `src/ui/app.rs`, add fields to App struct after line 95:

```rust
    pending_edit_approval: Option<EditApprovalState>,
```

Add state struct before App impl:

```rust
#[derive(Debug, Clone)]
struct EditApprovalState {
    file_path: String,
    old_string: String,
    new_string: String,
    diff: String,
}
```

**Step 2: Initialize new field in App::new()**

In `src/ui/app.rs:99-115`, add to initialization:

```rust
            pending_edit_approval: None,
```

**Step 3: Handle EditPreview in handle_ui_update**

In `src/ui/app.rs`, add to the match statement around line 400 (before the closing brace):

```rust
            UIUpdate::EditPreview {
                file_path,
                old_string,
                new_string,
                diff,
            } => {
                self.clear_input_line(stdout)?;

                // Store approval state
                self.pending_edit_approval = Some(EditApprovalState {
                    file_path: file_path.clone(),
                    old_string,
                    new_string,
                    diff: diff.clone(),
                });

                // Show diff preview
                self.render_edit_approval_prompt(stdout, &file_path, &diff)?;
            }
```

**Step 4: Add render_edit_approval_prompt method**

Add method to App impl (after render_session_list):

```rust
    fn render_edit_approval_prompt(&self, stdout: &mut impl Write, file_path: &str, diff: &str) -> io::Result<()> {
        queue!(
            stdout,
            SetForegroundColor(Color::Yellow),
            Print("┌─ Edit Preview ────────────────────────────────────────┐\n"),
            ResetColor
        )?;

        writeln!(stdout, "│ File: {}", file_path)?;
        writeln!(stdout, "│")?;

        // Show diff (truncate if too long)
        let max_lines = 15;
        let diff_lines: Vec<&str> = diff.lines().take(max_lines).collect();

        for line in diff_lines {
            let color = if line.starts_with('+') {
                Color::Green
            } else if line.starts_with('-') {
                Color::Red
            } else {
                Color::White
            };

            queue!(stdout, Print("│ "), SetForegroundColor(color))?;
            writeln!(stdout, "{}", line)?;
            queue!(stdout, ResetColor)?;
        }

        if diff.lines().count() > max_lines {
            writeln!(stdout, "│ ...")?;
        }

        writeln!(stdout, "│")?;
        queue!(
            stdout,
            SetForegroundColor(Color::Cyan),
            Print("│ [A]ccept  [R]eject\n"),
            ResetColor
        )?;

        queue!(
            stdout,
            SetForegroundColor(Color::Yellow),
            Print("└───────────────────────────────────────────────────────┘\n"),
            ResetColor
        )?;

        stdout.flush()
    }
```

**Step 5: Handle approval input in handle_input**

In `src/ui/app.rs`, modify handle_input to check for approval state. Add after line 548 (before normal input handling):

```rust
        // Handle edit approval input
        if self.pending_edit_approval.is_some() {
            match (key.code, key.modifiers) {
                (KeyCode::Char('a'), _) | (KeyCode::Char('A'), _) => {
                    // Send approval
                    // TODO: Send approval response through channel
                    self.pending_edit_approval = None;
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                (KeyCode::Char('r'), _) | (KeyCode::Char('R'), _) => {
                    // Send rejection
                    // TODO: Send rejection response through channel
                    self.pending_edit_approval = None;
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }
```

**Step 6: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully (we'll add channel logic next)

**Step 7: Commit**

```bash
git add src/ui/app.rs
git commit -m "feat: add edit approval UI prompt (TODO: wire up response channel)"
```

---

## Phase 6: Approval Channel Wiring

### Task 6: Add approval response channel to App

**Files:**
- Modify: `src/ui/app.rs:82-96` (App struct)
- Modify: `src/ui/app.rs:98-115` (App::new)
- Modify: `src/main.rs` (will need to create channels)

**Step 1: Add approval channel fields to App struct**

In `src/ui/app.rs`, modify App struct to add:

```rust
    approval_tx: Option<tokio::sync::oneshot::Sender<ApprovalResponse>>,
```

**Step 2: Update EditApprovalState to include channel**

Modify EditApprovalState:

```rust
#[derive(Debug)]
struct EditApprovalState {
    file_path: String,
    old_string: String,
    new_string: String,
    diff: String,
    response_tx: tokio::sync::oneshot::Sender<crate::agent::messages::ApprovalResponse>,
}
```

**Step 3: Initialize in App::new()**

Add to initialization:

```rust
            approval_tx: None,
```

**Step 4: Update EditPreview handler to accept response channel**

We need to modify UIUpdate::EditPreview to include the channel. Update in `src/agent/messages.rs`:

```rust
    EditPreview {
        file_path: String,
        old_string: String,
        new_string: String,
        diff: String,
        response_tx: tokio::sync::oneshot::Sender<ApprovalResponse>,
    },
```

**Step 5: Update handler in App to store channel**

Modify the EditPreview handler in `handle_ui_update`:

```rust
            UIUpdate::EditPreview {
                file_path,
                old_string,
                new_string,
                diff,
                response_tx,
            } => {
                self.clear_input_line(stdout)?;

                // Store approval state with channel
                self.pending_edit_approval = Some(EditApprovalState {
                    file_path: file_path.clone(),
                    old_string,
                    new_string,
                    diff: diff.clone(),
                    response_tx,
                });

                // Show diff preview
                self.render_edit_approval_prompt(stdout, &file_path, &diff)?;
            }
```

**Step 6: Wire up approval/rejection in handle_input**

Update the approval handling:

```rust
        // Handle edit approval input
        if let Some(approval_state) = self.pending_edit_approval.take() {
            match (key.code, key.modifiers) {
                (KeyCode::Char('a'), _) | (KeyCode::Char('A'), _) => {
                    let _ = approval_state.response_tx.send(crate::agent::messages::ApprovalResponse::Approve);
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                (KeyCode::Char('r'), _) | (KeyCode::Char('R'), _) => {
                    let _ = approval_state.response_tx.send(crate::agent::messages::ApprovalResponse::Reject);
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                    self.print_header(stdout)?;
                    return Ok(());
                }
                _ => {
                    // Put it back if user didn't approve/reject
                    self.pending_edit_approval = Some(approval_state);
                    return Ok(());
                }
            }
        }
```

**Step 7: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 8: Commit**

```bash
git add src/ui/app.rs src/agent/messages.rs
git commit -m "feat: wire up approval response channel in UI"
```

---

## Phase 7: Tool Registry Interception

### Task 7: Intercept edit calls in ToolRegistry

**Files:**
- Modify: `src/tools/registry.rs:37-58`
- Modify: `src/tools/edit.rs:46-73` (to get current content before executing)

**Step 1: Add UI update sender to ToolRegistry**

Modify ToolRegistry struct in `src/tools/registry.rs`:

```rust
use tokio::sync::mpsc::Sender;
use crate::agent::messages::UIUpdate;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    cache: ToolCache,
    ui_tx: Option<Sender<UIUpdate>>,
}
```

**Step 2: Update ToolRegistry::new() and add set_ui_sender**

```rust
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            cache: ToolCache::new(100),
            ui_tx: None,
        }
    }

    pub fn set_ui_sender(&mut self, ui_tx: Sender<UIUpdate>) {
        self.ui_tx = Some(ui_tx);
    }
```

**Step 3: Add edit interception logic to execute()**

Modify execute() method to intercept edits:

```rust
    pub async fn execute(&self, name: &str, params: Value) -> Result<ToolResult> {
        // Intercept edit tool for approval
        if name == "edit" && self.ui_tx.is_some() {
            return self.execute_edit_with_approval(params).await;
        }

        // [existing execute logic continues unchanged]
        // Check cache first for deterministic tools
        if Self::is_deterministic(name) {
            if let Some(cached) = self.cache.get(name, &params) {
                tracing::debug!("Tool cache hit: {}", name);
                return Ok(cached);
            }
        }

        // Execute tool
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;
        let result = tool.execute(params.clone()).await?;

        // Cache result if tool is deterministic
        if Self::is_deterministic(name) {
            self.cache.put(name, &params, result.clone());
        }

        Ok(result)
    }
```

**Step 4: Add execute_edit_with_approval method**

```rust
    async fn execute_edit_with_approval(&self, params: Value) -> Result<ToolResult> {
        use crate::tools::diff::compute_diff;

        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing file_path"))?;
        let old_string = params["old_string"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing old_string"))?;
        let new_string = params["new_string"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing new_string"))?;

        // Read current file content
        let content = tokio::fs::read_to_string(file_path).await?;

        if !content.contains(old_string) {
            return Ok(ToolResult {
                content: format!("String '{}' not found in file", old_string),
                is_error: true,
            });
        }

        // Compute diff
        let new_content = content.replace(old_string, new_string);
        let diff = compute_diff(&content, &new_content);

        // Create approval channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Send preview to UI
        if let Some(ui_tx) = &self.ui_tx {
            ui_tx
                .send(UIUpdate::EditPreview {
                    file_path: file_path.to_string(),
                    old_string: old_string.to_string(),
                    new_string: new_string.to_string(),
                    diff,
                    response_tx,
                })
                .await?;
        }

        // Wait for user response
        match response_rx.await {
            Ok(crate::agent::messages::ApprovalResponse::Approve) => {
                // Execute the edit
                let tool = self.get("edit").ok_or_else(|| anyhow!("Edit tool not found"))?;
                tool.execute(params).await
            }
            Ok(crate::agent::messages::ApprovalResponse::Reject) => {
                // User rejected
                Ok(ToolResult {
                    content: "Edit cancelled by user".to_string(),
                    is_error: false,
                })
            }
            Err(_) => {
                // Channel closed (user disconnected?)
                Ok(ToolResult {
                    content: "Edit approval cancelled".to_string(),
                    is_error: true,
                })
            }
        }
    }
```

**Step 5: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/tools/registry.rs
git commit -m "feat: intercept edit tool calls for approval in ToolRegistry"
```

---

## Phase 8: Integration and Testing

### Task 8: Wire up ToolRegistry UI sender in main.rs

**Files:**
- Modify: `src/main.rs` (find where ToolRegistry is created)

**Step 1: Find ToolRegistry creation**

Run: `grep -n "ToolRegistry::new" src/*.rs`

**Step 2: Add UI sender to registry**

After creating ToolRegistry, call:

```rust
registry.set_ui_sender(ui_tx.clone());
```

**Step 3: Verify it compiles and runs**

Run: `cargo build`
Expected: Builds successfully

Run: `cargo run`
Expected: App starts without errors

**Step 4: Manual test - trigger an edit**

Send message: "Edit the file test.txt, replace 'hello' with 'world'"

Expected behavior:
1. Edit preview prompt appears
2. Shows diff with +/- lines
3. User can press 'A' to accept or 'R' to reject
4. Edit executes or cancels accordingly

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up UI sender to ToolRegistry for edit approval"
```

---

## Phase 9: Config Option

### Task 9: Add edit_approval config option

**Files:**
- Modify: `src/config.rs:70-80` (UIConfig)
- Create test in `src/config.rs` tests module

**Step 1: Write the test first**

Add to tests in `src/config.rs`:

```rust
    #[test]
    fn test_edit_approval_default() {
        let config = UIConfig::default();
        assert_eq!(config.edit_approval, true); // Enabled by default
    }

    #[test]
    fn test_edit_approval_from_toml() {
        let toml_str = r#"
            [ui]
            edit_approval = false
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ui.edit_approval, false);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_edit_approval_default`
Expected: FAIL - field doesn't exist

**Step 3: Add edit_approval field to UIConfig**

In `src/config.rs:72-80`, add:

```rust
    /// Enable edit approval prompts
    #[serde(default = "default_edit_approval")]
    pub edit_approval: bool,
```

Add default function:

```rust
fn default_edit_approval() -> bool {
    true
}
```

**Step 4: Update UIConfig::default()**

```rust
impl Default for UIConfig {
    fn default() -> Self {
        Self {
            syntax_highlighting: default_syntax_highlighting(),
            max_output_lines: default_max_output_lines(),
            edit_approval: default_edit_approval(),
        }
    }
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test config`
Expected: All tests pass

**Step 6: Use config in ToolRegistry**

Modify `execute_edit_with_approval` to check config (requires passing config to registry).

For MVP, skip config checking - always prompt. Add TODO comment:

```rust
// TODO: Check config.ui.edit_approval before prompting
// For MVP, always prompt
```

**Step 7: Commit**

```bash
git add src/config.rs
git commit -m "feat: add edit_approval config option (always enabled in MVP)"
```

---

## Phase 10: Documentation and Polish

### Task 10: Update README and add usage docs

**Files:**
- Create: `docs/EDIT_APPROVAL.md`
- Modify: `README.md`

**Step 1: Create usage documentation**

Create `docs/EDIT_APPROVAL.md`:

```markdown
# Edit Approval Feature

Synthia now prompts for approval before executing file edits, showing a diff preview of the changes.

## How It Works

1. AI proposes an edit (e.g., "Replace X with Y in file.rs")
2. Synthia shows a diff preview:
   ```
   ┌─ Edit Preview ────────────────────────────────────┐
   │ File: src/app.rs
   │
   │ - let x = 5;
   │ + let x = 10;
   │
   │ [A]ccept  [R]eject
   └───────────────────────────────────────────────────┘
   ```
3. User presses:
   - `A` to accept and execute the edit
   - `R` to reject and cancel

## Configuration

Enable/disable in `~/.config/synthia/config.toml`:

```toml
[ui]
edit_approval = true  # false to disable prompts
```

Default: `true` (prompts enabled)

## Limitations

- MVP only supports edit tool (bash commands not yet included)
- No "always allow" patterns yet (coming in Phase 2)
- No session-wide approval mode yet (coming in Phase 3)

See `EDIT_APPROVAL_PLAN.md` for future phases.
```

**Step 2: Update README.md**

Add feature section:

```markdown
## Features

- **Edit Approval with Diff Preview** - Review all file edits before they execute, with syntax-highlighted diffs
```

**Step 3: Commit**

```bash
git add docs/EDIT_APPROVAL.md README.md
git commit -m "docs: add edit approval feature documentation"
```

---

## Testing Checklist

Before marking Phase 1 complete, verify:

- [ ] Unit tests pass: `cargo test`
- [ ] App builds: `cargo build`
- [ ] App runs: `cargo run`
- [ ] Edit preview appears when AI tries to edit a file
- [ ] Diff shows correct +/- lines with colors
- [ ] Pressing 'A' executes the edit
- [ ] Pressing 'R' cancels the edit with "Edit cancelled by user" message
- [ ] Other keys don't execute or cancel (gracefully ignored)
- [ ] Config option exists and loads from TOML

---

## Future Phases (Not in This Plan)

- **Phase 2:** Permission config storage (.claude/settings.local.json)
- **Phase 3:** Dynamic permission learning ("Always allow *.rs edits")
- **Phase 4:** Command approval (bash tool interception)
- **Phase 5:** Advanced features (audit logs, bulk approval, expiry)

See `EDIT_APPROVAL_PLAN.md` for complete roadmap.
