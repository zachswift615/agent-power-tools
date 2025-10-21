# New Training Data - Failure Recovery & Flask Templates

## Overview

Added **314 new training examples** focused on:
1. **Tool Failure Recovery** (300 examples) - Teaching the model to analyze errors and change strategy
2. **Flask/Jinja2 Templates** (14 examples) - Teaching appropriate tech stack choices for Flask projects

## Total Dataset Size

- **Training examples:** 2,417 (was 2,372, +45 net after 90/10 split)
- **Validation examples:** 269 (was 100)
- **Total:** 2,686 examples

## Dataset Composition

- Original examples: 2,372 (88.3%)
- Failure recovery: 300 (11.2%)
- Flask templates: 14 (0.5%)

## New Example Types

### 1. Tool Failure Recovery (300 examples)

Teaches the model to:
- **Analyze errors** instead of blindly retrying failed commands
- **Change strategy** when a tool fails
- **Use context clues** to choose appropriate tools

#### Patterns Covered

**Pattern 1: Wrong Package Manager (4 base scenarios × 75 repetitions)**
- User: "Install dependencies"
- Assistant tries: `npm install`
- Error: "package.json not found"
- Recovery: Check project type with glob → find requirements.txt → use `pip install -r requirements.txt`

Examples include:
- npm → pip (Python project)
- pip → npm (Node.js project)
- npm → cargo (Rust project)
- pip → bundle (Ruby project)

**Pattern 2: File Not Found (2 scenarios × 75 repetitions)**
- User: "Show me the main file"
- Assistant tries: `/app/main.py`
- Error: "File not found"
- Recovery: Use glob to find Python files → discover `src/app.py`

**Pattern 3: Permission Denied (2 scenarios × 75 repetitions)**
- User: "Check system logs"
- Assistant tries: `cat /var/log/syslog`
- Error: "Permission denied"
- Recovery: Try alternative command like `journalctl`

**Pattern 4: Command Not Found (2 scenarios × 75 repetitions)**
- User: "Check what's on port 8080"
- Assistant tries: `lsof -i :8080`
- Error: "command not found"
- Recovery: Try alternative like `netstat -tulpn | grep :8080`

### 2. Flask + Jinja2 Templates (14 examples)

Teaches the model to:
- **Recognize Flask/Python projects** from directory structure
- **Choose appropriate technology** (HTML/Jinja2, not React) for Flask apps
- **Create proper Flask templates** with Jinja2 syntax
- **Build CRUD interfaces** with simple HTML/CSS/vanilla JS

#### Examples

**Complete Todo App CRUD** (5 variations)
- User: "Build a frontend for my Flask todo app"
- Assistant:
  1. Checks project type with `glob **/*.py`
  2. Reads Flask app.py to understand routes
  3. Creates templates/index.html with Jinja2
  4. Implements full CRUD UI (add, complete, delete)
  5. Uses embedded CSS, no build tools required

**Blog Article Form** (3 variations)
- Creates HTML form for creating articles
- Proper Jinja2 templating
- Clean, styled forms

**Wrong Tech Stack Prevention** (3 variations)
- User: "Build a frontend"
- Assistant:
  1. Checks project type
  2. Finds Flask/Python files
  3. Creates Jinja2 templates instead of trying create-react-app
  4. Explains why this is the correct approach

## Key Learning Objectives

### For Failure Recovery

✅ **Don't retry the same failed command** - Analyze and change approach
✅ **Use error messages as clues** - "package.json not found" → not a Node project
✅ **Explore before assuming** - Use glob/grep to discover project structure
✅ **Know alternative tools** - lsof vs netstat, cat vs journalctl

### For Flask Templates

✅ **Detect project type first** - Check for *.py, requirements.txt
✅ **Choose appropriate tech** - Flask = Jinja2 templates, not React
✅ **Create production-ready templates** - Proper structure, styling, UX
✅ **Keep it simple** - No build tools needed for Flask templates

## How to Use

### Training with Merged Dataset

```bash
cd /Users/zachswift/projects/agent-power-tools/synthia/fine-tuning

# Use the merged datasets (already created)
python3 train_mlx.py \\
  --data data/train_merged.jsonl \\
  --valid data/valid_merged.jsonl \\
  --model-name Qwen/Qwen2.5-Coder-7B-Instruct \\
  --iters 600 \\
  --learning-rate 1e-5
```

### Regenerating Individual Datasets

```bash
# Regenerate failure recovery examples
python3 generate_failure_recovery.py

# Regenerate Flask examples
python3 generate_flask_examples.py

# Merge all datasets
python3 merge_datasets.py
```

## Files

- `generate_failure_recovery.py` - Generator for 300 failure recovery examples
- `generate_flask_examples.py` - Generator for 14 Flask template examples
- `merge_datasets.py` - Merges all datasets into train_merged.jsonl and valid_merged.jsonl
- `data/failure_recovery.jsonl` - 300 failure recovery examples
- `data/flask_templates.jsonl` - 14 Flask template examples
- `data/train_merged.jsonl` - Combined training set (2,417 examples)
- `data/valid_merged.jsonl` - Combined validation set (269 examples)

## Expected Improvements

After training with these examples, the model should:

1. **Stop getting stuck in retry loops** with npm install on Flask projects
2. **Analyze error messages** and change strategy appropriately
3. **Choose the right tech stack** for Flask apps (Jinja2, not React)
4. **Create production-ready HTML templates** for Flask projects
5. **Use context clues** from file structure to make better tool choices

## Validation

All examples validated for:
- ✅ Correct JSON format
- ✅ Proper message structure (user/assistant/tool roles)
- ✅ Valid tool_calls format
- ✅ Realistic conversation flow
