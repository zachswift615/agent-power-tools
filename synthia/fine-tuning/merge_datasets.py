#!/usr/bin/env python3
"""
Merge all training datasets into final train.jsonl and valid.jsonl files.
Combines:
- Original generated examples
- Failure recovery examples
- Flask/Jinja2 template examples
"""

import json
import random
import os

def load_jsonl(filepath):
    """Load examples from a JSONL file."""
    examples = []
    if not os.path.exists(filepath):
        print(f"Warning: {filepath} not found, skipping")
        return examples

    with open(filepath, "r") as f:
        for line in f:
            examples.append(json.loads(line))
    return examples

def save_jsonl(examples, filepath):
    """Save examples to a JSONL file."""
    with open(filepath, "w") as f:
        for example in examples:
            f.write(json.dumps(example) + "\n")

def main():
    data_dir = "/Users/zachswift/projects/agent-power-tools/synthia/fine-tuning/data"

    print("Loading datasets...")

    # Load existing training data
    original_train = load_jsonl(os.path.join(data_dir, "train.jsonl"))
    original_valid = load_jsonl(os.path.join(data_dir, "valid.jsonl"))

    print(f"Original training examples: {len(original_train)}")
    print(f"Original validation examples: {len(original_valid)}")

    # Load new datasets
    failure_recovery = load_jsonl(os.path.join(data_dir, "failure_recovery.jsonl"))
    flask_templates = load_jsonl(os.path.join(data_dir, "flask_templates.jsonl"))

    print(f"Failure recovery examples: {len(failure_recovery)}")
    print(f"Flask template examples: {len(flask_templates)}")

    # Combine all examples
    all_examples = []
    all_examples.extend(original_train)
    all_examples.extend(failure_recovery)
    all_examples.extend(flask_templates)

    # Shuffle for good distribution
    random.seed(42)  # For reproducibility
    random.shuffle(all_examples)

    # Split into train/validation (90/10 split)
    split_idx = int(len(all_examples) * 0.9)
    train_examples = all_examples[:split_idx]
    valid_examples = all_examples[split_idx:]

    # Keep original validation if we want consistency
    # Or use new split - let's use new split to include all new examples
    print(f"\nNew dataset sizes:")
    print(f"Training: {len(train_examples)} examples")
    print(f"Validation: {len(valid_examples)} examples")
    print(f"Total: {len(all_examples)} examples")

    # Save merged datasets
    train_path = os.path.join(data_dir, "train_merged.jsonl")
    valid_path = os.path.join(data_dir, "valid_merged.jsonl")

    save_jsonl(train_examples, train_path)
    save_jsonl(valid_examples, valid_path)

    print(f"\nMerged datasets saved:")
    print(f"Training: {train_path}")
    print(f"Validation: {valid_path}")

    # Calculate file sizes
    train_size = os.path.getsize(train_path) / (1024 * 1024)
    valid_size = os.path.getsize(valid_path) / (1024 * 1024)

    print(f"\nFile sizes:")
    print(f"Training: {train_size:.2f} MB")
    print(f"Validation: {valid_size:.2f} MB")

    # Analyze example types
    print(f"\nDataset composition:")
    print(f"Original examples: {len(original_train)} ({len(original_train)/len(all_examples)*100:.1f}%)")
    print(f"Failure recovery: {len(failure_recovery)} ({len(failure_recovery)/len(all_examples)*100:.1f}%)")
    print(f"Flask templates: {len(flask_templates)} ({len(flask_templates)/len(all_examples)*100:.1f}%)")

if __name__ == "__main__":
    main()
