#!/usr/bin/env python3
"""
MLX Fine-Tuning Pipeline for Synthia (Qwen2.5-Coder)
Optimized for Mac M1 Pro with 16GB RAM

Base model: Qwen/Qwen2.5-Coder-7B-Instruct
"""

import json
import argparse
from pathlib import Path
import mlx.core as mx
import mlx.nn as nn
from mlx_lm import load, generate
from mlx_lm.tuner import lora, utils


def validate_dataset(data_path: Path):
    """Validate JSONL dataset format"""
    print(f"Validating dataset: {data_path}")

    # MLX expects a directory with train.jsonl, valid.jsonl
    train_file = data_path / "train.jsonl"

    if not train_file.exists():
        raise FileNotFoundError(f"Training file not found: {train_file}")

    with open(train_file) as f:
        lines = f.readlines()

    print(f"Total training examples: {len(lines)}")

    for i, line in enumerate(lines[:5]):  # Check first 5
        try:
            data = json.loads(line)
            assert "messages" in data, f"Line {i+1}: Missing 'messages' key"

            # Check message structure
            for msg in data["messages"]:
                assert "role" in msg, f"Line {i+1}: Message missing 'role'"
                assert "content" in msg, f"Line {i+1}: Message missing 'content'"

            print(f"  ✓ Line {i+1}: Valid")
        except Exception as e:
            print(f"  ✗ Line {i+1}: {e}")
            raise

    print("✓ Dataset validation passed!")
    return len(lines)


def train_stage(
    model_path: str,
    data_path: Path,
    stage_name: str,
    iterations: int,
    learning_rate: float,
    output_dir: Path,
    batch_size: int = 2,
    lora_rank: int = 16,
    lora_alpha: int = 32,
    max_seq_length: int = 2048,
):
    """Train one stage of fine-tuning"""

    print(f"\n{'='*60}")
    print(f"STAGE: {stage_name}")
    print(f"{'='*60}")
    print(f"Model: {model_path}")
    print(f"Data: {data_path}")
    print(f"Iterations: {iterations}")
    print(f"Learning rate: {learning_rate}")
    print(f"Batch size: {batch_size}")
    print(f"LoRA rank: {lora_rank}")
    print(f"Max sequence length: {max_seq_length}")
    print(f"Output: {output_dir}")
    print(f"{'='*60}\n")

    # Validate dataset
    num_examples = validate_dataset(data_path)

    # Training config
    config = {
        "model": model_path,
        "train": True,
        "data": str(data_path.absolute()),  # Use absolute path
        "batch_size": batch_size,
        "iters": iterations,
        "val_batches": 5,  # Reduced from 25 - much faster validation
        "learning_rate": learning_rate,
        "steps_per_report": 50,  # Report less frequently
        "steps_per_eval": 200,  # Validate less frequently (every 200 iters instead of 50)
        "save_every": 200,  # Save less frequently
        "adapter_path": str(output_dir.absolute()),  # Use absolute path
        "max_seq_length": max_seq_length,
        "grad_checkpoint": True,

        # LoRA settings (note: not all may be used by MLX, depends on version)
        "num_layers": 16,  # Changed from lora_layers
    }

    # Save config
    output_dir.mkdir(parents=True, exist_ok=True)
    config_path = output_dir / "config.json"
    with open(config_path, "w") as f:
        json.dump(config, f, indent=2)
    print(f"✓ Saved config to {config_path}\n")

    # Run training via command line (easier than Python API)
    import subprocess

    cmd = [
        "mlx_lm.lora",
        "--model", model_path,
        "--data", str(data_path.absolute()),  # Use absolute path
        "--train",
        "-c", str(config_path),  # Use config file for all parameters
    ]

    print(f"Running: {' '.join(cmd)}\n")

    try:
        subprocess.run(cmd, check=True)
        print(f"\n✓ Stage '{stage_name}' training complete!")
        return True
    except subprocess.CalledProcessError as e:
        print(f"\n✗ Training failed: {e}")
        return False


def fuse_lora_weights(model_path: str, adapter_file: Path, output_path: Path):
    """Fuse LoRA weights back into base model"""

    print(f"\n{'='*60}")
    print(f"FUSING LORA WEIGHTS")
    print(f"{'='*60}")
    print(f"Model: {model_path}")
    print(f"Adapter: {adapter_file}")
    print(f"Output: {output_path}")
    print(f"{'='*60}\n")

    import subprocess

    cmd = [
        "mlx_lm.fuse",
        "--model", model_path,
        "--adapter-path", str(adapter_file.parent),  # Use parent directory, not the file itself
        "--save-path", str(output_path),
    ]

    print(f"Running: {' '.join(cmd)}\n")

    try:
        subprocess.run(cmd, check=True)
        print(f"\n✓ LoRA weights fused! Model saved to {output_path}")
        return True
    except subprocess.CalledProcessError as e:
        print(f"\n✗ Fusing failed: {e}")
        return False


