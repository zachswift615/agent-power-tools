"""
Quick validation script to check if setup is ready for training
Runs basic checks without requiring GPU or full environment
"""

import os
import sys
import json

def print_header(title):
    print(f"\n{'='*60}")
    print(f"  {title}")
    print(f"{'='*60}\n")

def check_file_exists(path, description):
    """Check if a file exists"""
    if os.path.exists(path):
        size = os.path.getsize(path)
        size_str = f"{size:,} bytes"
        if size > 1024*1024:
            size_str = f"{size/(1024*1024):.2f} MB"
        elif size > 1024:
            size_str = f"{size/1024:.2f} KB"
        print(f"✓ {description}: {size_str}")
        return True
    else:
        print(f"✗ {description}: NOT FOUND")
        return False

def validate_dataset(path):
    """Validate dataset format"""
    print_header("Validating Dataset")

    if not check_file_exists(path, "dataset.jsonl"):
        return False

    try:
        with open(path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        print(f"  - Total examples: {len(lines)}")

        # Parse first example
        first_example = json.loads(lines[0])

        if 'messages' not in first_example:
            print("✗ Dataset missing 'messages' field")
            return False

        print(f"  - Format: ChatML with {len(first_example['messages'])} messages per example")

        # Count tool calls
        tool_calls = 0
        for line in lines:
            example = json.loads(line)
            for msg in example['messages']:
                if msg.get('role') == 'assistant' and 'tool_calls' in msg:
                    tool_calls += len(msg['tool_calls'])

        print(f"  - Total tool calls: {tool_calls}")
        print("✓ Dataset format is valid")
        return True

    except Exception as e:
        print(f"✗ Error validating dataset: {e}")
        return False

def validate_scripts():
    """Check if all required scripts exist"""
    print_header("Validating Scripts")

    scripts = [
        ("train.py", "Main training script"),
        ("merge_and_export.py", "Model export script"),
        ("test_model.py", "Inference test script"),
        ("requirements.txt", "Python dependencies"),
        ("setup.ps1", "Windows setup script"),
    ]

    all_exist = True
    for filename, description in scripts:
        if not check_file_exists(filename, description):
            all_exist = False

    return all_exist

def check_python_imports():
    """Try importing key packages (if installed)"""
    print_header("Checking Python Environment")

    packages = [
        ("torch", "PyTorch"),
        ("transformers", "Hugging Face Transformers"),
        ("datasets", "Hugging Face Datasets"),
        ("unsloth", "Unsloth"),
        ("peft", "PEFT"),
        ("trl", "TRL"),
    ]

    installed = []
    missing = []

    for module, name in packages:
        try:
            __import__(module)
            installed.append(name)
            print(f"✓ {name} installed")
        except ImportError:
            missing.append(name)
            print(f"✗ {name} not installed")

    if missing:
        print(f"\n⚠ Missing packages: {', '.join(missing)}")
        print("  Run: pip install -r requirements.txt")
    else:
        print("\n✓ All required packages are installed")

    return len(missing) == 0

def check_cuda():
    """Check if CUDA is available"""
    print_header("Checking CUDA Support")

    try:
        import torch
        cuda_available = torch.cuda.is_available()

        if cuda_available:
            print(f"✓ CUDA is available")
            print(f"  - Device: {torch.cuda.get_device_name(0)}")
            print(f"  - CUDA version: {torch.version.cuda}")
            total_memory = torch.cuda.get_device_properties(0).total_memory
            print(f"  - Total VRAM: {total_memory / (1024**3):.2f} GB")

            if total_memory < 8 * 1024**3:
                print(f"⚠ Warning: Less than 8GB VRAM detected")
                print(f"  Consider reducing MAX_SEQ_LENGTH or LORA_RANK in train.py")

            return True
        else:
            print("✗ CUDA not available")
            print("  Training will be VERY slow on CPU")
            print("  Please install CUDA-enabled PyTorch:")
            print("  pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121")
            return False

    except ImportError:
        print("✗ PyTorch not installed")
        print("  Run: pip install -r requirements.txt")
        return False

def main():
    """Run all validation checks"""
    print_header("Synthia Fine-Tuning Setup Validator")

    print("This script checks if your environment is ready for training.")
    print("Note: Some checks require packages to be installed.\n")

    results = []

    # Check dataset
    results.append(("Dataset", validate_dataset("dataset.jsonl")))

    # Check scripts
    results.append(("Scripts", validate_scripts()))

    # Check Python environment (optional, won't fail if not installed)
    try:
        env_ok = check_python_imports()
        results.append(("Python Environment", env_ok))

        # Check CUDA (optional)
        cuda_ok = check_cuda()
        results.append(("CUDA Support", cuda_ok))
    except Exception as e:
        print(f"\n⚠ Could not check Python environment: {e}")
        print("  This is OK if you haven't run setup.ps1 yet")

    # Print summary
    print_header("Validation Summary")

    passed = sum(1 for _, ok in results if ok)
    total = len(results)

    for name, ok in results:
        status = "✓ PASS" if ok else "✗ FAIL"
        print(f"  {status}: {name}")

    print(f"\nOverall: {passed}/{total} checks passed")

    if passed == total:
        print("\n🎉 All checks passed! You're ready to start training.")
        print("\nNext steps:")
        print("  1. Activate virtual environment: .\\venv\\Scripts\\Activate.ps1")
        print("  2. Start training: python train.py")
        return 0
    else:
        print("\n⚠ Some checks failed. Please fix the issues above.")
        if not any(name == "Python Environment" and ok for name, ok in results):
            print("\nIf you haven't set up the environment yet:")
            print("  1. Run: .\\setup.ps1")
            print("  2. Then run this validator again")
        return 1

if __name__ == "__main__":
    sys.exit(main())
