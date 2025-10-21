"""
Diagnostic script to check training data formatting
"""
import json
from transformers import AutoTokenizer

# Load the tokenizer
print("Loading tokenizer...")
tokenizer = AutoTokenizer.from_pretrained("unsloth/qwen2.5-coder-7b-bnb-4bit")

# Load first training example
print("\nLoading first training example...")
with open("data/train.jsonl", "r") as f:
    first_example = json.loads(f.readline())

print("\n" + "="*80)
print("RAW TRAINING EXAMPLE:")
print("="*80)
print(json.dumps(first_example, indent=2))

# Apply chat template
print("\n" + "="*80)
print("AFTER CHAT TEMPLATE:")
print("="*80)
try:
    formatted_text = tokenizer.apply_chat_template(
        first_example["messages"],
        tokenize=False,
        add_generation_prompt=False
    )
    print(formatted_text)
except Exception as e:
    print(f"ERROR: {e}")
    print("\nTrying with tokenize=True...")
    try:
        token_ids = tokenizer.apply_chat_template(
            first_example["messages"],
            tokenize=True,
            add_generation_prompt=False
        )
        print(f"Token IDs: {token_ids[:100]}...")  # First 100 tokens
        decoded = tokenizer.decode(token_ids)
        print(f"\nDecoded:\n{decoded}")
    except Exception as e2:
        print(f"ERROR: {e2}")

# Check for special tokens
print("\n" + "="*80)
print("TOKENIZER SPECIAL TOKENS:")
print("="*80)
print(f"BOS token: {tokenizer.bos_token} (ID: {tokenizer.bos_token_id})")
print(f"EOS token: {tokenizer.eos_token} (ID: {tokenizer.eos_token_id})")
print(f"PAD token: {tokenizer.pad_token} (ID: {tokenizer.pad_token_id})")
print(f"UNK token: {tokenizer.unk_token}")

# Check chat template
print("\n" + "="*80)
print("CHAT TEMPLATE:")
print("="*80)
if hasattr(tokenizer, "chat_template") and tokenizer.chat_template:
    print(tokenizer.chat_template)
else:
    print("No chat template found!")

# Test a simple example without tool calls
print("\n" + "="*80)
print("SIMPLE EXAMPLE (NO TOOL CALLS):")
print("="*80)
simple_messages = [
    {"role": "user", "content": "Hello"},
    {"role": "assistant", "content": "Hi there!"}
]
try:
    simple_formatted = tokenizer.apply_chat_template(
        simple_messages,
        tokenize=False,
        add_generation_prompt=False
    )
    print(simple_formatted)
except Exception as e:
    print(f"ERROR: {e}")
