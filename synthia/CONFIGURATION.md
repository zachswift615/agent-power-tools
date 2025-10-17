# Synthia Configuration

Synthia uses TOML configuration files to customize its behavior. This document explains all available configuration options.

## Configuration File Locations

Synthia checks the following locations for configuration files, in order:

1. **`~/.config/synthia/config.toml`** - User-wide configuration (recommended)
2. **`./synthia.toml`** - Project-specific configuration in the current directory

The first file found will be used. If no configuration file is found, Synthia will use sensible defaults.

## Creating a Configuration File

To create a configuration file, copy the example config:

```bash
# For user-wide configuration
mkdir -p ~/.config/synthia
cp config.toml.example ~/.config/synthia/config.toml

# For project-specific configuration
cp config.toml.example ./synthia.toml
```

Then edit the file to customize settings.

## Configuration Options

### LLM Provider Settings

Configure the language model provider and generation parameters.

```toml
[llm]
api_base = "http://localhost:1234/v1"
api_key = "your-api-key-here"  # Optional
model = "qwen2.5-coder-7b-instruct"
temperature = 0.7
max_tokens = 4096
```

**`api_base`** (string, required)
- Base URL for the LLM API endpoint
- For local LM Studio: `"http://localhost:1234/v1"`
- For OpenAI: `"https://api.openai.com/v1"`
- For other OpenAI-compatible providers, use their endpoint URL
- Default: `"http://localhost:1234/v1"`

**`api_key`** (string, optional)
- API key for authentication
- Leave unset or comment out for local providers like LM Studio
- Required for cloud providers like OpenAI
- Default: `null` (no authentication)

**`model`** (string, required)
- Name of the model to use
- For local models: use the model name as shown in LM Studio
- For OpenAI: `"gpt-4"`, `"gpt-3.5-turbo"`, etc.
- Default: `"qwen2.5-coder-7b-instruct"`

**`temperature`** (float, required)
- Controls randomness in generation (0.0 to 1.0)
- Lower values (0.0-0.3): More focused and deterministic
- Medium values (0.4-0.7): Balanced creativity and consistency
- Higher values (0.8-1.0): More creative and random
- Default: `0.7`

**`max_tokens`** (integer, optional)
- Maximum number of tokens to generate
- Set to control response length and cost
- Set to `null` or omit for no limit
- Default: `4096`

### Timeout Settings

Configure timeouts for various tool operations (in seconds).

```toml
[timeouts]
bash_timeout = 120
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60
```

**`bash_timeout`** (integer, required)
- Timeout for bash command execution in seconds
- Increase if you run long-running commands or scripts
- Default: `120` (2 minutes)

**`git_timeout`** (integer, required)
- Timeout for git operations in seconds
- Increase for large repositories or slow network connections
- Default: `120` (2 minutes)

**`workshop_timeout`** (integer, required)
- Timeout for Workshop CLI commands in seconds
- Usually fast, but increase if using remote storage
- Default: `30` (30 seconds)

**`powertools_timeout`** (integer, required)
- Timeout for powertools operations in seconds
- Increase for large codebases or complex semantic queries
- Default: `60` (1 minute)

### UI Settings

Configure user interface behavior.

```toml
[ui]
syntax_highlighting = true
max_output_lines = 1000
```

**`syntax_highlighting`** (boolean, required)
- Enable syntax highlighting in markdown code blocks
- Set to `false` to disable for performance or compatibility
- Default: `true`

**`max_output_lines`** (integer, required)
- Maximum number of lines to display in tool output
- Longer outputs will be truncated to prevent UI clutter
- Default: `1000`

## Examples

### Local LM Studio Setup

```toml
[llm]
api_base = "http://localhost:1234/v1"
model = "qwen2.5-coder-7b-instruct"
temperature = 0.7
max_tokens = 4096

[timeouts]
bash_timeout = 120
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60

[ui]
syntax_highlighting = true
max_output_lines = 1000
```

### OpenAI Setup

```toml
[llm]
api_base = "https://api.openai.com/v1"
api_key = "sk-your-api-key-here"
model = "gpt-4"
temperature = 0.7
max_tokens = 2048

[timeouts]
bash_timeout = 120
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60

[ui]
syntax_highlighting = true
max_output_lines = 1000
```

### Minimal Configuration (Uses Defaults)

You can omit any settings to use defaults:

```toml
[llm]
model = "custom-model"
```

All other settings will use their default values.

## Partial Configuration

Synthia supports partial configuration. Any omitted settings will use their default values. For example:

```toml
# Only customize the model and temperature
[llm]
model = "my-custom-model"
temperature = 0.5
```

This configuration only changes the model and temperature, while all other settings (API base, timeouts, UI settings) use their defaults.

## Troubleshooting

### Configuration Not Loading

If your configuration doesn't seem to be applied:

1. Check that the file is in one of the expected locations:
   - `~/.config/synthia/config.toml`
   - `./synthia.toml`

2. Verify the file is valid TOML syntax. Run:
   ```bash
   # Install a TOML validator if needed
   cargo install taplo-cli

   # Validate your config
   taplo check ~/.config/synthia/config.toml
   ```

3. Check Synthia's logs for configuration loading messages:
   ```bash
   RUST_LOG=info synthia
   ```

### Invalid TOML Syntax

Common TOML syntax errors:

- **Missing quotes around strings**: Use `api_base = "http://localhost:1234/v1"` not `api_base = http://localhost:1234/v1`
- **Wrong section names**: Must be exactly `[llm]`, `[timeouts]`, `[ui]`
- **Invalid types**: `temperature` must be a float (e.g., `0.7`), `max_tokens` must be an integer

### Default Values Not Working

If you want to use a default value:
- Either omit the setting entirely, OR
- Comment it out with `#`

For optional values like `api_key`:
- Omit the line to use `null`, OR
- Comment it out: `# api_key = "..."`

## Programmatic Access

For developers integrating Synthia:

```rust
use synthia::config::Config;

// Load config with automatic fallback to defaults
let config = Config::load()?;

// Access settings
println!("Using model: {}", config.llm.model);
println!("Temperature: {}", config.llm.temperature);

// Create a default config file
Config::create_default_config("~/.config/synthia/config.toml")?;
```

See `src/config.rs` for the full API.
