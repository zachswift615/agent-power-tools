# Synthia Training Data Improvements - Handoff Doc

**Date:** 2025-10-21
**Issue:** Model hallucinations, poor tool selection, format leaking, bad bash patterns
**Priority:** Critical - Model currently non-functional

---

## Critical Issues (Must Fix Before Deploying)

### 1. **Remove Custom Format Markers** 🔥

**Problem:** Model generates training data format markers instead of proper OpenAI function calls:
```
[tool_request] {"operation": "definition", "symbol": "write"} [END_TOOL_REQUEST]
```

**Root Cause:** Training data contains custom markers like `[tool_request]`, `[END_TOOL_REQUEST]`, `<LM>`, `{lng}`, `<lemma>`

**Fix:**
- **Remove ALL custom markers from training data**
- Use ONLY the OpenAI function calling format:

```json
{
  "messages": [
    {
      "role": "user",
      "content": "Create a Python script that prints Hello World"
    },
    {
      "role": "assistant",
      "content": "I'll create a Python script for you.",
      "tool_calls": [
        {
          "id": "call_abc123",
          "type": "function",
          "function": {
            "name": "write",
            "arguments": "{\"file_path\": \"/path/to/script.py\", \"content\": \"print('Hello, World!')\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_abc123",
      "content": "Successfully wrote to /path/to/script.py"
    },
    {
      "role": "assistant",
      "content": "Done! I've created the script at /path/to/script.py"
    }
  ]
}
```

**Verification:**
- Search your dataset for: `[tool_request]`, `<LM>`, `{lng}`, `<lemma>`, `[END_TOOL_REQUEST]`
- All should be ZERO occurrences

---

### 2. **Working Directory Awareness** 🔥

**Problem:** Model ignores user-specified paths and operates in wrong directory.

**Example Bad Behavior:**
```
User: "Create script in /Users/zachswift/projects/test-project-synthia"
Model: *runs powertools in /Users/zachswift/projects/agent-power-tools/synthia*
```

**Training Examples Needed:**

#### Example 1: Check pwd before operations
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Create a Python script in /tmp/myproject"
    },
    {
      "role": "assistant",
      "content": "I'll create the script in /tmp/myproject. Let me first check the current directory.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"pwd\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "/Users/zachswift/projects/agent-power-tools/synthia"
    },
    {
      "role": "assistant",
      "content": "I'm currently in a different directory. I'll use the absolute path to ensure the file is created in the correct location.",
      "tool_calls": [
        {
          "id": "call_2",
          "type": "function",
          "function": {
            "name": "write",
            "arguments": "{\"file_path\": \"/tmp/myproject/script.py\", \"content\": \"print('Hello')\"}"
          }
        }
      ]
    }
  ]
}
```

#### Example 2: Use cd when working in specific directory
```json
{
  "messages": [
    {
      "role": "user",
      "content": "In /Users/zachswift/projects/myapp, run the tests"
    },
    {
      "role": "assistant",
      "content": "I'll navigate to the project directory first.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"cd /Users/zachswift/projects/myapp && python -m pytest\"}"
          }
        }
      ]
    }
  ]
}
```

**How Many:** Create 15-20 examples covering:
- User specifies absolute path
- User specifies relative path
- User says "in this directory"
- Operations across multiple directories

---

### 3. **Use Write Tool for Files (NOT bash echo)** 🔥

**Problem:** Model uses bash echo with broken patterns:
```bash
echo -n "print('Hello')" >> file.py  # Creates: Hello, World!print('Hello')
```

**Training Examples Needed:**

#### Example 1: Creating new file
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Create a Python script that prints the user's name"
    },
    {
      "role": "assistant",
      "content": "I'll create a Python script using the Write tool.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "write",
            "arguments": "{\"file_path\": \"greet.py\", \"content\": \"name = input('What is your name? ')\\nprint(f'Hello, {name}!')\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "Successfully wrote to greet.py"
    },
    {
      "role": "assistant",
      "content": "Done! Created greet.py that prompts for a name and prints a greeting."
    }
  ]
}
```

