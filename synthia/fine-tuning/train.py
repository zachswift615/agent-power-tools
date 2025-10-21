"""
Fine-tuning script for Qwen2.5-Coder-7B on tool use dataset
Optimized for RTX 4060 (8GB VRAM) on Windows

This script uses Unsloth's QLoRA implementation for memory-efficient fine-tuning:
- 4-bit quantization (reduces VRAM by ~75%)
- Gradient checkpointing (trades compute for memory)
- Small batch size with gradient accumulation
- 8-bit optimizer (reduces optimizer state memory)

Expected VRAM usage: ~6-7GB peak
Expected training time: ~1-2 hours on RTX 4060
"""

import os
import json
import torch
from datasets import load_dataset
from unsloth import FastLanguageModel
from trl import SFTTrainer
from transformers import TrainingArguments
from datetime import datetime

# ============================================================================
# CONFIGURATION - Adjust these settings for your needs
# ============================================================================

# Model configuration
MODEL_NAME = "unsloth/qwen2.5-coder-7b-bnb-4bit"  # Pre-quantized 4-bit model
MAX_SEQ_LENGTH = 2048  # Maximum sequence length (reduce if OOM)
LOAD_IN_4BIT = True  # Use 4-bit quantization (essential for 8GB VRAM)

# LoRA configuration
# LoRA adds trainable adapter layers instead of fine-tuning the entire model
# This dramatically reduces memory requirements
LORA_RANK = 16  # Higher = more expressive but uses more VRAM (8-64 typical)
LORA_ALPHA = 16  # Scaling factor (usually same as rank)
LORA_DROPOUT = 0.05  # Dropout for regularization (0.0-0.1 typical)

# Training configuration
DATASET_PATHS = [
    "data/train.jsonl",  # Original training data (2,472 examples)
    "data/flask_templates.jsonl",  # Flask template examples
    "data/failure_recovery.jsonl",  # Failure recovery examples
    "data/train_improved.jsonl"  # Additional improved training examples
]
OUTPUT_DIR = "./outputs/qwen2.5-coder-synthia-tool-use"  # Where to save checkpoints
# RESUME_FROM_CHECKPOINT = "./outputs/qwen2.5-coder-synthia-tool-use"  # Uncomment to continue from existing checkpoint
NUM_TRAIN_EPOCHS = 1  # Number of full passes through dataset
PER_DEVICE_BATCH_SIZE = 1  # Batch size per GPU (1-2 for 8GB VRAM)
GRADIENT_ACCUMULATION_STEPS = 8  # Simulate larger batch (effective batch = 1*8 = 8)
LEARNING_RATE = 2e-4  # Learning rate (2e-4 is good default for LoRA)
WARMUP_STEPS = 10  # Linear warmup steps
MAX_STEPS = -1  # Set to positive number to override epochs (e.g., 200)
SAVE_STEPS = 50  # Save checkpoint every N steps
LOGGING_STEPS = 10  # Log metrics every N steps

# Optimization settings for 8GB VRAM
USE_GRADIENT_CHECKPOINTING = "unsloth"  # "unsloth" is more memory efficient
OPTIMIZER = "adamw_8bit"  # 8-bit optimizer saves ~50% memory over adamw
FP16 = False  # Use FP16 mixed precision (disable on RTX 4060 if issues)
BF16 = False  # Use BF16 mixed precision (better if GPU supports it)

# Advanced settings
SEED = 42  # Random seed for reproducibility
REPORT_TO = "none"  # Disable wandb/tensorboard (set to "tensorboard" if wanted)

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
        print("CUDA not available - running on CPU (will be very slow!)")

def load_and_validate_dataset(dataset_paths):
    """Load multiple datasets and combine them"""
    print_separator("Loading Dataset")

    # Check all files exist
    for path in dataset_paths:
        if not os.path.exists(path):
            raise FileNotFoundError(f"Dataset not found: {path}")

    # Load all JSONL files
    print(f"Loading {len(dataset_paths)} dataset files...")
    for path in dataset_paths:
        print(f"  - {path}")

    dataset = load_dataset('json', data_files=dataset_paths, split='train')

    print(f"✓ Loaded combined dataset with {len(dataset)} examples")

    # Validate format
    if len(dataset) > 0:
        first_example = dataset[0]
        if 'messages' not in first_example:
            raise ValueError("Dataset must have 'messages' field in ChatML format")
        print(f"✓ Dataset format validated (ChatML)")
        print(f"\nFirst example preview:")
        print(f"  - Number of messages: {len(first_example['messages'])}")
        print(f"  - Roles: {[msg['role'] for msg in first_example['messages']]}")

    return dataset

