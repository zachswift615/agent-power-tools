"""
Merge LoRA adapters into base model and export to various formats
Optimized for RTX 4060 (8GB VRAM) on Windows

This script:
1. Loads the base model and fine-tuned LoRA adapters
2. Merges adapters into the base model
3. Exports to multiple formats:
   - 16-bit merged model (for further fine-tuning)
   - GGUF format (for LM Studio, llama.cpp, etc.)
   - Optional 4-bit quantized version

Expected VRAM usage: ~7-8GB peak during merge
Expected time: ~10-20 minutes
"""

import os
import shutil
import torch
from unsloth import FastLanguageModel
from datetime import datetime

# ============================================================================
# CONFIGURATION
# ============================================================================

# Input configuration
MODEL_NAME = "unsloth/qwen2.5-coder-7b-bnb-4bit"  # Base model
LORA_ADAPTER_PATH = "./outputs/qwen2.5-coder-synthia-tool-use"  # Path to LoRA adapters
MAX_SEQ_LENGTH = 2048  # Must match training config

# Output configuration
OUTPUT_BASE = "./outputs/qwen2.5-coder-synthia-merged"
EXPORT_16BIT = True  # Export 16-bit merged model
EXPORT_GGUF = False  # Export GGUF for LM Studio (DISABLED - use convert_to_gguf.sh instead)
EXPORT_4BIT = False  # Export 4-bit quantized (optional, saves disk space)

# GGUF quantization methods (lower bits = smaller file, slightly lower quality)
# Recommended for RTX 4060: Q4_K_M or Q5_K_M
GGUF_QUANTIZATION_METHODS = [
    "q4_k_m",  # 4-bit, medium quality (good balance)
    "q5_k_m",  # 5-bit, medium quality (better quality)
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

def format_size(bytes_value):
    """Format bytes to human-readable size"""
    for unit in ['B', 'KB', 'MB', 'GB']:
        if bytes_value < 1024.0:
            return f"{bytes_value:.2f} {unit}"
        bytes_value /= 1024.0
    return f"{bytes_value:.2f} TB"

def print_gpu_memory():
    """Print current GPU memory usage"""
    if torch.cuda.is_available():
        allocated = torch.cuda.memory_allocated()
        reserved = torch.cuda.memory_reserved()
        print(f"GPU Memory: {format_vram(allocated)} allocated, {format_vram(reserved)} reserved")
    else:
        print("CUDA not available")

def get_directory_size(path):
    """Calculate total size of directory"""
    total = 0
    try:
        for entry in os.scandir(path):
            if entry.is_file():
                total += entry.stat().st_size
            elif entry.is_dir():
                total += get_directory_size(entry.path)
    except Exception as e:
        print(f"Warning: Could not calculate size of {path}: {e}")
    return total

def verify_lora_adapters(adapter_path):
    """Check if LoRA adapters exist"""
    print_separator("Verifying LoRA Adapters")

    if not os.path.exists(adapter_path):
        raise FileNotFoundError(f"LoRA adapter path not found: {adapter_path}")

    # Check for required files
    required_files = ["adapter_model.safetensors", "adapter_config.json"]
    missing_files = []

    for file in required_files:
        file_path = os.path.join(adapter_path, file)
        if not os.path.exists(file_path):
            missing_files.append(file)

    if missing_files:
        raise FileNotFoundError(
            f"Missing required files in {adapter_path}: {missing_files}\n"
            f"Please ensure training completed successfully."
        )

    print(f"✓ LoRA adapters found at: {adapter_path}")

    # Print adapter info
    adapter_size = get_directory_size(adapter_path)
    print(f"✓ Adapter size: {format_size(adapter_size)}")

    return True

def load_model_with_adapters(model_name, adapter_path, max_seq_length):
    """Load base model and apply LoRA adapters"""
    print_separator("Loading Model with Adapters")

    print(f"Loading model with trained LoRA adapters from: {adapter_path}")
    print(f"Note: Loading in 4-bit (as trained), will merge to 16-bit later")

    # Load the model the same way it was trained (4-bit)
    # The merge step will convert it to full precision
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=adapter_path,  # Load from checkpoint
        max_seq_length=max_seq_length,
        dtype=None,  # Auto-detect
        load_in_4bit=True,  # Load in 4-bit like training
    )

    print(f"✓ Model loaded with LoRA adapters (4-bit)")
    print_gpu_memory()

    return model, tokenizer

def merge_and_export_16bit(model, tokenizer, output_path):
    """Merge LoRA adapters and export 16-bit model"""
    print_separator("Exporting 16-bit Merged Model")

    print("Merging LoRA adapters into base model...")
    print("This may take 5-10 minutes...")

    # Merge adapters into base weights
    model = model.merge_and_unload()

    print(f"✓ Adapters merged")
    print_gpu_memory()

    # Save merged model
    print(f"\nSaving to: {output_path}")

    os.makedirs(output_path, exist_ok=True)

    model.save_pretrained(output_path)
    tokenizer.save_pretrained(output_path)

    print(f"✓ 16-bit model saved")

    # Print model size
    model_size = get_directory_size(output_path)
    print(f"✓ Model size: {format_size(model_size)}")

    return model