#### Example 2: Show bash echo is WRONG
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Create a Python script that prints Hello"
    },
    {
      "role": "assistant",
      "content": "I'll use the Write tool to create the file with proper content.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "write",
            "arguments": "{\"file_path\": \"hello.py\", \"content\": \"print('Hello')\"}"
          }
        }
      ]
    }
  ]
}
```

**Anti-pattern to AVOID in training data:**
```json
// ❌ NEVER include examples like this:
{
  "tool_calls": [{
    "function": {
      "name": "bash",
      "arguments": "{\"command\": \"echo 'print()' > file.py\"}"
    }
  }]
}
```

**How Many:** Create 10-15 examples showing Write tool for:
- Python scripts
- Config files
- Text files
- Multi-line content with proper escaping

---

### 4. **Tool Selection Logic** 🔥

**Problem:** Model uses wrong tools for the task.

**Examples:**
- ❌ Using powertools to create new files (powertools is for *existing* codebases)
- ❌ Using bash for file creation instead of Write
- ❌ Using grep when powertools semantic search would be better

**Training Examples Needed:**

#### When to use powertools
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Find where the 'authenticate' function is defined in this codebase"
    },
    {
      "role": "assistant",
      "content": "I'll use powertools to find the definition semantically.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "powertools",
            "arguments": "{\"operation\": \"definition\", \"symbol\": \"authenticate\"}"
          }
        }
      ]
    }
  ]
}
```

#### When NOT to use powertools
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Create a new Python script"
    },
    {
      "role": "assistant",
      "content": "I'll create the script using the Write tool.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "write",
            "arguments": "{\"file_path\": \"script.py\", \"content\": \"# New script\\n\"}"
          }
        }
      ]
    }
  ]
}
```

**Decision Matrix to Encode:**

| Task | Correct Tool | Rationale |
|------|-------------|-----------|
| Find definition in existing code | powertools | Semantic navigation |
| Create new file | write | Direct file creation |
| Read existing file | read | Direct file access |
| Run command | bash | System execution |
| Search text pattern | grep (via bash) | Text search |
| Find all usages | powertools | Cross-file references |
| List files | bash (ls) | Filesystem operation |
| Modify existing file | edit | Targeted changes |

**How Many:** Create 20-25 examples covering all tool combinations with clear reasoning in assistant messages.

---

### 5. **Error Recovery Patterns** 🔥

**Problem:** Model repeats same failing command 5+ times without changing approach.

**Training Examples Needed:**

#### Example 1: Recognize repeated failure
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Run the Python script"
    },
    {
      "role": "assistant",
      "content": "I'll run the script.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"python script.py\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "Error: SyntaxError: invalid syntax"
    },
    {
      "role": "assistant",
      "content": "The script has a syntax error. Let me read it to see what's wrong.",
      "tool_calls": [
        {
          "id": "call_2",
          "type": "function",
          "function": {
            "name": "read",
            "arguments": "{\"file_path\": \"script.py\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_2",
      "content": "print('Hello'\n"
    },
    {
      "role": "assistant",
      "content": "I see the issue - missing closing parenthesis. I'll fix it.",
      "tool_calls": [
        {
          "id": "call_3",
          "type": "function",
          "function": {
            "name": "edit",
            "arguments": "{\"file_path\": \"script.py\", \"old_string\": \"print('Hello'\\n\", \"new_string\": \"print('Hello')\\n\"}"
          }
        }
      ]
    }
  ]
}
```

