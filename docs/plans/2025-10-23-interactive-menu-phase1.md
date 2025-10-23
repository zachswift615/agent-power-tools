# Interactive Menu System - Phase 1 Implementation Plan

> **For Claude:** Use `${SUPERPOWERS_SKILLS_ROOT}/skills/collaboration/subagent-driven-development/SKILL.md` to implement this plan task-by-task.

**Goal:** Add an interactive menu system (Ctrl+M) with session naming and reasoning level configuration.

**Architecture:**
- Extend the existing App struct with menu modal state
- Add session `name` field to Session struct (optional, AI-generated from first message)
- Add `reasoning_level` field to GenerationConfig (per-session, default: "medium")
- Menu items: Set Session Name, Save Session, New Session, Set Reasoning Level, [Context Management - Coming Soon], [Toggle Mode - Coming Soon]

**Tech Stack:** Rust, crossterm, tokio, serde (for session serialization)

---

## Task 1: Add session name field to Session struct

**Files:**
- Modify: `synthia/src/session.rs` (Session struct)
- Modify: `synthia/src/agent/messages.rs` (add SetSessionName command)
- Test: Manual testing (automated tests exist in session.rs, will verify they still pass)

**Step 1: Add name field to Session struct**

```rust
// In synthia/src/session.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: Option<String>,  // NEW: Optional friendly name
    pub created_at: i64,
    pub last_modified: i64,
    pub model: String,
    pub messages: Vec<Message>,
}
```

**Step 2: Update Session::new() to initialize name as None**

```rust
impl Session {
    pub fn new(model: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: generate_session_id(),
            name: None,  // NEW
            created_at: now,
            last_modified: now,
            model,
            messages: Vec::new(),
        }
    }
```

**Step 3: Add set_name() method to Session**

```rust
impl Session {
    // ... existing methods ...

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
        self.last_modified = chrono::Utc::now().timestamp();
    }
}
```

**Step 4: Update SessionInfo to include name**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: Option<String>,  // NEW
    pub created_at: i64,
    pub last_modified: i64,
    pub model: String,
    pub message_count: usize,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            name: session.name.clone(),  // NEW
            created_at: session.created_at,
            last_modified: session.last_modified,
            model: session.model.clone(),
            message_count: session.messages.len(),
        }
    }
}
```

**Step 5: Run existing tests to verify they still pass**

```bash
cd synthia && cargo test --lib session
```

Expected: All tests pass (existing tests should handle Option<String> gracefully)

**Step 6: Commit**

```bash
git add synthia/src/session.rs
git commit -m "feat(session): Add optional name field to Session struct"
```

---

## Task 2: Add reasoning level to GenerationConfig

**Files:**
- Modify: `synthia/src/llm/mod.rs` (GenerationConfig struct)
- Modify: `synthia/src/llm/provider.rs` (LLMProvider trait if needed)
- Modify: `synthia/src/llm/openai.rs` (inject reasoning level into system prompt)

**Step 1: Add reasoning_level field to GenerationConfig**

```rust
// In synthia/src/llm/mod.rs
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub streaming: bool,
    pub reasoning_level: String,  // NEW: "low", "medium", "high"
}
```

**Step 2: Update main.rs to initialize with default "medium"**

```rust
// In synthia/src/main.rs
let gen_config = GenerationConfig {
    model: config.llm.model.clone(),
    temperature: config.llm.temperature,
    max_tokens: config.llm.max_tokens,
    streaming: config.llm.streaming,
    reasoning_level: "medium".to_string(),  // NEW: default
};
```

**Step 3: Update OpenAICompatibleProvider to inject reasoning into system prompt**

```rust
// In synthia/src/llm/openai.rs, in the generate() method
// Find where system prompt is constructed and add:

