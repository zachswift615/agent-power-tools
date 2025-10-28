# Synthia Configuration Guide

## Single Source of Truth

Synthia uses a clear configuration hierarchy with **two possible config locations**:

1. **Global config** (user-wide settings): `~/.config/synthia/config.toml`
2. **Project config** (per-project overrides): `./synthia.toml`

## Configuration Priority

```
Highest → Lowest Priority:
1. ./synthia.toml         (project-level - overrides everything)
2. ~/.config/synthia/config.toml  (global user settings)
3. Hardcoded defaults     (fallback if no config found)
```

## Quick Start

### Global Configuration

Create your global config:

```bash
mkdir -p ~/.config/synthia
cat > ~/.config/synthia/config.toml << 'EOF'
[llm]
api_base = "http://localhost:1234/v1"
api_key = ""
model = "qwen/qwen3-coder-30b"
temperature = 0.7
max_tokens = 4096
streaming = true
context_window = 8192

[timeouts]
bash_timeout = 300
git_timeout = 120
workshop_timeout = 30
powertools_timeout = 60

[ui]
syntax_highlighting = true
max_output_lines = 1000
edit_approval = true
EOF
```

### Project-Level Overrides

Create a `synthia.toml` in your project directory to override specific settings:

```toml
# Project-specific config - only override what you need
[llm]
model = "deepseek/deepseek-coder-33b"  # Use different model for this project
temperature = 0.5                       # Lower temp for more deterministic output

# Other settings inherit from global config
```

## Configuration Fields

### [llm] - Language Model Settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_base` | string | `"http://localhost:1234/v1"` | LM Studio or OpenAI-compatible API endpoint |
| `api_key` | string | `""` | API key (optional for LM Studio) |
| `model` | string | `"google/gemma-3-12b"` | Model name/ID |
| `temperature` | float | `0.7` | Sampling temperature (0.0-1.0) |
| `max_tokens` | int | `4096` | Maximum tokens to generate |
| `streaming` | bool | `true` | Enable streaming responses |
| `context_window` | int | `8192` | Model's context window size |

### [timeouts] - Tool Timeout Settings (seconds)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `bash_timeout` | int | `300` | Timeout for bash commands |
| `git_timeout` | int | `120` | Timeout for git operations |
| `workshop_timeout` | int | `30` | Timeout for workshop commands |
| `powertools_timeout` | int | `60` | Timeout for powertools operations |

### [ui] - User Interface Settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `syntax_highlighting` | bool | `true` | Enable code syntax highlighting |
| `max_output_lines` | int | `1000` | Maximum lines to display in tool output |
| `edit_approval` | bool | `true` | Require approval for file edits |

### [tools] - Tool Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `powertools_binary_path` | string | null | Custom path to powertools binary (optional) |

## Remote LM Studio Setup

To connect to LM Studio on another computer:

1. **On the LM Studio computer:**
   - Open LM Studio
   - Go to Settings → Server
   - Change Host from `127.0.0.1` to `0.0.0.0`
   - Start the server

2. **On your Synthia computer:**

   Update `~/.config/synthia/config.toml`:

   ```toml
   [llm]
   api_base = "http://192.168.1.79:1234/v1"  # Replace with LM Studio computer's IP
   model = "qwen/qwen3-coder-30b"
   ```

3. **Test the connection:**

   ```bash
   curl http://192.168.1.79:1234/v1/models
   ```

## Troubleshooting

### Config not being loaded?

Check the Synthia startup logs:

```bash
# You should see one of these:
# "Loading global config from: /Users/you/.config/synthia/config.toml"
# "Loading project config from: /path/to/project/synthia.toml"
# "No config file found, using defaults"
```

### Wrong config location?

Synthia only checks:
- `~/.config/synthia/config.toml` (NOT `~/.synthia/config.toml`)
- `./synthia.toml` (in current directory)

### Project config not working?

Make sure it's named exactly `synthia.toml` (not `config.toml` or `.synthia.toml`)

## Examples

### Example 1: Different Models Per Project

**Global config** (`~/.config/synthia/config.toml`):
```toml
[llm]
model = "qwen/qwen3-coder-30b"  # Default: code-focused model
```

**Project A** (data science project):
```toml
# ./synthia.toml
[llm]
model = "deepseek/deepseek-coder-33b"  # Better for data tasks
```

**Project B** (uses global):
```
# No synthia.toml - uses global config
```

### Example 2: Local vs Remote Development

**Global config** (laptop):
```toml
[llm]
api_base = "http://192.168.1.79:1234/v1"  # Desktop with GPU
model = "qwen/qwen3-coder-30b"
```

**Project override** (when working offline):
```toml
# ./synthia.toml
[llm]
api_base = "http://localhost:1234/v1"  # Local LM Studio
model = "qwen/qwen2.5-coder-7b"        # Smaller model for laptop
```