def create_model_and_tokenizer(model_name, max_seq_length, load_in_4bit, lora_rank, lora_alpha, lora_dropout, resume_from_checkpoint=None):
    """Initialize model and tokenizer with LoRA adapters"""
    print_separator("Initializing Model")

    # Check if we're resuming from a checkpoint
    if resume_from_checkpoint and os.path.exists(resume_from_checkpoint):
        print(f"🔄 Resuming from checkpoint: {resume_from_checkpoint}")
        print(f"Loading existing LoRA adapters...")

        # Load model with existing LoRA adapters
        model, tokenizer = FastLanguageModel.from_pretrained(
            model_name=resume_from_checkpoint,  # Load from checkpoint directory
            max_seq_length=max_seq_length,
            dtype=None,
            load_in_4bit=load_in_4bit,
        )

        # CRITICAL FIX: Ensure chat template is set (same fix as above)
        if not tokenizer.chat_template or not tokenizer.chat_template.strip():
            print("⚠️  Tokenizer missing chat template - loading from official Qwen2.5-Coder...")
            from transformers import AutoTokenizer
            official_tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-7B-Instruct")
            tokenizer.chat_template = official_tokenizer.chat_template
            print("✓ Chat template loaded from official Qwen2.5-Coder")
        else:
            print(f"✓ Chat template already configured")

        print(f"✓ Model loaded from checkpoint with existing LoRA adapters")
        print_gpu_memory()

    else:
        print(f"Loading model: {model_name}")
        print(f"Max sequence length: {max_seq_length}")
        print(f"4-bit quantization: {load_in_4bit}")
        print(f"LoRA rank: {lora_rank}, alpha: {lora_alpha}, dropout: {lora_dropout}")

        # Load model with Unsloth optimizations
        model, tokenizer = FastLanguageModel.from_pretrained(
            model_name=model_name,
            max_seq_length=max_seq_length,
            dtype=None,  # Auto-detect dtype
            load_in_4bit=load_in_4bit,  # Use 4-bit quantization
        )

        print(f"✓ Base model loaded")
        print_gpu_memory()

        # CRITICAL FIX: Set chat template from official Qwen2.5-Coder
        # The Unsloth 4-bit version doesn't have the chat template configured
        # Without this, apply_chat_template() fails and produces corrupted training data
        # Note: We check for empty/None/whitespace to catch all cases
        if not tokenizer.chat_template or not tokenizer.chat_template.strip():
            print("⚠️  Tokenizer missing chat template - loading from official Qwen2.5-Coder...")
            from transformers import AutoTokenizer
            official_tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-7B-Instruct")
            tokenizer.chat_template = official_tokenizer.chat_template
            print("✓ Chat template loaded from official Qwen2.5-Coder")
        else:
            print(f"✓ Chat template already configured")

        # Add LoRA adapters
        # We target attention and MLP layers for maximum impact
        model = FastLanguageModel.get_peft_model(
            model,
            r=lora_rank,  # LoRA rank
            target_modules=[
                "q_proj", "k_proj", "v_proj", "o_proj",  # Attention layers
                "gate_proj", "up_proj", "down_proj",  # MLP layers
            ],
        lora_alpha=lora_alpha,
        lora_dropout=lora_dropout,
        bias="none",  # Don't train biases
        use_gradient_checkpointing=USE_GRADIENT_CHECKPOINTING,
        random_state=SEED,
    )

    print(f"✓ LoRA adapters added")
    print_gpu_memory()

    # Verify chat template is working
    print("\n🔍 Verifying chat template...")
    try:
        test_messages = [{"role": "user", "content": "test"}]
        test_output = tokenizer.apply_chat_template(test_messages, tokenize=False, add_generation_prompt=False)
        print(f"✓ Chat template verified and working correctly")
    except Exception as e:
        print(f"❌ CRITICAL ERROR: Chat template not working!")
        print(f"   Error: {e}")
        print(f"   Training will produce corrupted data - STOPPING!")
        raise

    return model, tokenizer

def print_training_config():
    """Print training configuration summary"""
    print_separator("Training Configuration")

    effective_batch_size = PER_DEVICE_BATCH_SIZE * GRADIENT_ACCUMULATION_STEPS

    config_summary = f"""
Model Settings:
  - Model: {MODEL_NAME}
  - Max sequence length: {MAX_SEQ_LENGTH} tokens
  - 4-bit quantization: {LOAD_IN_4BIT}
  - LoRA rank: {LORA_RANK} (alpha: {LORA_ALPHA}, dropout: {LORA_DROPOUT})

Training Settings:
  - Epochs: {NUM_TRAIN_EPOCHS}
  - Max steps: {MAX_STEPS if MAX_STEPS > 0 else 'Auto (based on epochs)'}
  - Batch size per device: {PER_DEVICE_BATCH_SIZE}
  - Gradient accumulation: {GRADIENT_ACCUMULATION_STEPS}
  - Effective batch size: {effective_batch_size}
  - Learning rate: {LEARNING_RATE}
  - Warmup steps: {WARMUP_STEPS}
  - Optimizer: {OPTIMIZER}

Memory Optimizations:
  - Gradient checkpointing: {USE_GRADIENT_CHECKPOINTING}
  - Mixed precision: FP16={FP16}, BF16={BF16}
  - 8-bit optimizer: {OPTIMIZER == 'adamw_8bit'}

Output:
  - Checkpoints: {OUTPUT_DIR}
  - Save every: {SAVE_STEPS} steps
  - Log every: {LOGGING_STEPS} steps
"""
    print(config_summary)

