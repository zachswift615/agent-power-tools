"""
Check the official Qwen2.5-Coder chat template
"""
from transformers import AutoTokenizer

print("Loading official Qwen2.5-Coder-7B tokenizer (not 4-bit version)...")
tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-7B-Instruct")

print("\n" + "="*80)
print("OFFICIAL QWEN2.5-CODER CHAT TEMPLATE:")
print("="*80)
if hasattr(tokenizer, "chat_template") and tokenizer.chat_template:
    print(tokenizer.chat_template)
else:
    print("No chat template found!")

# Test it with a simple example
print("\n" + "="*80)
print("TEST WITH SIMPLE EXAMPLE:")
print("="*80)
simple_messages = [
    {"role": "user", "content": "Hello"},
    {"role": "assistant", "content": "Hi there!"}
]
try:
    formatted = tokenizer.apply_chat_template(
        simple_messages,
        tokenize=False,
        add_generation_prompt=False
    )
    print(formatted)
    print("\n✓ Chat template works!")
except Exception as e:
    print(f"ERROR: {e}")

# Test with tool calls
print("\n" + "="*80)
print("TEST WITH TOOL CALLS:")
print("="*80)
tool_messages = [
    {"role": "user", "content": "Find files"},
    {
        "role": "assistant",
        "content": "I'll search for files.",
        "tool_calls": [
            {
                "id": "call_1",
                "type": "function",
                "function": {
                    "name": "glob",
                    "arguments": "{\"pattern\": \"*.py\"}"
                }
            }
        ]
    },
    {
        "role": "tool",
        "tool_call_id": "call_1",
        "name": "glob",
        "content": "main.py\ntest.py"
    },
    {
        "role": "assistant",
        "content": "Found 2 Python files."
    }
]
try:
    formatted = tokenizer.apply_chat_template(
        tool_messages,
        tokenize=False,
        add_generation_prompt=False
    )
    print(formatted)
    print("\n✓ Tool calls work!")
except Exception as e:
    print(f"ERROR: {e}")
