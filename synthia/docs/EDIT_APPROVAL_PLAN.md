# Edit Approval & Command Permission System

## Overview

Add a user approval system for edits and commands with dynamic per-project permissions, implemented entirely in the UI layer without changing the AI's tool interface.

**Key Principle:** User approval is a UI concern, not an AI concern. The AI proposes actions, the user gates them, the AI sees the result.

## Goals

✅ **No training data changes needed** - Tool interface stays identical
✅ **Show diffs before edits** - Like Claude Code's edit preview
✅ **Dynamic permission system** - Learn which commands user trusts
✅ **Per-project config** - Different rules for different projects
✅ **Flexible approval modes** - From always-ask to always-allow

## Architecture

### 1. UI-Only Implementation

The assistant's perspective **never changes**:

```rust
// Assistant makes tool call
{
  "role": "assistant",
  "tool_calls": [{
    "function": {
      "name": "edit",
      "arguments": "{\"file_path\": \"src/app.js\", ...}"
    }
  }]
}

// [UI INTERCEPTS HERE - shows diff, waits for user approval]
// Assistant doesn't see this step!

// Tool returns result (same as before)
{
  "role": "tool",
  "content": "Edit successful"  // or "Edit cancelled by user"
}
```

### 2. Permission Flow

```
Tool Call → Permission Check → User Prompt (if needed) → Execute → Return Result
                    ↓
            Allowed in config?
                 Yes → Execute immediately
                 No  → Show diff/preview, ask user
```

## Per-Project Permission Config

### Storage Location

**Use Claude Code's format for compatibility!**

```
.claude/
├── settings.json              # Project settings (can commit)
└── settings.local.json        # User-specific overrides (gitignore'd)
```

This allows:
- ✅ Familiar format for Claude Code users
- ✅ Shareable team defaults (settings.json committed)
- ✅ Personal overrides (settings.local.json gitignored)
- ✅ Potential config sharing between Synthia and Claude Code

### Permission Config Schema

**Format: Claude Code compatible JSON**

```json
// .claude/settings.local.json

{
  "synthia": {
    // Approval modes
    "editApprovalMode": "ask",  // ask | session | auto | manual
    "commandApprovalMode": "ask",
    "showApprovedDiffs": true,

    // Auto-approve settings
    "autoApproveRiskLevel": "safe",  // safe | moderate | dangerous
    "approvalTimeout": 300,

    // Allowed edit patterns (always auto-approve)
    "allowedEditPatterns": [
      {
        "pattern": "src/**/*.rs",
        "reason": "User allowed Rust source edits",
        "allowedAt": "2024-10-20T22:30:00Z"
      },
      {
        "pattern": "*.md",
        "reason": "User allowed markdown edits",
        "allowedAt": "2024-10-20T22:31:00Z"
      }
    ],

    // Allowed bash commands (exact match)
    "allowedCommands": [
      {
        "command": "npm install",
        "workingDirectory": "/Users/zachswift/projects/my-app",
        "reason": "User always allows npm install in this project",
        "allowedAt": "2024-10-20T22:32:00Z"
      },
      {
        "command": "cargo build --release",
        "workingDirectory": "/Users/zachswift/projects/my-app",
        "reason": "Safe build command",
        "allowedAt": "2024-10-20T22:33:00Z"
      }
    ],

    // Denied commands (never allow)
    "deniedCommands": [
      {
        "command": "rm -rf /",
        "reason": "Dangerous command"
      }
    ],

    // Allowed command patterns (regex)
    "allowedCommandPatterns": [
      {
        "pattern": "^git (status|log|diff).*",
        "reason": "Read-only git commands"
      },
      {
        "pattern": "^ls .*",
        "reason": "List directory commands"
      }
    ]
  }
}
```

### Team-Shareable Config

```json
// .claude/settings.json (commit this)

{
  "synthia": {
    // Safe defaults for the whole team
    "allowedCommands": [
      {
        "command": "npm test",
        "reason": "Safe test command"
      },
      {
        "command": "cargo build",
        "reason": "Safe build command"
      }
    ],

    "allowedCommandPatterns": [
      {
        "pattern": "^git (status|log|diff|branch)$",
        "reason": "Read-only git commands"
      }
    ]
  }
}
```

### Config Merging

Settings are merged with precedence:
1. `.claude/settings.local.json` (highest priority - user overrides)
2. `.claude/settings.json` (team defaults)
3. `~/.config/synthia/config.toml` (global user config)
4. Built-in defaults (lowest priority)

## UI Components

### 1. Edit Diff Preview

