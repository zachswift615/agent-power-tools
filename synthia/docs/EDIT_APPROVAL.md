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