def test_model(model_path: str, prompt: str):
    """Quick test of fine-tuned model"""

    print(f"\n{'='*60}")
    print(f"TESTING MODEL: {model_path}")
    print(f"{'='*60}")
    print(f"Prompt: {prompt}")
    print(f"{'='*60}\n")

    model, tokenizer = load(model_path)

    messages = [
        {"role": "system", "content": "You are Synthia, a helpful coding assistant with access to tools."},
        {"role": "user", "content": prompt}
    ]

    prompt_text = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
    response = generate(model, tokenizer, prompt=prompt_text, max_tokens=500, verbose=True)

    print(f"\n{'='*60}")
    print(f"RESPONSE:")
    print(f"{'='*60}")
    print(response)
    print(f"{'='*60}\n")


def main():
    parser = argparse.ArgumentParser(description="MLX Fine-Tuning Pipeline for Synthia")
    parser.add_argument("--stage", choices=["1", "2", "3", "all"], default="all",
                       help="Which training stage to run")
    parser.add_argument("--base-model", default="Qwen/Qwen2.5-Coder-7B-Instruct",
                       help="Base model to fine-tune from")
    parser.add_argument("--dataset", default="fine-tuning/data",
                       help="Path to training dataset directory (contains train.jsonl, valid.jsonl)")
    parser.add_argument("--output-dir", default="fine-tuning/models",
                       help="Output directory for models")
    parser.add_argument("--test", action="store_true",
                       help="Test model after training")

    args = parser.parse_args()

    base_model = args.base_model
    data_path = Path(args.dataset)
    output_base = Path(args.output_dir)

    stages = {
        "1": {
            "name": "tool-use-reinforcement",
            "model": base_model,
            "iterations": 800,
            "lr": 2e-5,
            "description": "Reinforce tool calling patterns with expanded dataset"
        },
        "2": {
            "name": "agentic-skills",
            "model": None,  # Will use stage 1 output
            "iterations": 1000,
            "lr": 1.5e-5,
            "description": "Add TDD, debugging, planning workflows"
        },
        "3": {
            "name": "full-integration",
            "model": None,  # Will use stage 2 output
            "iterations": 1500,
            "lr": 1e-5,
            "description": "Full dataset integration with all skills"
        }
    }

    # Determine which stages to run
    if args.stage == "all":
        stages_to_run = ["1", "2", "3"]
    else:
        stages_to_run = [args.stage]

    # Run training stages
    for stage_num in stages_to_run:
        stage = stages[stage_num]

        # Determine input model (previous stage output or base)
        if stage_num == "1":
            input_model = base_model
        else:
            prev_stage = str(int(stage_num) - 1)
            input_model = str(output_base / f"synthia-stage{prev_stage}")

        output_dir = output_base / f"stage{stage_num}"

        print(f"\n{'#'*60}")
        print(f"# STAGE {stage_num}: {stage['name'].upper()}")
        print(f"# {stage['description']}")
        print(f"{'#'*60}\n")

        # Train
        success = train_stage(
            model_path=input_model,
            data_path=data_path,
            stage_name=stage["name"],
            iterations=stage["iterations"],
            learning_rate=stage["lr"],
            output_dir=output_dir,
        )

        if not success:
            print(f"✗ Stage {stage_num} failed. Stopping.")
            break

        # Fuse weights
        adapter_file = output_dir / "adapters.npz"
        fused_output = output_base / f"synthia-stage{stage_num}"

        success = fuse_lora_weights(
            model_path=input_model,
            adapter_file=adapter_file,
            output_path=fused_output,
        )

        if not success:
            print(f"✗ Fusing failed for stage {stage_num}. Stopping.")
            break

        print(f"\n{'='*60}")
        print(f"✓ STAGE {stage_num} COMPLETE!")
        print(f"  Model saved to: {fused_output}")
        print(f"{'='*60}\n")

        # Test if requested
        if args.test:
            test_model(
                model_path=str(fused_output),
                prompt="Read the file src/main.rs and tell me what it does"
            )

    print(f"\n{'#'*60}")
    print(f"# ALL STAGES COMPLETE!")
    print(f"{'#'*60}\n")
    print(f"Models saved to: {output_base}/")
    print(f"\nTo use the final model:")
    print(f"  mlx_lm.generate --model {output_base}/synthia-stage{stages_to_run[-1]} --prompt 'Your prompt here'")
    print(f"\nTo convert to GGUF for LM Studio:")
    print(f"  # Use llama.cpp's convert.py script")
    print(f"  # Then quantize with llama.cpp")


if __name__ == "__main__":
    main()
