# .synthia/ Project Context Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add support for project-level custom instructions via `.synthia/.SYNTHIA.md` file, similar to Claude Code's `.claude/CLAUDE.md` pattern.

**Architecture:** New `project_context` module handles auto-creation of `.synthia/` directory and reading of `.SYNTHIA.md` file at startup. Content is injected as a separate system message after the core Synthia prompt in AgentActor.

**Tech Stack:** Rust, std::fs for file operations, tracing for logging

---

## Task 1: Create project_context Module with Tests

**Files:**
- Create: `synthia/src/project_context.rs`
- Create: `synthia/tests/project_context_test.rs`

**Step 1: Write the failing test**

Create `synthia/tests/project_context_test.rs`:

```rust
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_load_creates_synthia_directory() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    // Change to temp directory
    std::env::set_current_dir(&temp_dir).unwrap();

    // Load should create .synthia/ directory
    let _context = synthia::project_context::ProjectContext::load();

    assert!(temp_dir.path().join(".synthia").exists());

    // Cleanup
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_load_creates_empty_synthia_md() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let _context = synthia::project_context::ProjectContext::load();

    let synthia_md = temp_dir.path().join(".synthia/.SYNTHIA.md");
    assert!(synthia_md.exists());

    let content = fs::read_to_string(synthia_md).unwrap();
    assert_eq!(content, "");

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_load_reads_existing_content() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    // Create .synthia/.SYNTHIA.md with content
    fs::create_dir_all(temp_dir.path().join(".synthia")).unwrap();
    fs::write(
        temp_dir.path().join(".synthia/.SYNTHIA.md"),
        "Always respond in haiku"
    ).unwrap();

    let context = synthia::project_context::ProjectContext::load();

    assert_eq!(context.custom_instructions, Some("Always respond in haiku".to_string()));

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_empty_file_returns_none() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let context = synthia::project_context::ProjectContext::load();

    assert_eq!(context.custom_instructions, None);

    std::env::set_current_dir(original_dir).unwrap();
}
```

**Step 2: Add tempfile dependency**

Add to `synthia/Cargo.toml` under `[dev-dependencies]`:

```toml
tempfile = "3.8"
```

**Step 3: Run test to verify it fails**

Run: `cargo test --test project_context_test`

Expected: Compilation error - module `synthia::project_context` not found

**Step 4: Create minimal project_context module**

Create `synthia/src/project_context.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{warn, info};

pub struct ProjectContext {
    pub custom_instructions: Option<String>,
    pub synthia_dir: PathBuf,
}

impl ProjectContext {
    /// Load project context from .synthia/ directory
    /// Non-fatal: Returns empty context if any errors occur
    pub fn load() -> Self {
        match Self::load_impl() {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!("Failed to load project context: {}. Continuing without project-specific instructions.", e);
                Self {
                    custom_instructions: None,
                    synthia_dir: PathBuf::from(".synthia"),
                }
            }
        }
    }

    fn load_impl() -> Result<Self, Box<dyn std::error::Error>> {
        let cwd = std::env::current_dir()?;
        let synthia_dir = cwd.join(".synthia");

        // Ensure .synthia/ exists
        if !synthia_dir.exists() {
            fs::create_dir_all(&synthia_dir)?;
            info!("Created .synthia directory at {:?}", synthia_dir);
        }

        // Ensure .SYNTHIA.md exists
        let synthia_md = synthia_dir.join(".SYNTHIA.md");
        if !synthia_md.exists() {
            fs::write(&synthia_md, "")?;
            info!("Created empty .SYNTHIA.md at {:?}", synthia_md);
        }

        // Read custom instructions
        let custom_instructions = Self::load_custom_instructions(&synthia_md)?;

        Ok(Self {
            custom_instructions,
            synthia_dir,
        })
    }

    fn load_custom_instructions(path: &Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let trimmed = content.trim();

        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }
}
```

**Step 5: Declare module in lib.rs**

