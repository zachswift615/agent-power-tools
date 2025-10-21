# Synthia Fine-Tuning for Windows WSL2

Complete automated pipeline for fine-tuning Qwen2.5-Coder on Windows using WSL2 Ubuntu.

## What This Does

This automated script handles the entire fine-tuning pipeline with **isolated environments** to prevent dependency conflicts:

1. **Training Environment** - PyTorch with CUDA for GPU training
2. **Conversion Environment** - CPU-only tools for GGUF conversion

## Files in This Directory

| File | Purpose |
|------|---------|
| `run-full-pipeline.sh` | Main orchestration script - runs everything |
| `requirements-training.txt` | Training dependencies (PyTorch CUDA, Unsloth) |
| `requirements-conversion.txt` | Conversion dependencies (llama.cpp tools) |
| `README.md` | This file |

## Quick Start

### First Time Setup

1. **Make sure WSL2 is set up** (if not, run `../setup-wsl2-updated.sh` first)

2. **Delete corrupted venv if exists** (from previous failed runs):
   ```bash
   rm -rf ~/synthia-training/venv
   ```

3. **Navigate to fine-tuning directory in WSL2:**
   ```bash
   # Replace with your actual path to the fine-tuning directory
   cd /mnt/c/Users/YOUR_USERNAME/projects/agent-power-tools/synthia/fine-tuning
   ```

4. **Run the full pipeline:**
   ```bash
   ./windows-wsl2/run-full-pipeline.sh
   # Or if you get permission errors:
   bash windows-wsl2/run-full-pipeline.sh
   ```

5. **Go get coffee** ☕ - This takes 1-2 hours total:
   - Training: ~30-60 minutes
   - Merging: ~5 minutes
   - Conversion: ~20-30 minutes

### What the Script Does

```
┌─────────────────────────────────────┐
│  Training Environment (CUDA)        │
│  ├─ Install PyTorch + CUDA          │
│  ├─ Run train.py                    │
│  └─ Run merge_and_export.py         │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  Conversion Environment (CPU)       │
│  ├─ Install llama.cpp tools         │
│  ├─ Convert to F16 GGUF             │
│  ├─ Quantize to Q4_K_M              │
│  └─ Quantize to Q5_K_M              │
└─────────────────────────────────────┘
              ↓
         Final Output:
    ✓ 16-bit merged model
    ✓ F16 GGUF (~15GB)
    ✓ Q4_K_M GGUF (~4.4GB) ← Use this
    ✓ Q5_K_M GGUF (~5GB)
```

## Output Files

After completion, you'll find:

```
synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/
├── 16bit/                    # Merged 16-bit model (~14-15 GB)
│   ├── model-00001-of-00004.safetensors
│   ├── model-00002-of-00004.safetensors
│   ├── model-00003-of-00004.safetensors
│   ├── model-00004-of-00004.safetensors
│   ├── config.json
│   └── tokenizer files...
└── gguf/                     # GGUF files for LM Studio
    ├── model-f16.gguf        # 15GB - Full precision
    ├── model-q4_k_m.gguf     # 4.4GB - Recommended ⭐
    └── model-q5_k_m.gguf     # 5GB - Better quality
```

## Using the Model

### In LM Studio (MacBook)

1. Copy `model-q4_k_m.gguf` to your MacBook
2. Open LM Studio
3. Click "Import Model"
4. Select the GGUF file
5. Start chatting!

### Upload to Hugging Face

```bash
# Install Hugging Face CLI
pip install huggingface-hub

# Login
huggingface-cli login

# Upload
huggingface-cli upload YOUR_USERNAME/qwen2.5-coder-synthia-tool-use \
  outputs/qwen2.5-coder-synthia-merged/gguf/model-q4_k_m.gguf
```

## Troubleshooting

### "PyTorch CUDA not available"

The training environment lost CUDA. Reinstall PyTorch:

```bash
source ~/synthia-training/venv/bin/activate
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/nightly/cu121
```

### "Out of Memory" during merge

Your GPU ran out of VRAM. This shouldn't happen with the script, but if it does:

1. Close other GPU-using applications
2. Try rebooting WSL2: `wsl --shutdown` in Windows PowerShell, then restart

### Script fails at conversion step

The conversion environment might be corrupted. Delete and retry:

```bash
rm -rf ~/synthia-training/venv-conversion
bash windows-wsl2/run-full-pipeline.sh
```

## Environment Locations

- **Training venv**: `~/synthia-training/venv` (PyTorch with CUDA)
- **Conversion venv**: `~/synthia-training/venv-conversion` (CPU-only tools)
- **llama.cpp**: `~/llama.cpp` (conversion tools)

## Advanced: Running Steps Individually

If you need to run steps separately:

```bash
# Step 1: Training only
source ~/synthia-training/venv/bin/activate
export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="/usr/local/cuda/lib64:$LD_LIBRARY_PATH"
cd /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning
python train.py

# Step 2: Merge only
python merge_and_export.py

# Step 3: Convert only
source ~/synthia-training/venv-conversion/bin/activate
cd ~/llama.cpp
python convert_hf_to_gguf.py \
  /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/16bit \
  --outfile /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/gguf/model-f16.gguf \
  --outtype f16

./build/bin/llama-quantize \
  /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/gguf/model-f16.gguf \
  /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/gguf/model-q4_k_m.gguf \
  Q4_K_M
```

## System Requirements

- Windows 10/11 with WSL2
- NVIDIA GPU with 8GB+ VRAM (tested on RTX 4060)
- 25GB free disk space
- Ubuntu 24.04 in WSL2
- Python 3.11

## Credits

- **Unsloth**: https://github.com/unslothai/unsloth
- **llama.cpp**: https://github.com/ggerganov/llama.cpp
- **Qwen2.5-Coder**: https://huggingface.co/Qwen/Qwen2.5-Coder-7B
