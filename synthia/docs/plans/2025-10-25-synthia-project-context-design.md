# .synthia/ Project Context Design

**Date:** 2025-10-25
**Status:** Approved
**Author:** Design validated through brainstorming session

## Overview

Add support for project-level configuration and custom instructions via a `.synthia/` directory, mirroring Claude Code's `.claude/` pattern. The primary feature is `.SYNTHIA.md` - a markdown file that injects project-specific instructions into the system prompt.

## Goals

1. Enable project-specific instructions similar to Claude Code's `.claude/CLAUDE.md`
2. Provide a dedicated directory for future project-level features
3. Maintain clean separation between system instructions and project instructions
4. Make the feature discoverable and user-friendly

## Non-Goals

- Hot-reloading of `.SYNTHIA.md` (changes require restart)
- Template generation with examples (start with empty file)
- .gitignore management (users decide what to commit)

## Design Decisions

### 1. Integration Strategy: Separate System Message

**Decision:** Send .SYNTHIA.md content as a second system message after the core Synthia prompt.

**Alternatives Considered:**
- Append/prepend to existing system prompt: Mixes concerns, harder to manage
- Replace system prompt: Too risky, users might break essential instructions

**Rationale:** Clean separation allows project instructions to augment (not replace) core behavior. Follows multi-message pattern already used in conversation flow.

### 2. Directory Creation: Auto-create on Startup

**Decision:** Automatically create `.synthia/` and empty `.SYNTHIA.md` on first run in any directory.

**Alternatives Considered:**
- Manual creation only: Less user-friendly, requires documentation
- `synthia init` command: Extra step, less discoverable

**Rationale:** User-friendly, self-documenting. Users discover the file exists and can choose to populate it or ignore it.

### 3. File Reading: Startup Only

**Decision:** Read `.SYNTHIA.md` once at startup. Changes require restart.

**Alternatives Considered:**
- File watcher with hot-reload: Complex, mid-conversation context changes are confusing
- Reload command: Added complexity for marginal benefit

**Rationale:** Matches Claude Code behavior. Predictable, simple implementation. Restarting Synthia is fast enough.

### 4. Architecture: New project_context Module

**Decision:** Create `src/project_context.rs` to encapsulate all project-level logic.

**Alternatives Considered:**
- Inline in main.rs: Clutters entry point, poor separation of concerns
- Extend Config system: Conflates settings (TOML) with content (Markdown)

**Rationale:** Clean separation, extensible for future features (tools, memory, templates), follows single-responsibility principle.

## Architecture

### Module Structure

```
synthia/src/
├── main.rs                   # Calls ProjectContext::load(), passes to AgentActor
├── project_context.rs        # NEW: Handles .synthia/ setup and reading
├── agent/
│   └── actor.rs              # Modified: Accepts project_context, creates second system message
└── config.rs                 # Unchanged: Still handles config.toml
```

### Data Flow

```
Startup Sequence:
1. main.rs: init_tracing()
2. main.rs: Config::load()
3. main.rs: ProjectContext::load()              ← NEW
   ├─ Ensure .synthia/ exists
   ├─ Ensure .synthia/.SYNTHIA.md exists
   └─ Read file contents
4. main.rs: Create LLM provider
5. main.rs: Create tool registry
6. main.rs: AgentActor::new(..., project_context)  ← MODIFIED
   └─ Creates conversation: [SystemMsg1, SystemMsg2 (if present)]
7. main.rs: Spawn agent actor
8. main.rs: Run TUI

Message Flow:
[System: "You are Synthia..."]
[System: "<project-instructions>...</project-instructions>"]  ← NEW (if .SYNTHIA.md non-empty)
[User: "Hello"]
[Assistant: "..."]
```

## Implementation Details

### 1. New Module: src/project_context.rs

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

### 2. Modified: src/main.rs

```rust
mod project_context;
use project_context::ProjectContext;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;
    let config = Config::load();

    // NEW: Load project context
    let project_context = ProjectContext::load();
    if let Some(ref instructions) = project_context.custom_instructions {
        info!("Loaded project-specific instructions from .synthia/.SYNTHIA.md ({} bytes)", instructions.len());
    }

    let llm_provider = Arc::new(OpenAICompatibleProvider::new(
        config.llm.api_base.clone(),
        config.llm.api_key.clone(),
        config.llm.model.clone(),
    ));

    let tool_registry = Arc::new(ToolRegistry::new());
    // ... register tools ...

    let (command_tx, command_rx) = mpsc::channel(32);
    let (ui_tx, ui_rx) = mpsc::channel(32);

    // MODIFIED: Pass project_context to AgentActor
    let agent = AgentActor::new(
        llm_provider,
        tool_registry,
        ui_tx.clone(),
        config.clone(),
        project_context.custom_instructions,
    );
    tokio::spawn(agent.run(command_rx));

    let mut app = App::new(command_tx, ui_rx, config);
    app.run().await?;

    Ok(())
}
```