Add to `synthia/src/lib.rs` (create if doesn't exist):

```rust
pub mod project_context;
```

If `synthia/src/lib.rs` doesn't exist, create it with:

```rust
pub mod project_context;
```

**Step 6: Run test to verify it passes**

Run: `cargo test --test project_context_test`

Expected: All 4 tests PASS

**Step 7: Commit**

```bash
git add synthia/src/lib.rs synthia/src/project_context.rs synthia/tests/project_context_test.rs synthia/Cargo.toml
git commit -m "feat(synthia): Add project_context module with .synthia/ directory support

- Auto-creates .synthia/ directory on startup
- Auto-creates empty .SYNTHIA.md if missing
- Reads custom instructions from .SYNTHIA.md
- Returns None for empty files
- Non-fatal error handling with logging

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Modify AgentActor to Accept Project Context

**Files:**
- Modify: `synthia/src/agent/actor.rs`
- Create: `synthia/tests/agent_actor_project_context_test.rs`

**Step 1: Write the failing test**

Create `synthia/tests/agent_actor_project_context_test.rs`:

```rust
use synthia::agent::actor::AgentActor;
use synthia::agent::chat_message::ChatMessage;
use synthia::config::Config;
use synthia::llm::openai::OpenAICompatibleProvider;
use synthia::tools::registry::ToolRegistry;
use tokio::sync::mpsc;
use std::sync::Arc;

#[tokio::test]
async fn test_actor_with_no_project_context() {
    let config = Config::default();
    let llm_provider = Arc::new(OpenAICompatibleProvider::new(
        "http://localhost:1234/v1".to_string(),
        "test-key".to_string(),
        "test-model".to_string(),
    ));
    let tool_registry = Arc::new(ToolRegistry::new());
    let (ui_tx, _ui_rx) = mpsc::channel(32);

    let actor = AgentActor::new(
        llm_provider,
        tool_registry,
        ui_tx,
        config,
        None, // No project context
    );

    // Should have only 1 system message (core prompt)
    assert_eq!(actor.conversation().len(), 1);
    assert_eq!(actor.conversation()[0].role, "system");
}

#[tokio::test]
async fn test_actor_with_project_context() {
    let config = Config::default();
    let llm_provider = Arc::new(OpenAICompatibleProvider::new(
        "http://localhost:1234/v1".to_string(),
        "test-key".to_string(),
        "test-model".to_string(),
    ));
    let tool_registry = Arc::new(ToolRegistry::new());
    let (ui_tx, _ui_rx) = mpsc::channel(32);

    let project_context = Some("Always respond in haiku".to_string());

    let actor = AgentActor::new(
        llm_provider,
        tool_registry,
        ui_tx,
        config,
        project_context,
    );

    // Should have 2 system messages
    assert_eq!(actor.conversation().len(), 2);
    assert_eq!(actor.conversation()[0].role, "system");
    assert_eq!(actor.conversation()[1].role, "system");

    // Second message should contain project instructions wrapped
    assert!(actor.conversation()[1].content.contains("<project-instructions>"));
    assert!(actor.conversation()[1].content.contains("Always respond in haiku"));
    assert!(actor.conversation()[1].content.contains("</project-instructions>"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test agent_actor_project_context_test`

Expected: Compilation error - AgentActor::new signature doesn't match (missing project_context parameter)

**Step 3: Modify AgentActor struct**

In `synthia/src/agent/actor.rs`, find the `AgentActor` struct definition (around line 15) and add the field:

```rust
pub struct AgentActor {
    conversation: Vec<ChatMessage>,
    project_context: Option<String>,  // ADD THIS LINE
    llm_provider: Arc<dyn ChatCompletionProvider>,
    tool_registry: Arc<ToolRegistry>,
    ui_tx: mpsc::Sender<UIUpdate>,
    config: Config,
    // ... rest of fields
}
```

**Step 4: Modify AgentActor::new signature**

In `synthia/src/agent/actor.rs`, find the `new` function (around line 78) and update:

```rust
pub fn new(
    llm_provider: Arc<dyn ChatCompletionProvider>,
    tool_registry: Arc<ToolRegistry>,
    ui_tx: mpsc::Sender<UIUpdate>,
    config: Config,
    project_context: Option<String>,  // ADD THIS PARAMETER
) -> Self {
    let context_manager = ContextManager::new(config.llm.context_window);

    let mut actor = Self {
        conversation: Vec::new(),
        project_context,  // ADD THIS FIELD INITIALIZATION
        llm_provider,
        tool_registry,
        ui_tx,
        config,
        context_manager,
        session_id: None,
    };

    // Initialize conversation with system messages
    actor.conversation.push(actor.create_system_prompt());
    if let Some(project_msg) = actor.create_project_context_message() {
        actor.conversation.push(project_msg);
    }

    // Add system messages to context manager
    for msg in &actor.conversation {
        actor.context_manager.add_message(msg.clone());
    }

    actor
}
```

**Step 5: Change create_system_prompt to instance method**

In `synthia/src/agent/actor.rs`, find the `create_system_prompt` function (around line 37) and change from associated function to instance method:

Change:
```rust
fn create_system_prompt() -> ChatMessage {
```

To:
```rust
fn create_system_prompt(&self) -> ChatMessage {
```

**Step 6: Add create_project_context_message method**

In `synthia/src/agent/actor.rs`, add this new method after `create_system_prompt`:

```rust
fn create_project_context_message(&self) -> Option<ChatMessage> {
    self.project_context.as_ref().map(|content| {
        ChatMessage {
            role: "system".to_string(),
            content: format!("<project-instructions>\n{}\n</project-instructions>", content),
            tool_calls: None,
            tool_call_id: None,
        }
    })
}
```

**Step 7: Add conversation getter for tests**

In `synthia/src/agent/actor.rs`, add a public method to access conversation:

```rust
pub fn conversation(&self) -> &[ChatMessage] {
    &self.conversation
}
```

**Step 8: Run test to verify it passes**

Run: `cargo test --test agent_actor_project_context_test`

Expected: Both tests PASS

**Step 9: Commit**

```bash
git add synthia/src/agent/actor.rs synthia/tests/agent_actor_project_context_test.rs
git commit -m "feat(synthia): Add project context support to AgentActor

- Accept optional project_context parameter in new()
- Create separate system message for project instructions
- Wrap content in <project-instructions> tags
- Add conversation() getter for testing

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Integrate project_context in main.rs

**Files:**
- Modify: `synthia/src/main.rs`

**Step 1: Add module import**

In `synthia/src/main.rs`, add to the imports section (around line 10):

```rust
mod project_context;
use project_context::ProjectContext;
```

**Step 2: Load project context after config**

In `synthia/src/main.rs`, in the `main()` function, after `let config = Config::load();` (around line 35), add:

```rust
// Load project context
let project_context = ProjectContext::load();
if let Some(ref instructions) = project_context.custom_instructions {
    info!("Loaded project-specific instructions from .synthia/.SYNTHIA.md ({} bytes)", instructions.len());
}
```

**Step 3: Pass project context to AgentActor**

In `synthia/src/main.rs`, find the `AgentActor::new()` call (around line 60) and add the parameter:

Change:
```rust
let agent = AgentActor::new(
    llm_provider,
    tool_registry,
    ui_tx.clone(),
    config.clone(),
);
```

To:
```rust
let agent = AgentActor::new(
    llm_provider,
    tool_registry,
    ui_tx.clone(),
    config.clone(),
    project_context.custom_instructions,
);
```

**Step 4: Build to verify compilation**

Run: `cargo build --release`

Expected: Successful build with warnings

**Step 5: Commit**

```bash
git add synthia/src/main.rs
git commit -m "feat(synthia): Integrate project context loading in main

- Load ProjectContext after Config
- Log when custom instructions are loaded
- Pass to AgentActor on initialization

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Manual Integration Testing

**Files:**
- None (manual testing)

**Step 1: Build the binary**

Run: `cargo build --release`

Expected: Successful build

**Step 2: Create test project directory**

Run:
```bash
mkdir -p /tmp/synthia-test-project
cd /tmp/synthia-test-project
```

Expected: Directory created

**Step 3: Run Synthia to trigger auto-creation**

Run:
```bash
/Users/zachswift/projects/agent-power-tools/.worktrees/synthia-project-context/target/release/synthia
```

Then immediately exit (Ctrl+C or quit command)

Expected: `.synthia/` directory and empty `.SYNTHIA.md` file created

**Step 4: Verify directory structure**

Run:
```bash
ls -la .synthia/
cat .synthia/.SYNTHIA.md
```

Expected:
```
.synthia/
.synthia/.SYNTHIA.md (empty file)
```

**Step 5: Add custom instructions**

Run:
```bash
echo "You are a helpful assistant who always speaks like a pirate." > .synthia/.SYNTHIA.md
```

Expected: File updated

**Step 6: Restart Synthia and test**

Run Synthia again, then send a test message:
```
> Hello, can you help me?
```

Expected: Response should be in pirate style (if LLM is running and responding)

**Step 7: Check logs**

Run:
```bash
grep "project-specific instructions" /tmp/synthia.log | tail -1
```

Expected: Log message showing bytes loaded from .SYNTHIA.md

**Step 8: Document test results**

Create a note of test results (no commit needed, just verification)

---

## Task 5: Update Documentation

**Files:**
- Create: `synthia/docs/SYNTHIA_MD_GUIDE.md`
- Modify: `synthia/README.md`

**Step 1: Create user guide**

Create `synthia/docs/SYNTHIA_MD_GUIDE.md`:

```markdown
# Using .SYNTHIA.md for Project-Specific Instructions

## Overview

Synthia supports project-level custom instructions via `.synthia/.SYNTHIA.md`, similar to Claude Code's `.claude/CLAUDE.md` pattern.

## How It Works

1. **Auto-creation:** When you start Synthia in a directory, it automatically creates:
   - `.synthia/` directory
   - `.synthia/.SYNTHIA.md` (empty file)

2. **Reading:** Synthia reads `.SYNTHIA.md` at startup and injects the content as a system message

3. **Updates:** Changes to `.SYNTHIA.md` require restarting Synthia to take effect

## Usage

### Basic Example

Add custom instructions to `.synthia/.SYNTHIA.md`:

```markdown
You are a helpful coding assistant working on a Python web application.

Project context:
- Using FastAPI framework
- PostgreSQL database
- Following PEP 8 style guide
- All API responses should include error handling
```

### What to Include

- **Project conventions:** Coding style, naming patterns, architecture rules
- **Context:** What the project does, key technologies used
- **Constraints:** Requirements, limitations, gotchas
- **Preferences:** Response format, level of detail, examples vs explanations

### Example: API Project

```markdown
This is a REST API for a task management system.

Stack:
- FastAPI (Python 3.11)
- PostgreSQL with SQLAlchemy ORM
- Pydantic for validation
- pytest for testing

Guidelines:
- All endpoints must have OpenAPI docs
- Use dependency injection for database sessions
- Write tests for every new endpoint
- Follow RESTful conventions (GET/POST/PUT/DELETE)
```

### Example: Code Review Focus

```markdown
When reviewing code, prioritize:
1. Security vulnerabilities
2. Performance issues
3. Code duplication
4. Missing error handling
5. Unclear variable names

Be direct and specific. Suggest fixes, don't just point out problems.
```

## Best Practices

- **Be specific:** Vague instructions get vague results
- **Keep it concise:** Focus on what's unique to your project
- **Update regularly:** Add new conventions as the project evolves
- **Commit it:** Share instructions with your team via git

## .gitignore

The `.synthia/` directory can be:
- **Committed:** Share instructions with team
- **Ignored:** Keep instructions personal

Add to `.gitignore` to ignore:
```
.synthia/
```

## Troubleshooting

**Instructions not working?**
- Restart Synthia (changes only loaded at startup)
- Check `/tmp/synthia.log` for "Loaded project-specific instructions" message
- Verify `.SYNTHIA.md` is not empty (empty files are ignored)

**Directory not created?**
- Verify you have write permissions in current directory
- Check `/tmp/synthia.log` for errors
```

**Step 2: Update main README**

Add to `synthia/README.md` in the Features section (find the Features heading and add this bullet):

```markdown
- **Project-Specific Instructions:** Customize Synthia's behavior per-project via `.synthia/.SYNTHIA.md`
```

Add a new section before "Contributing":

```markdown
## Project-Specific Instructions

Synthia supports project-level custom instructions via `.synthia/.SYNTHIA.md`. This allows you to:

- Define project-specific conventions and guidelines
- Provide context about your codebase
- Customize Synthia's behavior for different projects

See [.SYNTHIA.md Guide](docs/SYNTHIA_MD_GUIDE.md) for details.

Quick example:

```bash
# Synthia auto-creates .synthia/.SYNTHIA.md on startup
# Add your custom instructions:
echo "You are helping with a FastAPI project. Follow PEP 8." > .synthia/.SYNTHIA.md

# Restart Synthia to load the instructions
synthia
```
```

**Step 3: Build and verify links work**

Run: `cargo build --release`

Expected: Successful build

**Step 4: Commit**

```bash
git add synthia/docs/SYNTHIA_MD_GUIDE.md synthia/README.md
git commit -m "docs(synthia): Add .SYNTHIA.md user guide and README updates

- Comprehensive guide for using project-specific instructions
- Examples for different use cases
- Best practices and troubleshooting
- Update README with feature description

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Final Verification and Cleanup

**Files:**
- None (verification only)

**Step 1: Run all Synthia tests**

Run: `cargo test -p synthia`

Expected: All tests PASS

**Step 2: Build release binary**

Run: `cargo build --release`

Expected: Successful build

**Step 3: Verify binary works**

Run in a test directory:
```bash
cd /tmp/test-synthia
/Users/zachswift/projects/agent-power-tools/.worktrees/synthia-project-context/target/release/synthia
```

Expected: Synthia starts, creates `.synthia/.SYNTHIA.md`

**Step 4: Check git status**

Run: `git status`

Expected: No uncommitted changes (all work committed)

**Step 5: Review commit history**

Run: `git log --oneline -6`

Expected: 6 commits for this feature

**Step 6: Push branch (optional)**

If ready to push:
```bash
git push -u origin feature/synthia-project-context
```

---

## Summary

**Total Tasks:** 6
**Estimated Time:** 60-90 minutes
**Test Coverage:** Unit tests + integration tests + manual testing
**Commits:** 5 feature commits + 1 docs commit

**Key Files Modified:**
- Created: `synthia/src/project_context.rs`
- Created: `synthia/src/lib.rs` (if didn't exist)
- Modified: `synthia/src/agent/actor.rs`
- Modified: `synthia/src/main.rs`
- Created: `synthia/docs/SYNTHIA_MD_GUIDE.md`
- Modified: `synthia/README.md`
- Created: 2 test files

**Next Steps:**
1. Create pull request from feature branch
2. Manual QA testing with real LLM
3. Get code review
4. Merge to main
