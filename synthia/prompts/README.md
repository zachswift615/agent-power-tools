# Synthia System Prompts

This directory contains system prompts for Synthia, loaded at compile time via `include_str!`.

## Current Structure

- **`system_prompt.md`** - The main system prompt for interactive CLI mode

## Future Expansion

When Synthia supports multiple modes (CLI vs non-interactive vs sub-agent), create additional prompt files:

- `system_prompt_cli.md` - Interactive CLI mode (current default)
- `system_prompt_noninteractive.md` - Non-interactive agent mode
- `system_prompt_subagent.md` - Sub-agent mode (for agent-to-agent communication)

Update `actor.rs` to select the appropriate prompt based on initialization parameters:

```rust
// Example future implementation
const SYSTEM_PROMPT_CLI: &str = include_str!("../../prompts/system_prompt_cli.md");
const SYSTEM_PROMPT_NONINTERACTIVE: &str = include_str!("../../prompts/system_prompt_noninteractive.md");
const SYSTEM_PROMPT_SUBAGENT: &str = include_str!("../../prompts/system_prompt_subagent.md");

impl AgentActor {
    fn create_system_prompt(&self) -> Message {
        let prompt_text = match self.mode {
            AgentMode::CLI => SYSTEM_PROMPT_CLI,
            AgentMode::NonInteractive => SYSTEM_PROMPT_NONINTERACTIVE,
            AgentMode::SubAgent => SYSTEM_PROMPT_SUBAGENT,
        };

        Message {
            role: Role::System,
            content: vec![ContentBlock::Text {
                text: prompt_text.to_string(),
            }],
        }
    }
}
```

## Editing Prompts

1. Edit the markdown file directly
2. Rebuild Synthia: `cargo build`
3. Changes are embedded at compile time (no runtime file loading needed)

## Key Sections in system_prompt.md

- **Security Policy** - Guidelines for security-related requests
- **Tone and Style** - Communication style and formatting
- **Professional Objectivity** - Technical accuracy over validation
- **Tool Usage Policy** - Proactive tool usage and parallel calling
- **Available Tools** - List of tools Synthia can use
- **Code References** - How to reference code locations
- **Task Management** - Using the todo tool
- **Examples** - Concrete examples of correct behavior

## Important: Parallel Tool Calling

The prompt emphasizes parallel tool calling for performance. Examples:
- Reading multiple files: call `read` 3 times in parallel
- Independent operations: call `git` and `read` together
- Only use sequential calls when one output depends on another
