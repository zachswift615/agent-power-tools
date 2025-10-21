"""
Verify the training data was actually formatted correctly
by checking what the trainer saw during training
"""
import json
from transformers import AutoTokenizer
from datasets import load_dataset

MODEL_NAME = "unsloth/qwen2.5-coder-7b-bnb-4bit"
DATASET_PATHS = [
    "data/train.jsonl",
    "data/flask_templates.jsonl",
    "data/failure_recovery.jsonl"
]

print("="*80)
print("VERIFYING TRAINING DATA FORMAT")
print("="*80)

# Load tokenizer
print("\n1. Loading tokenizer...")
tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)

# Check if chat template exists
print(f"\n2. Checking base tokenizer...")
print(f"   Has chat template: {tokenizer.chat_template is not None}")

if not tokenizer.chat_template:
    print("   ❌ Base tokenizer has NO chat template!")
    print("   Loading from official Qwen...")
    from transformers import AutoTokenizer as AT
    official_tok = AT.from_pretrained("Qwen/Qwen2.5-Coder-7B-Instruct")
    tokenizer.chat_template = official_tok.chat_template
    print("   ✓ Chat template loaded")

# Load a few training examples
print("\n3. Loading training examples...")
datasets = []
for path in DATASET_PATHS:
    try:
        ds = load_dataset("json", data_files=path, split="train")
        datasets.append(ds)
        print(f"   ✓ Loaded {len(ds)} examples from {path}")
    except:
        pass

# Combine datasets
from datasets import concatenate_datasets
combined = concatenate_datasets(datasets) if datasets else None

if combined is None:
    print("   ❌ Could not load any datasets!")
    exit(1)

print(f"\n4. Checking first 3 formatted examples:")

for i in range(min(3, len(combined))):
    example = combined[i]

    print(f"\n{'='*80}")
    print(f"EXAMPLE {i+1}:")
    print(f"{'='*80}")

    # Apply chat template (this is what train.py does)
    formatted = tokenizer.apply_chat_template(
        example["messages"],
        tokenize=False,
        add_generation_prompt=False
    )

    print(formatted)

    # Check for critical markers
    has_im_start = "<|im_start|>" in formatted
    has_im_end = "<|im_end|>" in formatted
    has_tool_call = "<tool_call>" in formatted
    has_endoftext = "<|endoftext|>" in formatted

    print(f"\n✓ Has <|im_start|>: {has_im_start}")
    print(f"✓ Has <|im_end|>: {has_im_end}")
    print(f"✓ Has <tool_call>: {has_tool_call}")
    print(f"⚠️ Has <|endoftext|>: {has_endoftext}")

    if not has_im_end:
        print("❌ CRITICAL: Missing <|im_end|> markers!")
        print("   Model won't learn when to stop!")

print(f"\n{'='*80}")
print("DIAGNOSIS:")
print(f"{'='*80}")

# Check one example in detail
example = combined[0]
formatted = tokenizer.apply_chat_template(
    example["messages"],
    tokenize=False,
    add_generation_prompt=False
)

# Count im_end tokens
im_end_count = formatted.count("<|im_end|>")
message_count = len(example["messages"])

print(f"\nFirst example has:")
print(f"  - {message_count} messages")
print(f"  - {im_end_count} <|im_end|> tokens")

if im_end_count < message_count:
    print(f"\n❌ PROBLEM: Not enough <|im_end|> tokens!")
    print(f"   Expected: {message_count}, Got: {im_end_count}")
    print(f"   The model won't learn proper message boundaries.")
else:
    print(f"\n✓ Correct number of <|im_end|> tokens")

# Check if last message has im_end
if not formatted.endswith("<|im_end|>") and not formatted.endswith("<|im_end|>\n"):
    print(f"\n❌ CRITICAL: Training examples don't end with <|im_end|>!")
    print(f"   Last 100 chars: ...{formatted[-100:]}")
    print(f"   Model won't learn to stop generating!")
else:
    print(f"\n✓ Training examples properly end with <|im_end|>")

print("\n" + "="*80)
print("CONCLUSION:")
print("="*80)

if im_end_count >= message_count and (formatted.endswith("<|im_end|>") or formatted.endswith("<|im_end|>\n")):
    print("✓ Training data appears correctly formatted.")
    print("  Problem might be in the training process itself.")
    print("  Possible causes:")
    print("    1. Trainer not using formatted data correctly")
    print("    2. EOS token not added during tokenization")
    print("    3. Merge step corrupted the model")
else:
    print("❌ Training data NOT correctly formatted!")
    print("  This explains why the model can't stop generating.")
    print("  Need to fix chat template application in train.py")
