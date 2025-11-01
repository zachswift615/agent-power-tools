# Permission System Design

**Date:** 2025-11-01
**Status:** Design Complete
**Author:** Design session with user

## Overview

Add a persistent permission system to Synthia that allows users to approve tool executions once and optionally save their decision to avoid future prompts. The system supports two distinct approval flows: file edit previews with diffs, and command execution approvals for non-file operations.

## Goals

1. **Reduce prompt fatigue**: Allow users to approve entire categories of operations (e.g., "all cargo commands", "all reads from /Users/zachswift/**")
2. **Maintain safety**: Keep informational diffs visible even for auto-approved file edits
3. **User control**: Persist permissions in project-specific config files
4. **Minimal disruption**: Extend existing approval flow rather than replace it

## Architecture

### Component Structure

Three new modules in the Synthia codebase:

1. **`permission_config.rs`** - Data structures and file I/O
2. **`permission_manager.rs`** - Permission checking logic and pattern matching
3. **Integration changes** - Modify `main.rs`, `registry.rs`, and `app.rs`

### Data Flow

```
Agent calls tool
    ↓
Registry checks PermissionManager
    ↓
┌───────────────────────────────────────┐
│ Permission Decision:                  │
│  • Allow → Execute (show info diff)   │
│  • Deny → Reject                      │
│  • Ask → Show approval prompt         │
└───────────────────────────────────────┘
    ↓
User responds (if prompted)
    ↓
If "don't ask again" → Save to config
    ↓
Execute or cancel
```

## Data Structures

### Permission Configuration

**File:** `.synthia/settings-local.json` (project root)

**Format:**
```json
{
  "permissions": {
    "allow": [
      "Bash(cargo:*)",
      "Bash(/full/path/to/script.sh)",
      "Read(//Users/zachswift/**)",
      "Read(/relative/path/**)",
      "Write(/absolute/path/to/file.rs)",
      "Edit(//Users/zachswift/projects/foo/src/main.rs)",
      "Git(commit:*)",
      "WebFetch(domain:github.com)",
      "mcp__powertools__index_project"
    ],
    "deny": [],
    "ask": []
  }
}
```

**Pattern Types:**

- `Bash(command-name:*)` - Allow bash command with any arguments
- `Bash(/full/path/to/script.sh)` - Allow specific script
- `Read(//absolute/path/**)` - Absolute path (double slash prefix)
- `Read(/relative/path/**)` - Relative to project root
- `Write(pattern)`, `Edit(pattern)` - Same glob system
- `WebFetch(domain:example.com)` - Domain-based web fetch
- `mcp__toolname__method` - MCP server tools

**Rust Structures:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub permissions: Permissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub ask: Vec<String>,
}
```

## Permission Manager

### Core API

```rust
pub struct PermissionManager {
    config: PermissionConfig,
    config_path: PathBuf,
    project_root: PathBuf,
}

pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

impl PermissionManager {
    pub fn new(project_root: PathBuf) -> Result<Self>;
    pub fn check_permission(&self, tool: &str, params: &Value) -> PermissionDecision;
    pub fn add_permission(&mut self, pattern: String) -> Result<()>;
}
```

### Permission Checking Logic

1. **Check deny list first** - If pattern matches → reject immediately (security)
2. **Check allow list** - If pattern matches → allow immediately
3. **Default to Ask** - Prompt user for decision

### Pattern Construction

When user selects "don't ask again", construct pattern based on tool:

| Tool | Input Example | Generated Pattern | Scope |
|------|--------------|-------------------|-------|
| Bash | `cargo test --all` | `Bash(cargo:*)` | All cargo commands |
| Read | `/Users/zachswift/projects/foo/src/main.rs` | `Read(//Users/zachswift/projects/foo/**)` | All files in directory tree |
| Write/Edit (per-file) | `/Users/zachswift/projects/foo/src/main.rs` | `Edit(//Users/zachswift/projects/foo/src/main.rs)` | Exact file only |
| Git | `git commit -m "message"` | `Git(commit:*)` | All commit commands |
| WebFetch | `https://api.github.com/...` | `WebFetch(domain:github.com)` | Domain-scoped |
| MCP | `mcp__powertools__index_project` | `mcp__powertools__index_project` | Exact tool |

### Glob Matching

- Use `glob` crate for pattern matching
- `**` matches any number of directories
- `*` matches any characters within a path segment
- Normalize paths before comparison (resolve `.`, `..`, symlinks)

## User Interface

### Two Distinct Approval Flows

#### 1. File Edit Flow (Edit/Write Tools)

**Not in allow list:**
```
┌─ Edit Preview ────────────────┐
│ File: path/to/file.rs         │
│ Changes: +5 -3                │
│                               │
│ [diff display with context]   │
│                               │
└───────────────────────────────┘

[A]ccept  [D]on't ask for this file  [R]eject
```

**In allow list (auto-approved):**
- Show diff in conversation output (informational, non-blocking)
- Execute immediately
- User sees changes but doesn't need to approve

#### 2. Command Execution Flow (Bash/Git/Read/WebFetch/etc)

**Not in allow list:**
```
┌─ Permission Required ─────────────────────────────────────┐
│ Tool: bash                                                 │
│ Command: cargo test                                        │
│ Directory: /Users/zachswift/projects/agent-power-tools    │
│                                                            │
│ Do you want to proceed?                                   │
│   → 1. Yes                                                │
│     2. Yes, and don't ask again for 'cargo' commands      │
│     3. No (esc)                                           │
└────────────────────────────────────────────────────────────┘
```

**Interaction:**
- Arrow keys (↑↓) or numbers (1-3) to select
- Enter to confirm
- Escape = option 3 (cancel)

**In allow list:**
- Execute immediately, no prompt

### UI Update Messages

New variants for `UIUpdate` enum:

```rust
pub enum UIUpdate {
    // Existing:
    EditPreview { /* existing fields */ },

    // New:
    PermissionPrompt {
        tool_name: String,
        operation_details: String,
        suggested_pattern: String,
        response_tx: oneshot::Sender<PermissionResponse>,
    },
}

pub enum PermissionResponse {
    Yes,
    YesAndDontAsk(String),  // Contains the pattern to add
    No,
}
```

## Integration Points

### Changes to `registry.rs`

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    ui_sender: Option<UnboundedSender<UIUpdate>>,
    permission_manager: Arc<Mutex<PermissionManager>>,  // NEW
}