let system_content = if let Some(first) = messages.first() {
    if matches!(first.role, Role::System) {
        if let Some(ContentBlock::Text { text }) = first.content.first() {
            // Append reasoning level to existing system prompt
            format!("{}\n\nReasoning: {}", text, config.reasoning_level)
        } else {
            format!("Reasoning: {}", config.reasoning_level)
        }
    } else {
        format!("Reasoning: {}", config.reasoning_level)
    }
} else {
    format!("Reasoning: {}", config.reasoning_level)
};

// Then use this system_content when building the request
```

**Step 4: Test by running synthia and checking logs**

```bash
cd synthia && cargo run
# Check /tmp/synthia.log to verify system prompt includes "Reasoning: medium"
```

Expected: Log shows system prompt with reasoning level

**Step 5: Commit**

```bash
git add synthia/src/llm/mod.rs synthia/src/llm/openai.rs synthia/src/main.rs
git commit -m "feat(llm): Add reasoning_level to GenerationConfig with system prompt injection"
```

---

## Task 3: Add Command variants for menu actions

**Files:**
- Modify: `synthia/src/agent/messages.rs` (Command enum)

**Step 1: Add new Command variants**

```rust
// In synthia/src/agent/messages.rs
#[derive(Debug, Clone)]
pub enum Command {
    // ... existing variants ...
    SetSessionName(String),       // NEW
    SetReasoningLevel(String),    // NEW: "low", "medium", "high"
    ShowMenu,                     // NEW: trigger menu display
}
```

**Step 2: Run cargo check to verify no compilation errors**

```bash
cd synthia && cargo check
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add synthia/src/agent/messages.rs
git commit -m "feat(messages): Add Command variants for menu actions"
```

---

## Task 4: Handle new commands in AgentActor

**Files:**
- Modify: `synthia/src/agent/actor.rs` (handle SetSessionName and SetReasoningLevel)

**Step 1: Add handlers in AgentActor::run() command match**

```rust
// In synthia/src/agent/actor.rs, in the command match statement
Command::SetSessionName(name) => {
    if let Some(session) = &mut self.current_session {
        session.set_name(name.clone());
        if let Err(e) = session.save() {
            tracing::error!("Failed to save session after setting name: {}", e);
        } else {
            tracing::info!("Session name set to: {}", name);
        }
    }
}
Command::SetReasoningLevel(level) => {
    // Validate level
    if !["low", "medium", "high"].contains(&level.as_str()) {
        tracing::warn!("Invalid reasoning level: {}, ignoring", level);
        continue;
    }
    self.generation_config.reasoning_level = level.clone();
    tracing::info!("Reasoning level set to: {}", level);
}
Command::ShowMenu => {
    // UI will handle the menu display, just send a response
    let _ = self.ui_tx.send(UIUpdate::MenuDisplayRequested).await;
}
```

**Step 2: Add MenuDisplayRequested to UIUpdate enum**

```rust
// In synthia/src/agent/messages.rs
#[derive(Debug)]
pub enum UIUpdate {
    // ... existing variants ...
    MenuDisplayRequested,  // NEW
}
```

**Step 3: Run cargo check**

```bash
cd synthia && cargo check
```

Expected: Compiles successfully

**Step 4: Commit**

```bash
git add synthia/src/agent/actor.rs synthia/src/agent/messages.rs
git commit -m "feat(agent): Handle SetSessionName and SetReasoningLevel commands"
```

---

## Task 5: Implement menu UI in App struct

**Files:**
- Modify: `synthia/src/ui/app.rs` (add menu state and rendering)

**Step 1: Add menu state to App struct**

```rust
// In synthia/src/ui/app.rs
pub struct App {
    // ... existing fields ...
    show_menu: bool,              // NEW
    menu_selected: usize,         // NEW: selected menu item index
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            // ... existing fields ...
            show_menu: false,         // NEW
            menu_selected: 0,         // NEW
        }
    }
