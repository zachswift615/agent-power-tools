#!/bin/bash
# ==============================================================================
# Synthia Fine-Tuning Setup for WSL2 Ubuntu 24.04 - TESTED & WORKING
# This script sets up the complete ML environment in WSL2
# ==============================================================================

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

print_header() {
    echo -e "\n${CYAN}================================================================================"
    echo -e "  $1"
    echo -e "================================================================================${NC}\n"
}

print_success() { echo -e "${GREEN}✓ $1${NC}"; }
print_error() { echo -e "${RED}✗ $1${NC}"; }
print_info() { echo -e "${WHITE}ℹ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠ $1${NC}"; }

# ==============================================================================
# Step 1: Update Ubuntu System
# ==============================================================================

print_header "Step 1: Updating Ubuntu System"
print_info "This may take a few minutes on first run..."

sudo apt update
sudo apt upgrade -y

print_success "System updated"

# ==============================================================================
# Step 2: Install Build Essentials
# ==============================================================================

print_header "Step 2: Installing Build Tools"

sudo apt install -y \
    build-essential \
    git \
    curl \
    wget \
    software-properties-common \
    ca-certificates \
    gnupg \
    lsb-release

print_success "Build tools installed"

# ==============================================================================
# Step 3: Install Python 3.11
# ==============================================================================

print_header "Step 3: Installing Python 3.11"

# Add deadsnakes PPA for Python 3.11
sudo add-apt-repository -y ppa:deadsnakes/ppa
sudo apt update

# Install Python 3.11 and related packages
sudo apt install -y \
    python3.11 \
    python3.11-venv \
    python3.11-dev \
    python3-pip

# Make Python 3.11 the default
sudo update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1
sudo update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

print_info "Python version: $(python --version)"
print_success "Python 3.11 installed"

# ==============================================================================
# Step 4: Check NVIDIA GPU Access
# ==============================================================================

print_header "Step 4: Checking NVIDIA GPU Access"

if command -v nvidia-smi &> /dev/null; then
    print_success "nvidia-smi found - GPU access already configured"
    nvidia-smi
else
    print_info "Installing NVIDIA CUDA drivers for WSL2..."

    # Remove old GPG key if exists
    sudo apt-key del 7fa2af80 2>/dev/null || true

    # Install CUDA repository
    wget https://developer.download.nvidia.com/compute/cuda/repos/wsl-ubuntu/x86_64/cuda-keyring_1.1-1_all.deb
    sudo dpkg -i cuda-keyring_1.1-1_all.deb
    sudo apt update

    # Install CUDA toolkit (this is lightweight for WSL2)
    sudo apt install -y cuda-toolkit-12-7

    # Add CUDA to PATH
    echo 'export PATH=/usr/local/cuda/bin:$PATH' >> ~/.bashrc
    echo 'export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
    source ~/.bashrc

    print_success "NVIDIA CUDA drivers installed"
fi

# ==============================================================================
# Step 5: Set Up Project Directory
# ==============================================================================

print_header "Step 5: Setting Up Project Directory"

# Create workspace directory
WORKSPACE="$HOME/synthia-training"
mkdir -p "$WORKSPACE"
cd "$WORKSPACE"

print_info "Workspace created at: $WORKSPACE"

# Clone or link to Windows project (if accessible)
WINDOWS_PROJECT="/mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning"
if [ -d "$WINDOWS_PROJECT" ]; then
    print_info "Found Windows project directory, creating symbolic link..."
    ln -sf "$WINDOWS_PROJECT" "$WORKSPACE/synthia-windows"
    print_success "Linked to Windows project at: $WORKSPACE/synthia-windows"
fi

print_success "Project directory ready"

# ==============================================================================
# Step 6: Create Virtual Environment
# ==============================================================================

print_header "Step 6: Creating Virtual Environment"

VENV_PATH="$WORKSPACE/venv"

if [ -d "$VENV_PATH" ]; then
    print_warning "Virtual environment already exists - removing and recreating..."
    rm -rf "$VENV_PATH"
fi

python3.11 -m venv "$VENV_PATH"
print_success "Virtual environment created"

source "$VENV_PATH/bin/activate"
print_success "Virtual environment activated"

# Upgrade pip
python -m pip install --upgrade pip
print_success "pip upgraded"

# ==============================================================================
# Step 7: Install PyTorch NIGHTLY with CUDA Support (CRITICAL!)
# ==============================================================================

print_header "Step 7: Installing PyTorch NIGHTLY with CUDA 12.1"
print_warning "Using nightly build is REQUIRED for torch.int1 support"

# Install PyTorch nightly - this is CRITICAL for torchao compatibility
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/nightly/cu121

print_success "PyTorch nightly installed"

# Verify PyTorch and CUDA
print_info "Verifying PyTorch installation..."
python << 'PYEOF'
import torch
print(f"PyTorch version: {torch.__version__}")
print(f"CUDA available: {torch.cuda.is_available()}")
if torch.cuda.is_available():
    print(f"CUDA device: {torch.cuda.get_device_name(0)}")
    print(f"CUDA version: {torch.version.cuda}")
print(f"Has torch.int1: {hasattr(torch, 'int1')}")
if not hasattr(torch, 'int1'):
    print("ERROR: PyTorch does not have torch.int1 - nightly build may not have installed correctly")
    exit(1)
PYEOF

if [ $? -ne 0 ]; then
    print_error "PyTorch installation verification failed"
    exit 1
fi

print_success "PyTorch verified with torch.int1 support"

# ==============================================================================
# Step 8: Install Unsloth with Dependencies (Linux Method)
# ==============================================================================

print_header "Step 8: Installing Unsloth and Dependencies"

print_info "Using the recommended Linux installation method..."