```
┌─ Edit: src/config.rs ────────────────────────────────────┐
│                                                           │
│  14    #[serde(default)]                                  │
│  15    pub ui: UIConfig,                                  │
│  16                                                       │
│  17  + #[serde(default)]                                 │
│  18  + pub tools: ToolsConfig,                           │
│  19                                                       │
│  20  }                                                    │
│                                                           │
│  Options:                                                 │
│  [1] Accept this edit                                     │
│  [2] Always allow edits to *.rs files in this project    │
│  [3] Reject this edit                                     │
│  [4] Reject and stop asking (manual mode)                │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

### 2. Command Approval Prompt

```
┌─ Command Approval ────────────────────────────────────────┐
│                                                           │
│  Command: npm install                                     │
│  Directory: /Users/zachswift/projects/my-app             │
│                                                           │
│  This command will install Node.js dependencies.          │
│                                                           │
│  Options:                                                 │
│  [1] Allow once                                           │
│  [2] Always allow "npm install" in this directory         │
│  [3] Always allow "npm install" everywhere                │
│  [4] Reject                                               │
│  [5] Reject and block this command                        │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

### 3. Session-Wide Approval

```
┌─ Edit Approval Mode ──────────────────────────────────────┐
│                                                           │
│  Synthia wants to edit 3 files in this session.           │
│                                                           │
│  Options:                                                 │
│  [1] Review each edit (safe, slower)                      │
│  [2] Allow all edits this session (faster, less safe)     │
│  [3] Cancel all edits                                     │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Basic Edit Approval (MVP)

**Goal:** Show diff before edits, allow/reject

**Changes needed:**
- [ ] Add `UIUpdate::EditPreview` variant
- [ ] Add diff computation to Edit tool
- [ ] Add approval prompt to TUI
- [ ] Block edit execution until user responds
- [ ] Return "Edit cancelled by user" on rejection

**Files to modify:**
- `src/tools/edit.rs` - Add diff preview before execute
- `src/agent/messages.rs` - Add `EditPreview` UI update
- `src/ui/app.rs` - Add edit approval prompt UI
- `src/config.rs` - Add `edit_approval_mode` config option

### Phase 2: Permission Config Storage

**Goal:** Save approved patterns to disk using Claude Code's format

**Changes needed:**
- [ ] Create `PermissionConfig` struct
- [ ] Add Claude Code settings.json loader/saver
- [ ] Add permission checker before tool execution
- [ ] Auto-approve if permission exists
- [ ] Merge settings from multiple sources

**New files:**
- `src/permissions.rs` - Permission config management
- `src/permissions/config.rs` - JSON schema (Claude Code compatible)
- `src/permissions/checker.rs` - Permission validation
- `src/permissions/merger.rs` - Merge settings from multiple locations

**Config locations:**
- `~/.config/synthia/config.toml` - Global defaults
- `.claude/settings.json` - Team/project defaults (committed)
- `.claude/settings.local.json` - User overrides (gitignored)

### Phase 3: Dynamic Permission Learning

**Goal:** Let user add permissions on-the-fly

**Changes needed:**
- [ ] Add "Always allow" options to approval prompts
- [ ] Write new permissions to config file
- [ ] Validate and deduplicate permissions
- [ ] Show permission summary in UI

**UI enhancements:**
- Add "Always allow edits to *.rs" option
- Add "Always allow this command in this directory" option
- Add permission management command: `/permissions list|add|remove`

### Phase 4: Command Approval

**Goal:** Same approval flow for bash commands

**Changes needed:**
- [ ] Add `UIUpdate::CommandPreview` variant
- [ ] Add command risk assessment (destructive vs safe)
- [ ] Add command approval prompt
- [ ] Check command permissions before execute

**Risk categories:**
- 🟢 Safe: `ls`, `cat`, `git status`, `npm list`
- 🟡 Moderate: `npm install`, `cargo build`, `git commit`
- 🔴 Dangerous: `rm -rf`, `sudo`, `chmod -R`, `git push --force`

### Phase 5: Advanced Features

**Nice-to-haves:**
- [ ] Permission expiry (auto-revoke after N days)
- [ ] Audit log of all approvals/rejections
- [ ] Dry-run mode (show what would happen)
- [ ] Bulk permission management UI
- [ ] Import/export permissions
- [ ] Team-shared permission configs (commit to git)

## Configuration Options

### Global Config

```toml
# ~/.config/synthia/config.toml

[approval]
# Mode for edit approvals
edit_mode = "ask"  # ask | session | auto | manual

# Mode for command approvals
command_mode = "ask"  # ask | session | auto | manual

# Show diffs even for auto-approved edits
show_approved_diffs = true

# Risk threshold for auto-approval (safe | moderate | dangerous)
auto_approve_risk_level = "safe"

# Timeout for approval prompts (seconds)
approval_timeout = 300

[permissions]
# Where to store per-project permissions
storage = "claude"  # claude (.claude/settings.local.json) | synthia (.synthia/) | both

# Auto-save new permissions
auto_save = true
```

### Project Config (Claude Code Format)

```json
// .claude/settings.local.json

