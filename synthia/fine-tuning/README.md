# Synthia Fine-Tuning Dataset

## Overview

This directory contains a training dataset for fine-tuning Qwen 2.5 Coder 7B to use Synthia's tools proactively and effectively.

## Dataset Details

- **Format**: JSON Lines (.jsonl)
- **Total Examples**: 250
- **File Size**: ~108 KB
- **Purpose**: Teach the model to use tools proactively without being prompted

## Tool Distribution

The dataset includes realistic examples across all Synthia tools:

- **bash**: 116 calls (43.3%) - System commands, build scripts, testing, monitoring
- **read**: 68 calls (25.4%) - Reading config files, source code, logs, documentation
- **glob**: 23 calls (8.6%) - Finding files by pattern (*.py, **/*.ts, etc.)
- **write**: 19 calls (7.1%) - Creating new files (.gitignore, endpoints, models)
- **edit**: 18 calls (6.7%) - Modifying existing files (config updates, code changes)
- **grep**: 14 calls (5.2%) - Searching code patterns (TODOs, functions, imports)
- **powertools**: 10 calls (3.7%) - Code navigation, refactoring, semantic search
  - goto_definition, find_references, list_functions, rename_symbol
  - search_ast, list_classes, project_stats, inline_variable
  - batch_replace, index_project

## Example Scenarios

### Simple Single-Tool Use (50%)
- Check server status
- Read configuration
- Find files by pattern
- Run tests
- Check git status

### Multi-Step Workflows (30%)
- Debug errors (read logs → check code → verify config)
- Update configuration (read → edit → verify)
- Setup features (write routes → add tests → update docs)

### Complex Workflows (20%)
- Full debugging sessions
- Feature implementation (multiple files)
- Refactoring operations
- Project setup and configuration

## Training Characteristics

### Proactive Behavior
- Model initiates tool use without "Would you like me to..."
- Direct action: "I'll check the logs" → tool call
- No unnecessary permission seeking

### Natural Language
- Varied user phrasings (not robotic)
- Real-world programming tasks
- Authentic file paths and code snippets

### Error Handling
- Realistic tool outputs (success and failure)
- Multi-step problem solving
- Context-aware follow-up actions

## Quality Guidelines

1. **Realistic**: Based on actual development workflows
2. **Diverse**: Covers all major programming tasks
3. **Proactive**: Model acts independently
4. **Natural**: Human-like conversation flow
5. **Educational**: Clear cause-and-effect patterns

## File Structure

```
fine-tuning/
├── dataset.jsonl          # Main training dataset (250 examples)
├── generate_dataset.py    # Generation script (archived)
└── README.md             # This file
```

## Usage

This dataset can be used to fine-tune Qwen 2.5 Coder 7B (or similar models) to:
- Use tools proactively without prompting
- Choose appropriate tools for tasks
- Chain tools together for complex workflows
- Maintain natural conversation while using tools

## Example Entry

```json
{
  "messages": [
    {
      "role": "user",
      "content": "Check if the server is running on port 3000"
    },
    {
      "role": "assistant",
      "content": "I'll check what's running on port 3000.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"lsof -i :3000\", \"description\": \"Check port 3000\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "name": "bash",
      "content": "node    12345 user   20u  IPv4 *:3000 (LISTEN)"
    },
    {
      "role": "assistant",
      "content": "Yes, there's a Node.js server running on port 3000 (PID 12345)."
    }
  ]
}
```

## Next Steps

1. **Fine-tune** the model using this dataset
2. **Evaluate** on held-out test cases
3. **Iterate** based on performance metrics
4. **Deploy** the fine-tuned model as Synthia v2

## License

This dataset is part of the Synthia project and follows the same license terms.