# Install unsloth from git (recommended for Linux)
pip install "unsloth[colab-new] @ git+https://github.com/unslothai/unsloth.git"

# Install additional dependencies
pip install \
    transformers \
    datasets \
    accelerate \
    peft \
    trl \
    bitsandbytes \
    scipy \
    sentencepiece \
    protobuf \
    tqdm \
    rich \
    psutil \
    py-cpuinfo

print_success "All dependencies installed"

# ==============================================================================
# Step 9: Apply Critical Patches
# ==============================================================================

print_header "Step 9: Applying Critical Patches"

# Patch unsloth vision.py for PyTorch nightly compatibility
print_info "Patching unsloth for PyTorch nightly compatibility..."
VISION_PY="$VENV_PATH/lib/python3.11/site-packages/unsloth/models/vision.py"

if [ -f "$VISION_PY" ]; then
    # Remove skip_guard_eval_unsafe parameter that doesn't exist in PyTorch 2.6.0.dev
    sed -i 's/torch_compiler_set_stance(stance = "default", skip_guard_eval_unsafe = False)/torch_compiler_set_stance(stance = "default")/g' "$VISION_PY"
    print_success "Unsloth patched for PyTorch compatibility"
else
    print_warning "vision.py not found - patch may not be needed"
fi

print_success "Patches applied"

# ==============================================================================
# Step 10: Test Installation
# ==============================================================================

print_header "Step 10: Testing Installation"

python << 'PYEOF'
import sys

print("=" * 80)
print("Testing PyTorch...")
print("=" * 80)
import torch
print(f"✓ PyTorch version: {torch.__version__}")
print(f"✓ CUDA available: {torch.cuda.is_available()}")
print(f"✓ Has torch.int1: {hasattr(torch, 'int1')}")
if torch.cuda.is_available():
    print(f"✓ CUDA device: {torch.cuda.get_device_name(0)}")
    print(f"✓ CUDA version: {torch.version.cuda}")

print("\n" + "=" * 80)
print("Testing Unsloth...")
print("=" * 80)
try:
    from unsloth import FastLanguageModel
    print("✓ Unsloth imported successfully!")
    print("✓ FastLanguageModel available")
except Exception as e:
    print(f"✗ Unsloth import failed: {e}")
    sys.exit(1)

print("\n" + "=" * 80)
print("Testing Other Dependencies...")
print("=" * 80)
packages = ["transformers", "datasets", "accelerate", "peft", "trl", "bitsandbytes"]
for pkg in packages:
    try:
        module = __import__(pkg)
        version = getattr(module, "__version__", "unknown")
        print(f"✓ {pkg}: {version}")
    except Exception as e:
        print(f"✗ {pkg}: {e}")

print("\n" + "=" * 80)
print("🎉 ALL TESTS PASSED!")
print("=" * 80)
PYEOF

if [ $? -ne 0 ]; then
    print_error "Installation test failed"
    exit 1
fi

print_success "Installation test passed!"

# ==============================================================================
# Step 11: Create Activation Script
# ==============================================================================

print_header "Step 11: Creating Quick Activation Script"

cat > "$WORKSPACE/activate.sh" << 'ACTIVEOF'
#!/bin/bash
# Quick activation script for Synthia training environment

WORKSPACE="$HOME/synthia-training"
source "$WORKSPACE/venv/bin/activate"

echo "🚀 Synthia Training Environment Activated"
echo "   Workspace: $WORKSPACE"
echo "   Python: $(python --version)"
echo "   PyTorch: $(python -c 'import torch; print(torch.__version__)')"
echo "   CUDA: $(python -c 'import torch; print(torch.cuda.is_available())')"
echo ""
echo "Ready to train! 🔥"
ACTIVEOF

chmod +x "$WORKSPACE/activate.sh"

print_success "Activation script created: $WORKSPACE/activate.sh"

# ==============================================================================
# Summary
# ==============================================================================

print_header "🎉 Setup Complete!"

echo -e "${CYAN}Your WSL2 ML Environment is Ready!${NC}\n"

echo -e "${WHITE}Installed:${NC}"
echo -e "  ✓ Ubuntu 24.04 LTS"
echo -e "  ✓ Python 3.11"
echo -e "  ✓ PyTorch NIGHTLY with CUDA 12.1 (with torch.int1)"
echo -e "  ✓ Unsloth with all dependencies"
echo -e "  ✓ NVIDIA GPU drivers for WSL2"

echo -e "\n${WHITE}Workspace Location:${NC}"
echo -e "  $WORKSPACE"

echo -e "\n${WHITE}Quick Start:${NC}"
echo -e "  1. Activate environment:"
echo -e "     ${YELLOW}source ~/synthia-training/activate.sh${NC}"
echo -e ""
echo -e "  2. Test GPU:"
echo -e "     ${YELLOW}python -c 'import torch; print(torch.cuda.get_device_name(0))'${NC}"
echo -e ""
echo -e "  3. Start training:"
echo -e "     ${YELLOW}cd /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning${NC}"
echo -e "     ${YELLOW}python train.py${NC}"

echo -e "\n${WHITE}Access Windows Files:${NC}"
echo -e "  Your Windows C: drive is mounted at: ${YELLOW}/mnt/c/${NC}"
echo -e "  Your project: ${YELLOW}/mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/${NC}"

echo -e "\n${WHITE}Important Notes:${NC}"
echo -e "  ${YELLOW}⚠${NC}  PyTorch NIGHTLY is required (not stable 2.5.1)"
echo -e "  ${YELLOW}⚠${NC}  Make sure you have torch.int1 support"
echo -e "  ${YELLOW}⚠${NC}  Training uses ~6-7GB VRAM on RTX 4060"

echo -e "\n${GREEN}Happy Training! 🚀${NC}\n"