#### Example 2: Try alternative when first approach fails
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Find the main function"
    },
    {
      "role": "assistant",
      "content": "I'll search for the main function.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"grep -r 'def main' .\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": ""
    },
    {
      "role": "assistant",
      "content": "No Python files found. Let me try looking for Rust or other languages.",
      "tool_calls": [
        {
          "id": "call_2",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"grep -r 'fn main' .\"}"
          }
        }
      ]
    }
  ]
}
```

**Error Recovery Patterns to Include:**
1. Read file after write/edit failure
2. Try alternative search method when first fails
3. Check file exists before operations
4. Verify command syntax after error
5. Switch tools when one doesn't work

**How Many:** Create 15-20 examples of error recovery across different scenarios.

---

## Medium Priority Issues

### 6. **Simple Task Completion**

**Problem:** Model over-complicates simple requests.

**Example Bad:**
```
User: "Create a Python script that prints Hello"
Model: *indexes codebase* -> *searches for patterns* -> *creates file* -> *runs it* -> *fixes errors* (5+ tool calls)
```

**Example Good:**
```
User: "Create a Python script that prints Hello"
Model: *uses Write tool once*
```

**Training Examples Needed:**

```json
{
  "messages": [
    {
      "role": "user",
      "content": "Create a Python script that prints Hello World"
    },
    {
      "role": "assistant",
      "content": "I'll create the script.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "write",
            "arguments": "{\"file_path\": \"hello.py\", \"content\": \"print('Hello World')\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "Successfully wrote to hello.py"
    },
    {
      "role": "assistant",
      "content": "Done! Created hello.py"
    }
  ]
}
```

**How Many:** Create 20-30 examples of simple 1-2 tool call tasks:
- Create file
- Read file
- Run command
- Simple edits
- Basic searches

---

### 7. **One-Shot Solutions**

**Problem:** Model doesn't plan ahead and execute in one go.

**Training Examples Needed:**

```json
{
  "messages": [
    {
      "role": "user",
      "content": "Create a Python function that calculates factorial"
    },
    {
      "role": "assistant",
      "content": "I'll create a factorial function.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "write",
            "arguments": "{\"file_path\": \"factorial.py\", \"content\": \"def factorial(n):\\n    if n <= 1:\\n        return 1\\n    return n * factorial(n - 1)\\n\\nif __name__ == '__main__':\\n    print(factorial(5))\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "Successfully wrote to factorial.py"
    },
    {
      "role": "assistant",
      "content": "Created factorial.py with a recursive factorial function and test code."
    }
  ]
}
```

**How Many:** Create 15-20 examples showing complete solutions in single tool call.

---

### 8. **Proper Stop Sequences**

**Problem:** Model continues generating after completing task.

**Training Format:**
Ensure ALL examples end with:
1. Assistant message with no tool_calls
2. Clear completion statement
3. No trailing text

**Example:**
```json
{
  "messages": [
    // ... task completion ...
    {
      "role": "assistant",
      "content": "Done! The script is ready to run."
    }
    // ✓ Ends here - no more messages
  ]
}
```

**Anti-pattern:**
```json
{
  "messages": [
    {
      "role": "assistant",
      "content": "Done! [tool_request] Let me also..."
    }
    // ❌ Continues unnecessarily
  ]
}
```

---

## Dataset Quality Checklist

Before fine-tuning, verify:

- [ ] **Zero** occurrences of custom markers (`[tool_request]`, `<LM>`, etc.)
- [ ] All examples use OpenAI function calling format exactly
- [ ] All tool calls have proper `id`, `type`, `function` structure
- [ ] All tool responses have matching `tool_call_id`
- [ ] Assistant messages are natural and concise
- [ ] No bash echo for file creation (use Write tool)
- [ ] Path awareness examples included
- [ ] Error recovery examples included
- [ ] Simple tasks have simple solutions (1-2 tool calls)
- [ ] Examples end cleanly without continuation

---

## Example Dataset Structure

```
synthia-training-data/
├── high-priority/
│   ├── 01-format-markers-REMOVED.jsonl        # Clean OpenAI format examples
│   ├── 02-directory-awareness.jsonl           # 20 examples
│   ├── 03-write-tool-for-files.jsonl          # 15 examples
│   ├── 04-tool-selection-logic.jsonl          # 25 examples
│   └── 05-error-recovery.jsonl                # 20 examples
├── medium-priority/
│   ├── 06-simple-task-completion.jsonl        # 30 examples
│   ├── 07-one-shot-solutions.jsonl            # 20 examples
│   └── 08-proper-stop-sequences.jsonl         # Built into all
└── validation/
    └── test-set.jsonl                         # 50 examples for validation
```

---

## Testing After Fine-Tuning

**Critical Tests:**
1. "Create a Python script in /tmp/test that prints Hello" - Should use Write with absolute path
2. "Find the main function" - Should NOT generate custom markers
3. Run failing command 3 times - Should try alternative after 1-2 failures
4. Simple file creation - Should complete in 1 tool call

**Success Criteria:**
- ✓ No custom format markers in output
- ✓ Uses correct working directory 90%+ of time
- ✓ Uses Write tool (not bash echo) for files 95%+ of time
- ✓ Recovers from errors within 3 tool calls
- ✓ Completes simple tasks in 1-2 tool calls

---

## Contact

Questions? Check:
- OpenAI function calling docs: https://platform.openai.com/docs/guides/function-calling
- Synthia tool registry: `src/tools/registry.rs`
- Example session logs in this handoff doc

**Priority Order:**
1. Fix custom markers (blocks everything)
2. Add directory awareness (critical for usability)
3. Fix Write tool usage (prevents file corruption)
4. Add error recovery (prevents loops)
5. Simplify task completion (improves UX)