{
  "synthia": {
    "editApprovalMode": "ask",
    "commandApprovalMode": "ask",
    "allowedEditPatterns": [...],
    "allowedCommands": [...]
  }
}
```

**Benefits of using Claude Code's format:**
- Familiar to existing Claude Code users
- Can share configs between tools
- Standard `.claude/` directory (already gitignored by most setups)
- Team settings in `settings.json`, personal in `settings.local.json`
- JSON is easier to edit programmatically than TOML

## Safety Features

### 1. Destructive Command Detection

```rust
fn is_destructive(command: &str) -> RiskLevel {
    if command.contains("rm -rf") || command.contains("format") {
        return RiskLevel::Dangerous;
    }
    if command.contains("sudo") || command.contains("npm install") {
        return RiskLevel::Moderate;
    }
    RiskLevel::Safe
}
```

### 2. File Pattern Validation

```rust
fn validate_file_pattern(pattern: &str) -> Result<()> {
    // Don't allow overly broad patterns like "*"
    if pattern == "*" || pattern == "**/*" {
        return Err(anyhow!("Pattern too broad - please be more specific"));
    }
    Ok(())
}
```

### 3. Command Pattern Safety

```rust
fn validate_command_pattern(pattern: &str) -> Result<()> {
    // Don't allow dangerous regex that could match destructive commands
    if pattern.contains("rm") || pattern.contains("format") {
        return Err(anyhow!("Cannot create permission pattern for destructive commands"));
    }
    Ok(())
}
```

## Example User Workflows

### Workflow 1: First Edit

```
1. AI proposes edit to src/app.rs
2. UI shows diff preview
3. User sees change is good
4. User selects: "Always allow edits to src/**/*.rs"
5. Permission saved to .claude/settings.local.json
6. Future Rust edits auto-approved
```

### Workflow 2: Command Permission

```
1. AI tries: npm install
2. UI shows: "Install Node.js dependencies?"
3. User selects: "Always allow npm install in this project"
4. Permission saved with working_directory constraint
5. Future npm installs in this project auto-approved
6. npm install in other projects still requires approval
```

### Workflow 3: Bulk Approval

```
1. AI proposes 10 edits
2. User selects: "Allow all edits this session"
3. Session flag set: auto_approve_edits = true
4. All 10 edits execute immediately
5. Flag resets on next session
```

## Backwards Compatibility

✅ **No breaking changes** - Existing behavior when no permissions configured
✅ **Opt-in** - Defaults to "ask" mode, user can switch to "auto"
✅ **Training data unchanged** - AI sees same tool results
✅ **Config optional** - Works without permission files

## Testing Strategy

### Unit Tests

- [ ] Permission config parsing
- [ ] File pattern matching
- [ ] Command pattern matching
- [ ] Risk level detection

### Integration Tests

- [ ] Edit approval flow
- [ ] Command approval flow
- [ ] Permission persistence
- [ ] Session-wide approval

### Manual Test Cases

1. Edit with diff preview → Accept
2. Edit with diff preview → Reject
3. Edit with "always allow *.rs" → Auto-approve future
4. Command with "always allow npm install here" → Auto-approve future
5. Session mode → All edits approved
6. Dangerous command → Always prompt even with session mode

## Migration Path

### For Existing Users

1. **No immediate changes** - Works exactly as before
2. **Gradual adoption** - Users discover approval prompts
3. **Build permissions over time** - Natural usage creates whitelist
4. **Optional auto-approve** - Advanced users can disable prompts

### For New Users

1. **Safe by default** - Always ask on first use
2. **Learn as you go** - Build permission list through usage
3. **Quick-start templates** - Preset permissions for common projects
4. **Guided setup** - Show permission tutorial on first run

## Future Enhancements

### 1. Smart Permission Suggestions

```
AI: "I notice you've approved 5 edits to *.rs files.
     Would you like to always allow Rust edits in this project?"
```

### 2. Shared Team Permissions

```json
// .claude/settings.json (committed to git)
// Safe defaults for the team
{
  "synthia": {
    "allowedCommands": [
      {"command": "npm test", "reason": "Safe test command"},
      {"command": "cargo build", "reason": "Safe build command"}
    ]
  }
}
```

### 3. Visual Permission Manager

```
┌─ Permission Manager ──────────────────────────────────────┐
│                                                           │
│  File Edits:                                              │
│    ✓ src/**/*.rs                Added 2 days ago          │
│    ✓ *.md                       Added 1 week ago          │
│                                                           │
│  Commands:                                                │
│    ✓ npm install (this dir)     Added 3 days ago          │
│    ✓ cargo build --release      Added 1 week ago          │
│                                                           │
│  [A]dd  [R]emove  [C]lear All  [Q]uit                    │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

## Success Metrics

- Users feel in control of AI actions
- Fewer accidental destructive edits
- Faster workflows after initial permission setup
- No regression in AI behavior
- No training data regeneration needed

## References

- Claude Code's edit approval system
- Git's `--no-verify` flag pattern
- VS Code's workspace trust model
- Docker's `--privileged` mode gating