def format_messages_for_training(examples, tokenizer):
    """Format messages using chat template"""
    texts = []
    for messages in examples["messages"]:
        # Apply chat template
        text = tokenizer.apply_chat_template(
            messages,
            tokenize=False,
            add_generation_prompt=False
        )
        # CRITICAL FIX: Append EOS token so model learns when to stop
        # Without this, model will never generate EOS and get stuck in infinite loops
        text = text + tokenizer.eos_token
        texts.append(text)
    return {"text": texts}

# ============================================================================
# MAIN TRAINING LOOP
# ============================================================================

def main():
    """Main training function"""

    print_separator("Synthia Fine-Tuning Script")
    print(f"Start time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"PyTorch version: {torch.__version__}")
    print(f"CUDA available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"CUDA device: {torch.cuda.get_device_name(0)}")
        print(f"Total VRAM: {format_vram(torch.cuda.get_device_properties(0).total_memory)}")

    # Step 1: Load dataset
    dataset = load_and_validate_dataset(DATASET_PATHS)

    # Step 2: Initialize model and tokenizer
    model, tokenizer = create_model_and_tokenizer(
        model_name=MODEL_NAME,
        max_seq_length=MAX_SEQ_LENGTH,
        load_in_4bit=LOAD_IN_4BIT,
        lora_rank=LORA_RANK,
        lora_alpha=LORA_ALPHA,
        lora_dropout=LORA_DROPOUT,
        resume_from_checkpoint=RESUME_FROM_CHECKPOINT if 'RESUME_FROM_CHECKPOINT' in dir() else None,
    )

    # Step 3: Format dataset for training
    print_separator("Preparing Dataset")
    print("Applying chat template to all examples...")
    dataset = dataset.map(
        lambda examples: format_messages_for_training(examples, tokenizer),
        batched=True,
    )
    print(f"✓ Dataset prepared with {len(dataset)} examples")

    # Step 4: Print training configuration
    print_training_config()

    # Step 5: Create training arguments
    training_args = TrainingArguments(
        # Output
        output_dir=OUTPUT_DIR,
        overwrite_output_dir=True,

        # Training schedule
        num_train_epochs=NUM_TRAIN_EPOCHS,
        max_steps=MAX_STEPS,

        # Batch size and accumulation
        per_device_train_batch_size=PER_DEVICE_BATCH_SIZE,
        gradient_accumulation_steps=GRADIENT_ACCUMULATION_STEPS,

        # Optimizer
        optim=OPTIMIZER,
        learning_rate=LEARNING_RATE,
        warmup_steps=WARMUP_STEPS,

        # Mixed precision
        fp16=FP16,
        bf16=BF16,

        # Logging and checkpointing
        logging_steps=LOGGING_STEPS,
        save_steps=SAVE_STEPS,
        save_total_limit=3,  # Keep only last 3 checkpoints

        # Other settings
        seed=SEED,
        report_to=REPORT_TO,
        load_best_model_at_end=False,  # Disable to save memory

        # Disable evaluation to save time and memory
    )

    # Step 6: Create trainer
    print_separator("Creating Trainer")
    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=dataset,
        dataset_text_field="text",  # Use the formatted text field
        max_seq_length=MAX_SEQ_LENGTH,
        args=training_args,
        packing=False,  # Don't pack sequences (can cause issues with tool use)
    )

    print("✓ Trainer initialized")
    print_gpu_memory()

    # Step 7: Start training
    print_separator("Training Started")
    print("This will take 1-2 hours on RTX 4060...")
    print("You can monitor progress below.\n")

    # Train the model
    trainer.train()

    # Step 8: Save final model
    print_separator("Saving Final Model")

    # Save LoRA adapters
    model.save_pretrained(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)

    print(f"✓ LoRA adapters saved to: {OUTPUT_DIR}")

    # Save training info
    info_path = os.path.join(OUTPUT_DIR, "training_info.json")
    training_info = {
        "model_name": MODEL_NAME,
        "training_date": datetime.now().isoformat(),
        "num_examples": len(dataset),
        "num_epochs": NUM_TRAIN_EPOCHS,
        "lora_rank": LORA_RANK,
        "lora_alpha": LORA_ALPHA,
        "learning_rate": LEARNING_RATE,
        "max_seq_length": MAX_SEQ_LENGTH,
    }

    with open(info_path, 'w') as f:
        json.dump(training_info, f, indent=2)

    print(f"✓ Training info saved to: {info_path}")

    # Final memory stats
    print_separator("Training Complete")
    print_gpu_memory()
    print(f"\nEnd time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("\nNext steps:")
    print("1. Run merge_and_export.py to merge LoRA adapters into the base model")
    print("2. Run test_model.py to test the fine-tuned model")
    print("3. Use the merged model in LM Studio or other inference engines")

if __name__ == "__main__":
    main()
