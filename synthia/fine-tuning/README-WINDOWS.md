# Synthia Fine-Tuning on Windows - Complete Guide

This directory contains everything you need to fine-tune Qwen2.5-Coder for tool-use on Windows.

## TL;DR - Quick Start

1. **Install WSL2** (see [SETUP_WSL2.md](SETUP_WSL2.md))
2. **Run setup**: `./setup-wsl2-updated.sh`
3. **Train model**: `python train.py`
4. **Convert to GGUF**: `./convert_to_gguf.sh`
5. **Upload to HuggingFace**: Use `huggingface-cli`

## What's Included

| File | Description |
|------|-------------|
| `SETUP_WSL2.md` | Complete WSL2 setup instructions |
| `setup-wsl2-updated.sh` | Automated setup script (TESTED & WORKING) |
| `train.py` | Training script (fixed for Qwen chat templates) |
| `convert_to_gguf.sh` | GGUF conversion script |
| `dataset.jsonl` | Training data (250 tool-use examples) |

## Why This Approach Works

### Windows Native (❌ Doesn't Work)
- PyTorch 2.5.1 missing `torch.int1`
- torchao 0.13.0 requires torch.int1
- xformers version conflicts
- No PyTorch 2.7+ for Windows yet

### WSL2 (✅ Works Perfect)
- Ubuntu 24.04 LTS in WSL2
- PyTorch **nightly** (2.6.0.dev) with torch.int1
- Full NVIDIA GPU acceleration
- All dependencies work correctly

## System Requirements

- **OS**: Windows 10/11 (build 19041+)
- **GPU**: NVIDIA with 8GB+ VRAM (tested on RTX 4060)
- **Disk**: 25GB free space
- **RAM**: 16GB+ recommended
- **Internet**: Fast connection for downloads

## Installation Time

| Step | Time |
|------|------|
| WSL2 setup | 10-15 min |
| Automated setup script | 15-30 min |
| **Total** | **25-45 min** |

## Training Time

| Hardware | Time |
|----------|------|
| RTX 4060 (8GB) | 1-2 hours |
| RTX 3090 (24GB) | 30-45 min |

## What Gets Trained

- **Base Model**: Qwen2.5-Coder-7B (pre-quantized 4-bit)
- **Method**: QLoRA (4-bit quantization + LoRA adapters)
- **Data**: 250 examples of tool usage patterns
- **Output**: Fine-tuned model specialized in tool calling

## Key Fixes Applied

### 1. PyTorch Nightly (CRITICAL)
```bash
# This is required - not the stable version!
pip install --pre torch --index-url https://download.pytorch.org/whl/nightly/cu121
```

### 2. Unsloth Patch
```bash
# Removes incompatible parameter from vision.py
sed -i 's/skip_guard_eval_unsafe = False//g' .../unsloth/models/vision.py
```

### 3. Train.py Fixes
- Uses `unsloth.chat_templates.get_chat_template()` instead of manual template
- Removed deprecated `evaluation_strategy` parameter
- Fixed for Qwen2.5 model format

### 4. GGUF Conversion
- Uses llama.cpp with CMake build (not old Makefile)
- Converts to F16 first, then quantizes to Q4_K_M
- Creates 4.4GB file from 15GB F16 model

## Output Files

After successful training:

```
outputs/qwen2.5-coder-synthia-merged/
├── model-f16.gguf        # 15GB - Full precision
├── model-q4_k_m.gguf     # 4.4GB - Recommended for LM Studio
└── gguf/                 # Original merged safetensors
    ├── config.json
    ├── model-00001-of-00004.safetensors
    ├── model-00002-of-00004.safetensors
    ├── model-00003-of-00004.safetensors
    └── model-00004-of-00004.safetensors
```

## Using the Model

### Option 1: LM Studio (MacBook)

1. Upload to Hugging Face:
   ```bash
   huggingface-cli upload YOUR_USERNAME/qwen2.5-coder-synthia-tool-use \
     outputs/qwen2.5-coder-synthia-merged/model-q4_k_m.gguf
   ```

2. On MacBook:
   - Open LM Studio
   - Search: `YOUR_USERNAME/qwen2.5-coder-synthia-tool-use`
   - Download and load

### Option 2: Direct Transfer

Copy `model-q4_k_m.gguf` to your MacBook and import into LM Studio.

## Training Configuration

Optimized for RTX 4060 (8GB VRAM):

| Setting | Value | Notes |
|---------|-------|-------|
| Sequence Length | 2048 | Reduce to 1024 if OOM |
| Batch Size | 1 | Per device |
| Gradient Accumulation | 8 | Effective batch = 8 |
| LoRA Rank | 16 | Balance of quality/speed |
| Quantization | 4-bit | ~75% VRAM savings |
| Optimizer | adamw_8bit | ~50% optimizer memory savings |

## Troubleshooting

### "torch has no attribute 'int1'"
PyTorch nightly didn't install. Reinstall:
```bash
pip uninstall -y torch torchvision torchaudio
pip install --pre torch --index-url https://download.pytorch.org/whl/nightly/cu121
```

### "nvidia-smi: command not found"
CUDA drivers not installed in WSL2. Run setup script again or install manually.

### OOM (Out of Memory) during training
Reduce MAX_SEQ_LENGTH in train.py from 2048 to 1024.

### "set_stance() got an unexpected keyword argument"
Unsloth patch not applied. Run:
```bash
sed -i 's/skip_guard_eval_unsafe = False//g' ~/synthia-training/venv/lib/python3.11/site-packages/unsloth/models/vision.py
```

## Performance Metrics

**Training on RTX 4060:**
- VRAM Usage: ~6-7GB peak
- Speed: ~0.5 iterations/sec
- Total Time: ~1.5 hours for 1 epoch

**GGUF Conversion:**
- F16 conversion: ~10 minutes
- Q4_K_M quantization: ~3 minutes
- Total: ~15 minutes

## Directory Structure

```
synthia/fine-tuning/
├── README-WINDOWS.md              ← This file
├── SETUP_WSL2.md                  ← Detailed setup guide
├── setup-wsl2-updated.sh          ← Automated setup (USE THIS)
├── setup-wsl2.sh                  ← Old version (don't use)
├── convert_to_gguf.sh             ← GGUF conversion
├── train.py                       ← Training script
├── dataset.jsonl                  ← Training data
└── outputs/                       ← Generated models
```

## What You Learned

This process covered:
- ✅ WSL2 setup and NVIDIA GPU passthrough
- ✅ PyTorch nightly builds and compatibility
- ✅ QLoRA fine-tuning with memory optimization
- ✅ Model format conversion (safetensors → GGUF)
- ✅ Quantization techniques (FP16 → Q4_K_M)
- ✅ Dependency resolution in ML environments

## Credits & Resources

- **Unsloth**: https://github.com/unslothai/unsloth
- **llama.cpp**: https://github.com/ggerganov/llama.cpp
- **Qwen2.5**: https://huggingface.co/Qwen/Qwen2.5-Coder-7B
- **WSL2 Docs**: https://learn.microsoft.com/en-us/windows/wsl/

## License

Follow the licenses of the respective components:
- Qwen2.5-Coder: Apache 2.0
- Unsloth: Apache 2.0
- Your fine-tuned model: Your choice

## Support

For issues:
1. Check `SETUP_WSL2.md` troubleshooting section
2. Verify PyTorch has torch.int1: `python -c "import torch; print(hasattr(torch, 'int1'))"`
3. Check CUDA: `nvidia-smi`
4. Review this README

Happy fine-tuning! 🚀