```

**Step 2: Add menu rendering method**

```rust
// In synthia/src/ui/app.rs
impl App {
    fn render_menu(&self, stdout: &mut impl Write) -> io::Result<()> {
        self.clear_input_line(stdout)?;

        writeln!(stdout, "\n=== Synthia Menu (↑/↓ navigate | Enter select | Esc cancel) ===")?;

        let menu_items = vec![
            "Set Session Name",
            "Save Session",
            "New Session",
            "Set Reasoning Level",
            "Context Management (Coming Soon)",
            "Toggle Mode (Coming Soon)",
        ];

        for (idx, item) in menu_items.iter().enumerate() {
            let selected = if idx == self.menu_selected { ">" } else { " " };

            if idx == self.menu_selected {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
            }

            // Dim "Coming Soon" items
            if item.contains("Coming Soon") {
                queue!(stdout, SetForegroundColor(Color::DarkGrey))?;
            }

            writeln!(stdout, "{} {}", selected, item)?;

            if idx == self.menu_selected || item.contains("Coming Soon") {
                queue!(stdout, ResetColor)?;
            }
        }

        writeln!(stdout)?;
        stdout.flush()
    }
```

**Step 3: Handle Ctrl+M to show menu in handle_input()**

```rust
// In synthia/src/ui/app.rs, in handle_input() method, add before session list handling:
(KeyCode::Char('m'), KeyModifiers::CONTROL) => {
    self.show_menu = true;
    self.menu_selected = 0;
    self.render_menu(stdout)?;
}
```

**Step 4: Add menu navigation in handle_input()**

```rust
// In synthia/src/ui/app.rs, in handle_input() method, add after edit approval handling:
if self.show_menu {
    match key.code {
        KeyCode::Up => {
            if self.menu_selected > 0 {
                self.menu_selected -= 1;
                self.render_menu(stdout)?;
            }
            return Ok(());
        }
        KeyCode::Down => {
            let menu_item_count = 6;  // Total menu items
            if self.menu_selected < menu_item_count - 1 {
                self.menu_selected += 1;
                self.render_menu(stdout)?;
            }
            return Ok(());
        }
        KeyCode::Enter => {
            self.handle_menu_selection(stdout).await?;
            return Ok(());
        }
        KeyCode::Esc => {
            self.show_menu = false;
            execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            self.print_header(stdout)?;
            return Ok(());
        }
        _ => return Ok(()),
    }
}
```

**Step 5: Add handle_menu_selection() method**

```rust
// In synthia/src/ui/app.rs
impl App {
    async fn handle_menu_selection(&mut self, stdout: &mut impl Write) -> anyhow::Result<()> {
        match self.menu_selected {
            0 => {
                // Set Session Name
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;

                // Prompt for name
                print_colored_line(stdout, "Enter session name:", Color::Yellow)?;
                self.input.clear();
                self.cursor_position = 0;
                self.input_needs_render = true;

                // TODO: Capture input and send SetSessionName command
                // For now, this is a placeholder
            }
            1 => {
                // Save Session
                self.cmd_tx.send(Command::SaveSession).await?;
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            2 => {
                // New Session
                self.cmd_tx.send(Command::NewSession).await?;
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            3 => {
                // Set Reasoning Level - show submenu
                self.show_reasoning_submenu(stdout)?;
            }
            4 | 5 => {
                // Coming Soon items - do nothing
                self.show_menu = false;
                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
            }
            _ => {}
        }

        Ok(())
    }
}
```

**Step 6: Run cargo check**

```bash
cd synthia && cargo check
```

Expected: Compiles successfully (note: show_reasoning_submenu() is a stub, will implement next)

**Step 7: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "feat(ui): Add interactive menu with Ctrl+M shortcut and navigation"
```

---

## Task 6: Implement reasoning level submenu

**Files:**
- Modify: `synthia/src/ui/app.rs` (add reasoning level submenu)

**Step 1: Add submenu state to App struct**

```rust
// In synthia/src/ui/app.rs, App struct
pub struct App {
    // ... existing fields ...
    show_reasoning_submenu: bool,      // NEW
    reasoning_submenu_selected: usize, // NEW
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            // ... existing fields ...
            show_reasoning_submenu: false,     // NEW
            reasoning_submenu_selected: 1,     // NEW: default to "medium" (index 1)
        }
    }
```

**Step 2: Implement show_reasoning_submenu() method**

```rust
// In synthia/src/ui/app.rs
impl App {
    fn show_reasoning_submenu(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        self.show_menu = false;
        self.show_reasoning_submenu = true;
        self.reasoning_submenu_selected = 1; // Default to medium
        self.render_reasoning_submenu(stdout)
    }

    fn render_reasoning_submenu(&self, stdout: &mut impl Write) -> io::Result<()> {
        self.clear_input_line(stdout)?;

        writeln!(stdout, "\n=== Select Reasoning Level (↑/↓ navigate | Enter select | Esc cancel) ===")?;

        let levels = vec![
            ("Low", "Fast responses for general dialogue"),
            ("Medium", "Balanced speed and detail"),
            ("High", "Deep and detailed analysis"),
        ];

        for (idx, (level, desc)) in levels.iter().enumerate() {
            let selected = if idx == self.reasoning_submenu_selected { ">" } else { " " };

            if idx == self.reasoning_submenu_selected {
                queue!(stdout, SetForegroundColor(Color::Cyan))?;
            }

            writeln!(stdout, "{} {} - {}", selected, level, desc)?;

            if idx == self.reasoning_submenu_selected {
                queue!(stdout, ResetColor)?;
            }
        }

        writeln!(stdout)?;
        stdout.flush()
    }
}
```

**Step 3: Add reasoning submenu navigation in handle_input()**

```rust
// In synthia/src/ui/app.rs, in handle_input(), add after menu handling:
if self.show_reasoning_submenu {
    match key.code {
        KeyCode::Up => {
            if self.reasoning_submenu_selected > 0 {
                self.reasoning_submenu_selected -= 1;
                self.render_reasoning_submenu(stdout)?;
            }
            return Ok(());
        }
        KeyCode::Down => {
            if self.reasoning_submenu_selected < 2 {  // 3 options (0, 1, 2)
                self.reasoning_submenu_selected += 1;
                self.render_reasoning_submenu(stdout)?;
            }
            return Ok(());
        }
        KeyCode::Enter => {
            let level = match self.reasoning_submenu_selected {
                0 => "low",
                1 => "medium",
                2 => "high",
                _ => "medium",
            };

            self.cmd_tx.send(Command::SetReasoningLevel(level.to_string())).await?;
            self.show_reasoning_submenu = false;
            execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            self.print_header(stdout)?;

            print_colored_line(stdout, &format!("Reasoning level set to: {}", level), Color::Green)?;

            return Ok(());
        }
        KeyCode::Esc => {
            self.show_reasoning_submenu = false;
            self.show_menu = true;
            self.render_menu(stdout)?;
            return Ok(());
        }
        _ => return Ok(()),
    }
}
```

**Step 4: Run cargo check**

```bash
cd synthia && cargo check
```

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "feat(ui): Add reasoning level submenu with navigation"
```

---

## Task 7: Implement session name input modal

**Files:**
- Modify: `synthia/src/ui/app.rs` (add session name input modal)

**Step 1: Add session name input state to App struct**

```rust
// In synthia/src/ui/app.rs, App struct
pub struct App {
    // ... existing fields ...
    show_session_name_input: bool,  // NEW
    session_name_input: String,     // NEW: separate from main input
    session_name_cursor: usize,     // NEW
}

impl App {
    pub fn new(cmd_tx: Sender<Command>, ui_rx: Receiver<UIUpdate>) -> Self {
        Self {
            // ... existing fields ...
            show_session_name_input: false,   // NEW
            session_name_input: String::new(), // NEW
            session_name_cursor: 0,            // NEW
        }
    }
```

**Step 2: Update handle_menu_selection() for "Set Session Name"**

```rust
// In synthia/src/ui/app.rs, update case 0 in handle_menu_selection():
0 => {
    // Set Session Name
    self.show_menu = false;
    self.show_session_name_input = true;
    self.session_name_input.clear();
    self.session_name_cursor = 0;

    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    self.print_header(stdout)?;
    self.render_session_name_input(stdout)?;
}
```

**Step 3: Add render_session_name_input() method**

```rust
// In synthia/src/ui/app.rs
impl App {
    fn render_session_name_input(&self, stdout: &mut impl Write) -> io::Result<()> {
        execute!(stdout, Clear(ClearType::FromCursorDown))?;

        print_colored_line(stdout, "Enter session name (Enter to confirm, Esc to cancel):", Color::Yellow)?;

        queue!(
            stdout,
            SetForegroundColor(Color::Green),
            Print("Name: "),
            ResetColor,
            Print(&self.session_name_input),
        )?;

        // Position cursor
        let cursor_x = 6 + self.session_name_cursor; // "Name: " = 6 chars
        queue!(stdout, cursor::MoveTo(cursor_x as u16, 3))?;

        stdout.flush()
    }
}
```

**Step 4: Add session name input handling in handle_input()**

```rust
// In synthia/src/ui/app.rs, in handle_input(), add after reasoning submenu handling:
if self.show_session_name_input {
    match (key.code, key.modifiers) {
        (KeyCode::Enter, _) => {
            if !self.session_name_input.is_empty() {
                self.cmd_tx.send(Command::SetSessionName(self.session_name_input.clone())).await?;

                execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                self.print_header(stdout)?;
                print_colored_line(stdout, &format!("Session name set to: {}", self.session_name_input), Color::Green)?;
            }

            self.show_session_name_input = false;
            self.session_name_input.clear();
            self.session_name_cursor = 0;
            return Ok(());
        }
        (KeyCode::Esc, _) => {
            self.show_session_name_input = false;
            self.session_name_input.clear();
            self.session_name_cursor = 0;

            execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            self.print_header(stdout)?;
            return Ok(());
        }
        (KeyCode::Char(c), _) => {
            self.session_name_input.push(c);
            self.session_name_cursor += 1;
            self.render_session_name_input(stdout)?;
            return Ok(());
        }
        (KeyCode::Backspace, _) => {
            if self.session_name_cursor > 0 {
                self.session_name_cursor -= 1;
                self.session_name_input.pop();
                self.render_session_name_input(stdout)?;
            }
            return Ok(());
        }
        (KeyCode::Left, _) => {
            if self.session_name_cursor > 0 {
                self.session_name_cursor -= 1;
                self.render_session_name_input(stdout)?;
            }
            return Ok(());
        }
        (KeyCode::Right, _) => {
            if self.session_name_cursor < self.session_name_input.len() {
                self.session_name_cursor += 1;
                self.render_session_name_input(stdout)?;
            }
            return Ok(());
        }
        _ => return Ok(()),
    }
}
```

**Step 5: Run cargo check**

```bash
cd synthia && cargo check
```

Expected: Compiles successfully

**Step 6: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "feat(ui): Add session name input modal with basic editing"
```

---

## Task 8: Display session name in session list

**Files:**
- Modify: `synthia/src/ui/app.rs` (update render_session_list to show names)

**Step 1: Update render_session_list() to display session names**

```rust
// In synthia/src/ui/app.rs, in render_session_list() method
// Replace the writeln! that displays session info:

let display_name = session.name.as_ref()
    .map(|n| format!("{} ({})", n, &session.id[..session.id.len().min(10)]))
    .unwrap_or_else(|| session.id[..session.id.len().min(30)].to_string());

writeln!(
    stdout,
    "{} {} - {} msgs - {}",
    selected,
    timestamp,
    session.message_count,
    display_name
)?;
```

**Step 2: Build and test manually**

```bash
cd synthia && cargo build
# Run and test: press Ctrl+M, navigate to "Set Session Name", enter a name, then Ctrl+L to see session list
```

Expected: Session list shows friendly name if set, otherwise shows ID

**Step 3: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "feat(ui): Display session names in session list"
```

---

## Task 9: Update session list rendering to prevent flag blocking

**Files:**
- Modify: `synthia/src/ui/app.rs` (fix modal flag checks in render loop)

**Step 1: Update render_input_line() condition to check all modal flags**

```rust
// In synthia/src/ui/app.rs, in run() method, update the condition:
// Replace:
if !self.show_session_list && self.input_needs_render {

// With:
if !self.show_session_list
    && !self.show_menu
    && !self.show_reasoning_submenu
    && !self.show_session_name_input
    && self.input_needs_render {
```

**Step 2: Run cargo check**

```bash
cd synthia && cargo check
```

Expected: Compiles successfully

**Step 3: Build and test**

```bash
cd synthia && cargo build
# Test all modals: Ctrl+M menu, reasoning submenu, session name input
```

Expected: No input line rendering conflicts

**Step 4: Commit**

```bash
git add synthia/src/ui/app.rs
git commit -m "fix(ui): Prevent input line rendering during modal displays"
```

---

## Task 10: Add AI-generated session names for first message (future enhancement placeholder)

**Note:** This task is marked as a future enhancement. For now, we'll document where to implement it.

**Files:**
- Future: `synthia/src/agent/actor.rs` (after first user message, generate name)

**Implementation notes for future work:**

```rust
// In AgentActor::handle_message(), after first message:
// 1. Check if session.messages.len() == 1 (first message)
// 2. Check if session.name.is_none()
// 3. Send request to LLM: "Generate a short 2-4 word summary of this request: {first_message}"
// 4. Parse response and call session.set_name()
// 5. Save session
```

**Step 1: Add a TODO comment in actor.rs**

```rust
// In synthia/src/agent/actor.rs, in handle_message() method, add:
// TODO: Generate AI session name from first message
// if self.current_session.as_ref().map(|s| s.messages.len() == 1 && s.name.is_none()).unwrap_or(false) {
//     // Generate short summary via LLM
//     // session.set_name(summary)
// }
```

**Step 2: Commit**

```bash
git add synthia/src/agent/actor.rs
git commit -m "docs(agent): Add TODO for AI-generated session names"
```

---

## Success Criteria

**After completing all tasks:**

1. ✅ Ctrl+M opens interactive menu with 6 items
2. ✅ Menu navigation works (↑/↓, Enter, Esc)
3. ✅ "Set Session Name" prompts for input and saves to session
4. ✅ "Save Session" triggers session save
5. ✅ "New Session" creates new session
6. ✅ "Set Reasoning Level" shows submenu with Low/Medium/High options
7. ✅ Reasoning level gets injected into system prompt
8. ✅ Session list displays friendly names when set
9. ✅ All existing tests still pass
10. ✅ No input rendering conflicts with modals

**Future work (Phase 2):**
- AI-generated session names from first message
- JSONL logging to `~/.synthia/projects/<project_name>/`
- Context compaction at 80% usage
- Planning vs Execution mode toggle

---

## Testing Notes

**Manual testing checklist:**
- [ ] Open menu with Ctrl+M
- [ ] Navigate menu with arrow keys
- [ ] Set session name (enter text, confirm with Enter)
- [ ] Verify session name appears in session list (Ctrl+L)
- [ ] Set reasoning level to "high"
- [ ] Send a message and check /tmp/synthia.log for "Reasoning: high"
- [ ] Test Esc key to cancel all modals
- [ ] Verify existing shortcuts still work (Ctrl+S, Ctrl+N, Ctrl+L, Ctrl+C, Ctrl+D)