### 3. Modified: src/agent/actor.rs

**Add field to struct:**
```rust
pub struct AgentActor {
    conversation: Vec<ChatMessage>,
    project_context: Option<String>,  // NEW
    llm_provider: Arc<dyn ChatCompletionProvider>,
    tool_registry: Arc<ToolRegistry>,
    // ... other fields ...
}
```

**Modify constructor:**
```rust
pub fn new(
    llm_provider: Arc<dyn ChatCompletionProvider>,
    tool_registry: Arc<ToolRegistry>,
    ui_tx: mpsc::Sender<UIUpdate>,
    config: Config,
    project_context: Option<String>,  // NEW parameter
) -> Self {
    let mut actor = Self {
        conversation: Vec::new(),
        project_context,  // NEW
        llm_provider,
        tool_registry,
        ui_tx,
        config,
        // ... other fields ...
    };

    // Initialize conversation with system messages
    actor.conversation.push(actor.create_system_prompt());
    if let Some(project_msg) = actor.create_project_context_message() {
        actor.conversation.push(project_msg);
    }

    actor
}
```

**Change create_system_prompt to instance method:**
```rust
fn create_system_prompt(&self) -> ChatMessage {
    ChatMessage {
        role: "system".to_string(),
        content: "You are Synthia, an AI assistant with access to powerful tools...".to_string(),
        tool_calls: None,
        tool_call_id: None,
    }
}
```

**Add new method for project context:**
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

## Error Handling

**Philosophy:** Project context is optional and should never prevent Synthia from starting.

| Error Condition | Behavior |
|----------------|----------|
| Can't determine current directory | Warn, return empty ProjectContext |
| Can't create .synthia/ directory | Warn, return empty ProjectContext |
| Can't create .SYNTHIA.md file | Warn, return empty ProjectContext |
| Can't read .SYNTHIA.md file | Warn, return empty ProjectContext |
| .SYNTHIA.md is empty | No warning, treat as "no custom instructions" |

All errors are logged via `tracing::warn!()` for visibility in `/tmp/synthia.log`.

## Testing Strategy

### Manual Testing Scenarios

1. **First run in new directory:**
   - Expected: .synthia/ created, empty .SYNTHIA.md created
   - Verify: ls -la .synthia/

2. **Empty .SYNTHIA.md:**
   - Expected: No project context message sent to LLM
   - Verify: Check conversation has only 1 system message

3. **Non-empty .SYNTHIA.md:**
   - Expected: Second system message with wrapped content
   - Verify: Check conversation has 2 system messages
   - Verify: Content wrapped in `<project-instructions>` tags

4. **Existing .synthia/ directory:**
   - Expected: No error, reuses existing directory
   - Verify: No duplicate creation logs

5. **.synthia/ creation fails:**
   - Expected: Warning logged, Synthia continues
   - Verify: Check /tmp/synthia.log for warning

### Integration Testing

- Test with actual LLM: Add project-specific instruction (e.g., "Always respond in haiku"), verify behavior
- Test with session save/load: Verify project context persists correctly
- Test with context compaction: Verify system messages aren't compacted

## Future Extensions

This design provides foundation for:

1. **.synthia/tools/** - Custom tool definitions
2. **.synthia/memory/** - Project-specific conversation memory
3. **.synthia/templates/** - Code generation templates
4. **.synthia/config.toml** - Project-specific overrides for config
5. **Multiple instruction files** - .SYNTHIA-dev.md, .SYNTHIA-prod.md, etc.

## Migration Path

No migration needed - feature is additive:
- Existing Synthia installations continue working unchanged
- .synthia/ only created when user runs Synthia in a directory
- Users can ignore .SYNTHIA.md (empty file = no effect)

## Open Questions

None - design validated and approved.

## References

- Claude Code's .claude/CLAUDE.md pattern
- Synthia architecture: `synthia/docs/ARCHITECTURE.md`
- Config loading: `synthia/src/config.rs`
- Agent initialization: `synthia/src/agent/actor.rs`