def export_gguf(model, tokenizer, output_base, quantization_methods):
    """Export model to GGUF format for LM Studio"""
    print_separator("Exporting GGUF Models")

    print("Converting to GGUF format...")
    print("This format works with LM Studio, llama.cpp, and other tools.\n")

    # Create GGUF output directory
    gguf_dir = os.path.join(output_base, "gguf")
    os.makedirs(gguf_dir, exist_ok=True)

    for quant_method in quantization_methods:
        print(f"\n→ Exporting {quant_method.upper()} quantization...")

        try:
            # Export to GGUF with specific quantization
            output_path = os.path.join(gguf_dir, f"model-{quant_method}.gguf")

            # Use Unsloth's GGUF export
            model.save_pretrained_gguf(
                gguf_dir,
                tokenizer,
                quantization_method=quant_method,
            )

            # Find the generated file (Unsloth adds suffix)
            gguf_files = [f for f in os.listdir(gguf_dir) if f.endswith(f"{quant_method}.gguf")]

            if gguf_files:
                gguf_file = os.path.join(gguf_dir, gguf_files[0])
                if os.path.exists(gguf_file):
                    file_size = os.path.getsize(gguf_file)
                    print(f"  ✓ Saved: {gguf_files[0]}")
                    print(f"  ✓ Size: {format_size(file_size)}")
                else:
                    print(f"  ✗ File not found: {gguf_file}")
            else:
                print(f"  ✗ GGUF file not generated")

        except Exception as e:
            print(f"  ✗ Error exporting {quant_method}: {e}")

    print(f"\n✓ GGUF models saved to: {gguf_dir}")

def export_4bit_quantized(model_path, output_path):
    """Export 4-bit quantized model (optional)"""
    print_separator("Exporting 4-bit Quantized Model")

    print("Loading merged model for quantization...")

    # Load the merged 16-bit model
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=model_path,
        max_seq_length=2048,
        dtype=None,
        load_in_4bit=True,  # Quantize to 4-bit
    )

    print(f"✓ Model quantized to 4-bit")
    print_gpu_memory()

    # Save quantized model
    print(f"\nSaving to: {output_path}")
    os.makedirs(output_path, exist_ok=True)

    model.save_pretrained(output_path)
    tokenizer.save_pretrained(output_path)

    print(f"✓ 4-bit model saved")

    # Print model size
    model_size = get_directory_size(output_path)
    print(f"✓ Model size: {format_size(model_size)}")

# ============================================================================
# MAIN EXPORT FUNCTION
# ============================================================================

def main():
    """Main export function"""

    print_separator("Synthia Model Export Script")
    print(f"Start time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"PyTorch version: {torch.__version__}")
    print(f"CUDA available: {torch.cuda.is_available()}")

    if torch.cuda.is_available():
        print(f"CUDA device: {torch.cuda.get_device_name(0)}")
        print(f"Total VRAM: {format_vram(torch.cuda.get_device_properties(0).total_memory)}")

    # Step 1: Verify LoRA adapters exist
    verify_lora_adapters(LORA_ADAPTER_PATH)

    # Step 2: Load model with adapters
    model, tokenizer = load_model_with_adapters(
        model_name=MODEL_NAME,
        adapter_path=LORA_ADAPTER_PATH,
        max_seq_length=MAX_SEQ_LENGTH,
    )

    # Step 3: Export 16-bit merged model
    merged_model = None
    if EXPORT_16BIT:
        merged_16bit_path = os.path.join(OUTPUT_BASE, "16bit")
        merged_model = merge_and_export_16bit(model, tokenizer, merged_16bit_path)

    # Step 4: Export GGUF
    if EXPORT_GGUF:
        if merged_model is None:
            # Need to merge first
            print("\nMerging model for GGUF export...")
            merged_model = model.merge_and_unload()

        export_gguf(merged_model, tokenizer, OUTPUT_BASE, GGUF_QUANTIZATION_METHODS)

    # Step 5: Export 4-bit quantized (optional)
    if EXPORT_4BIT:
        if not EXPORT_16BIT:
            print("\nWarning: Must export 16-bit model first to create 4-bit version")
        else:
            quantized_4bit_path = os.path.join(OUTPUT_BASE, "4bit")
            export_4bit_quantized(merged_16bit_path, quantized_4bit_path)

    # Step 6: Summary
    print_separator("Export Complete")

    print("Summary:")
    print(f"✓ Base model: {MODEL_NAME}")
    print(f"✓ LoRA adapters: {LORA_ADAPTER_PATH}")
    print(f"✓ Output directory: {OUTPUT_BASE}\n")

    if EXPORT_16BIT:
        print(f"✓ 16-bit merged model: {os.path.join(OUTPUT_BASE, '16bit')}")
    if EXPORT_GGUF:
        print(f"✓ GGUF models: {os.path.join(OUTPUT_BASE, 'gguf')}")
    if EXPORT_4BIT:
        print(f"✓ 4-bit quantized: {os.path.join(OUTPUT_BASE, '4bit')}")

    print("\nNext steps:")
    print("1. Test the model with test_model.py")
    print("2. Import GGUF into LM Studio:")
    print(f"   - Open LM Studio")
    print(f"   - Click 'Import Model'")
    print(f"   - Select: {os.path.join(OUTPUT_BASE, 'gguf')}/model-q4_k_m.gguf")
    print("3. Start using your fine-tuned Synthia model!")

    total_size = get_directory_size(OUTPUT_BASE)
    print(f"\nTotal output size: {format_size(total_size)}")
    print(f"End time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")

if __name__ == "__main__":
    main()