pub async fn execute(&self, name: &str, params: Value) -> Result<ToolResult> {
    // 1. Check permission first
    let decision = self.permission_manager.lock().unwrap()
        .check_permission(name, &params);

    match decision {
        PermissionDecision::Allow => {
            // For Edit/Write: show informational diff, then execute
            // For others: execute directly
        }
        PermissionDecision::Deny => {
            return Err("Operation denied by permissions")
        }
        PermissionDecision::Ask => {
            // Show approval prompt (existing for Edit/Write, new for others)
        }
    }

    // 2. Execute tool...
}
```

### Changes to `main.rs`

```rust
// Load config and create PermissionManager
let permission_manager = Arc::new(Mutex::new(
    PermissionManager::new(project_context.project_root.clone())?
));

// Pass to ToolRegistry
let mut tool_registry = ToolRegistry::new(permission_manager);
```

### Changes to `app.rs`

Add handling for:
1. `UIUpdate::PermissionPrompt` - Display three-option menu
2. Keyboard input for option selection (1-3, arrows, escape)
3. Send `PermissionResponse` back through channel

## Error Handling

### File System

1. **Missing config file** - Create default empty config on first run
2. **Corrupted JSON** - Log error, fall back to "Ask" for all operations (safe default)
3. **Write failures** - Warn user but still execute operation (don't block work)
4. **External modifications** - Reload config on each check (simple, correct)

### Pattern Matching

1. **Duplicate patterns** - Check before adding (avoid duplicates in config file)
2. **Conflicting patterns** - If both allow and deny match → deny wins (security)
3. **Symlinks** - Resolve to canonical paths before matching
4. **Path traversal** - Normalize all paths, reject `..` attempts to escape

### UI State

1. **Informational diff display** - Add to conversation output stream (no special timing)
2. **Multiple pending approvals** - Queue them (only one prompt at a time)
3. **User switches sessions** - Clear pending prompts on session change

### Security

1. **Path normalization** - Always resolve to canonical paths
2. **No pattern execution** - Patterns are data, never executed
3. **Overly broad warnings** - Warn if user creates `Bash(*:*)` or `Write(//**)`

## Implementation Details

### Atomic Config Writes

```rust
pub fn add_permission(&mut self, pattern: String) -> Result<()> {
    // 1. Check if pattern already exists
    if !self.config.permissions.allow.contains(&pattern) {
        self.config.permissions.allow.push(pattern);
    }

    // 2. Write atomically (temp file + rename)
    let json = serde_json::to_string_pretty(&self.config)?;
    let temp_path = self.config_path.with_extension("tmp");
    fs::write(&temp_path, json)?;
    fs::rename(temp_path, &self.config_path)?;

    Ok(())
}
```

### Informational Diff for Auto-Approved Edits

```rust
// In registry.rs
if permission == PermissionDecision::Allow && (name == "edit" || name == "write") {
    // Compute diff
    let diff = compute_diff(...);

    // Add to conversation output (non-blocking)
    if let Some(sender) = &self.ui_sender {
        sender.send(UIUpdate::ToolExecutionInfo {
            tool: name.to_string(),
            message: format!("Auto-approved edit:\n{}", diff),
        })?;
    }

    // Execute immediately
    tool.execute(params).await
}
```

## Tools Requiring Approval

Based on user requirements:

1. ✅ **Write operations** (write, edit) - Enhanced with per-file approval
2. ✅ **Bash commands** (all) - New permission prompt
3. ✅ **Git operations** - New permission prompt
4. ✅ **Read operations** - New permission prompt (for paths outside project)
5. ✅ **WebFetch** - New permission prompt (domain-based)
6. ✅ **MCP tools** - Tool-name-based permissions

## Testing Strategy

1. **Unit tests** for `PermissionManager`:
   - Pattern matching (glob, exact, command prefixes)
   - Config load/save (missing file, corrupted JSON, atomic writes)
   - Duplicate detection

2. **Integration tests** for `ToolRegistry`:
   - Allow flow (immediate execution)
   - Deny flow (rejection)
   - Ask flow (prompt display)
   - Config updates after "don't ask again"

3. **Manual UI tests**:
   - File edit approval with diff display
   - Command execution approval with three options
   - Informational diff display for auto-approved edits
   - Keyboard navigation (arrows, numbers, escape)

## Future Enhancements

1. **UI improvements**:
   - Show all allowed patterns in settings menu
   - Allow editing/removing permissions via UI
   - Risk level indicators (safe vs destructive operations)

2. **Permission features**:
   - Time-based permissions (expire after session)
   - Confirmation for overly broad patterns
   - Import/export permission profiles

3. **Security**:
   - Audit log of all approved/denied operations
   - Team-level permissions (committed config + local overrides)
   - Admin mode with stronger warnings

## Open Questions

None - design is complete and validated.

## References

- Existing approval flow: `synthia/src/tools/registry.rs` lines 78-213
- UI rendering: `synthia/src/ui/app.rs` lines 894-987
- Config format example: `/Users/zachswift/projects/substansive_synth/.claude/settings.local.json`
