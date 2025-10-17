"""
Quick inference test script for fine-tuned Synthia model
Tests if the model learned tool use patterns correctly

This script:
1. Loads the fine-tuned model (from merged or LoRA)
2. Tests with sample prompts
3. Verifies tool calls are generated correctly
4. Prints formatted output for inspection

Expected VRAM usage: ~6-7GB
Expected time: <1 minute per test
"""

import os
import json
import torch
from unsloth import FastLanguageModel
from datetime import datetime

# ============================================================================
# CONFIGURATION
# ============================================================================

# Model configuration - Update these paths based on your export
USE_MERGED_MODEL = True  # Use merged 16-bit model (recommended for testing)
MERGED_MODEL_PATH = "./outputs/qwen2.5-coder-synthia-merged/16bit"

# Alternative: Use LoRA adapters directly
USE_LORA_ADAPTERS = False
BASE_MODEL = "unsloth/qwen2.5-coder-7b-bnb-4bit"
LORA_ADAPTER_PATH = "./outputs/qwen2.5-coder-synthia-tool-use"

# Generation settings
MAX_SEQ_LENGTH = 2048
LOAD_IN_4BIT = True  # Use 4-bit to save VRAM during testing
MAX_NEW_TOKENS = 512  # Maximum tokens to generate
TEMPERATURE = 0.1  # Low temperature for more deterministic outputs
TOP_P = 0.9
DO_SAMPLE = True

# Test prompts - These mimic Claude Code asking for tool use
TEST_PROMPTS = [
    {
        "name": "Read file test",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful coding assistant with access to file system tools."
            },
            {
                "role": "user",
                "content": "What's in the README.md file?"
            }
        ]
    },
    {
        "name": "Search for Python errors",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful coding assistant with access to file system tools."
            },
            {
                "role": "user",
                "content": "Check if there are any Python syntax errors in the codebase."
            }
        ]
    },
    {
        "name": "Find function definition",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful coding assistant with access to file system tools."
            },
            {
                "role": "user",
                "content": "Where is the function 'process_data' defined? I need to see its implementation."
            }
        ]
    },
    {
        "name": "List files",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful coding assistant with access to file system tools."
            },
            {
                "role": "user",
                "content": "Show me all TypeScript files in the src/ directory."
            }
        ]
    },
]

# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

def print_separator(title=""):
    """Print a nice separator for console output"""
    if title:
        print(f"\n{'=' * 80}")
        print(f"  {title}")
        print(f"{'=' * 80}\n")
    else:
        print(f"{'=' * 80}\n")

def format_vram(bytes_value):
    """Format bytes to GB"""
    return f"{bytes_value / 1024**3:.2f} GB"

def print_gpu_memory():
    """Print current GPU memory usage"""
    if torch.cuda.is_available():
        allocated = torch.cuda.memory_allocated()
        reserved = torch.cuda.memory_reserved()
        print(f"GPU Memory: {format_vram(allocated)} allocated, {format_vram(reserved)} reserved")
    else:
        print("CUDA not available")

def load_model_for_inference(use_merged, merged_path, use_lora, base_model, lora_path, max_seq_length, load_4bit):
    """Load model for inference (either merged or with LoRA adapters)"""
    print_separator("Loading Model for Inference")

    if use_merged:
        if not os.path.exists(merged_path):
            raise FileNotFoundError(
                f"Merged model not found at: {merged_path}\n"
                f"Please run merge_and_export.py first."
            )

        print(f"Loading merged model from: {merged_path}")

        model, tokenizer = FastLanguageModel.from_pretrained(
            model_name=merged_path,
            max_seq_length=max_seq_length,
            dtype=None,
            load_in_4bit=load_4bit,
        )

        print(f"✓ Merged model loaded")

    elif use_lora:
        if not os.path.exists(lora_path):
            raise FileNotFoundError(
                f"LoRA adapters not found at: {lora_path}\n"
                f"Please run train.py first."
            )

        print(f"Loading base model: {base_model}")
        print(f"Loading LoRA adapters from: {lora_path}")

        model, tokenizer = FastLanguageModel.from_pretrained(
            model_name=base_model,
            max_seq_length=max_seq_length,
            dtype=None,
            load_in_4bit=load_4bit,
        )

        model = FastLanguageModel.get_peft_model(
            model,
            r=16,
            target_modules=[
                "q_proj", "k_proj", "v_proj", "o_proj",
                "gate_proj", "up_proj", "down_proj",
            ],
            lora_alpha=16,
            lora_dropout=0.05,
            bias="none",
            use_gradient_checkpointing="unsloth",
            random_state=42,
        )

        from peft import PeftModel
        model = PeftModel.from_pretrained(model, lora_path)

        print(f"✓ Model with LoRA adapters loaded")

    else:
        raise ValueError("Must set either USE_MERGED_MODEL or USE_LORA_ADAPTERS to True")

    # Enable inference mode
    FastLanguageModel.for_inference(model)

    print_gpu_memory()

    return model, tokenizer

def extract_tool_calls(text):
    """Extract tool call patterns from generated text"""
    tool_calls = []

    # Look for <function_calls> blocks
    if "<function_calls>" in text:
        start_idx = text.find("<function_calls>")
        end_idx = text.find("