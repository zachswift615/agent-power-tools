# Synthia Fine-Tuning Setup Guide - WSL2 (Windows)

This guide documents the **complete working setup** for fine-tuning Qwen2.5-Coder on Windows using WSL2.

## Why WSL2?

Native Windows installation has dependency conflicts:
- PyTorch 2.5.1 doesn't have torch.int1 (required by torchao 0.13.0+)
- PyTorch 2.7+ not available for Windows yet
- xformers compatibility issues

**WSL2 works perfectly** because we can use PyTorch nightly builds.

## Prerequisites

- Windows 10/11 (build 19041 or higher)
- At least 25GB free disk space
- NVIDIA GPU with latest drivers installed on Windows
- Administrator access

## Step 1: Install WSL2

### 1.1 Enable WSL2 Features

Open PowerShell as Administrator and run:

```powershell
# Enable WSL and Virtual Machine Platform
dism.exe /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart
dism.exe /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart

# Restart Windows (REQUIRED)
Restart-Computer
```

### 1.2 Set WSL2 as Default

After restart, open PowerShell as Administrator:

```powershell
wsl --set-default-version 2
```

### 1.3 Install Ubuntu 24.04

```powershell
wsl --install -d Ubuntu-24.04
```

**Note:** Windows will restart again. After restart, Ubuntu will automatically open and prompt you to create a username and password.

### 1.4 Verify Installation

In your Ubuntu terminal:

```bash
# Check WSL version (should show version 2)
wsl.exe -l -v

# Check Ubuntu version
lsb_release -a
```

## Step 2: Run Automated Setup Script

The setup script installs everything you need:
- Python 3.11
- NVIDIA CUDA drivers for WSL2
- PyTorch nightly (2.6.0.dev with torch.int1 support)
- Unsloth with all dependencies
- Required patches for compatibility

### 2.1 Navigate to Project Directory

```bash
cd /mnt/c/Users/YOUR_USERNAME/projects/agent-power-tools/synthia/fine-tuning
```

### 2.2 Fix Line Endings (if needed)

```bash
sed -i 's/\r$//' setup-wsl2.sh
chmod +x setup-wsl2.sh
```

### 2.3 Run Setup

```bash
./setup-wsl2.sh
```

This will take 15-30 minutes. You'll be prompted for your sudo password several times.

## Step 3: Verify Installation

After setup completes:

```bash
# Activate environment
source ~/synthia-training/venv/bin/activate

# Check PyTorch version and CUDA
python -c "import torch; print('PyTorch:', torch.__version__); print('CUDA:', torch.cuda.is_available())"

# Check GPU name
python -c "import torch; print('GPU:', torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'N/A')"

# Check Unsloth
python -c "import unsloth; from unsloth import FastLanguageModel; print('Unsloth working!')"
```

Expected output:
- PyTorch: 2.6.0.dev20241112+cu121 (or similar nightly version)
- CUDA: True
- GPU: NVIDIA GeForce RTX 4060 (or your GPU)
- Unsloth working!

## Step 4: Train Your Model

```bash
cd /mnt/c/Users/YOUR_USERNAME/projects/agent-power-tools/synthia/fine-tuning
source ~/synthia-training/venv/bin/activate
python train.py
```

Training will take 1-2 hours on RTX 4060 (8GB VRAM).

## Step 5: Convert to GGUF Format

After training, convert the model to GGUF format for use in LM Studio:

```bash
cd /mnt/c/Users/YOUR_USERNAME/projects/agent-power-tools/synthia/fine-tuning
./convert_to_gguf.sh
```

This creates:
- `model-f16.gguf` (15GB, full precision)
- `model-q4_k_m.gguf` (4.4GB, recommended for LM Studio)

## Step 6: Upload to Hugging Face

```bash
source ~/synthia-training/venv/bin/activate

# Login (need a Write token from https://huggingface.co/settings/tokens)
huggingface-cli login

# Upload
huggingface-cli upload YOUR_USERNAME/qwen2.5-coder-synthia-tool-use \
  outputs/qwen2.5-coder-synthia-merged/model-q4_k_m.gguf \
  model-q4_k_m.gguf \
  --repo-type model
```

## Troubleshooting

### "torch has no attribute 'int1'"

This means PyTorch nightly didn't install correctly. Fix:

```bash
source ~/synthia-training/venv/bin/activate
pip uninstall -y torch torchvision torchaudio
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/nightly/cu121
```

### "nvidia-smi not found"

CUDA drivers not installed. The setup script should handle this, but if needed:

```bash
wget https://developer.download.nvidia.com/compute/cuda/repos/wsl-ubuntu/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt update
sudo apt install -y cuda-toolkit-12-7
```

### Training crashes with OOM (Out of Memory)

Edit `train.py` and reduce:
- `MAX_SEQ_LENGTH = 1024` (was 2048)
- `PER_DEVICE_BATCH_SIZE = 1` (keep at 1)

### "set_stance() got an unexpected keyword argument"

This is fixed by the setup script. If you see this error, apply the patch manually:

```bash
source ~/synthia-training/venv/bin/activate
VISION_PY=~/synthia-training/venv/lib/python3.11/site-packages/unsloth/models/vision.py
sed -i 's/torch_compiler_set_stance(stance = "default", skip_guard_eval_unsafe = False)/torch_compiler_set_stance(stance = "default")/g' $VISION_PY
```

## Quick Reference

### Activate Environment
```bash
source ~/synthia-training/venv/bin/activate
```

### Check GPU
```bash
nvidia-smi
```

### Access Windows Files from WSL2
```bash
cd /mnt/c/Users/YOUR_USERNAME/
```

### Access WSL2 Files from Windows
Open File Explorer and type: `\\wsl$\Ubuntu-24.04\home\YOUR_USERNAME\`

## What Gets Installed

| Component | Version | Purpose |
|-----------|---------|---------|
| Ubuntu | 24.04 LTS | Linux environment |
| Python | 3.11 | Runtime |
| PyTorch | 2.6.0.dev (nightly) | ML framework with torch.int1 |
| CUDA Toolkit | 12.7 | GPU drivers for WSL2 |
| Unsloth | 2025.10.5 | Fast fine-tuning library |
| llama.cpp | Latest | GGUF conversion |

## Key Differences from Windows Native

✅ **Works in WSL2:**
- PyTorch nightly with torch.int1 support
- Full GPU acceleration via WSL2
- All ML libraries work correctly

❌ **Doesn't work on Windows:**
- PyTorch 2.5.1 missing torch.int1
- torchao 0.13.0 incompatible
- xformers version conflicts

## Next Steps

After successful training:
1. Download model from Hugging Face to your MacBook
2. Open LM Studio
3. Load `model-q4_k_m.gguf`
4. Test with tool-calling prompts!

## File Locations

| What | Where (WSL2) | Where (Windows) |
|------|--------------|-----------------|
| Project files | `/mnt/c/Users/YOUR_USERNAME/projects/...` | `C:\Users\YOUR_USERNAME\projects\...` |
| Virtual env | `~/synthia-training/venv` | N/A (WSL2 only) |
| Output model | `outputs/qwen2.5-coder-synthia-merged/` | Same path, accessible from Windows |

## Credits

- Unsloth: https://github.com/unslothai/unsloth
- llama.cpp: https://github.com/ggerganov/llama.cpp
- Qwen2.5-Coder: https://huggingface.co/Qwen/Qwen2.5-Coder-7B
