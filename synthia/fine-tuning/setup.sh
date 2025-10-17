#!/bin/bash

# ==============================================================================
# Synthia Fine-Tuning Setup Script for Windows (Git Bash)
# Optimized for RTX 4060 (8GB VRAM)
#
# This script:
# 1. Checks system requirements (Python, CUDA)
# 2. Creates virtual environment
# 3. Installs PyTorch with CUDA 12.1 support
# 4. Installs Unsloth and all dependencies
# 5. Verifies installation
#
# Requirements:
# - Windows 10/11 with Git Bash
# - Python 3.10 or 3.11 (3.12 not fully supported yet)
# - NVIDIA GPU with CUDA support (RTX 4060)
# - ~20GB free disk space
#
# Usage:
#   bash setup.sh
#   or
#   ./setup.sh  (if executable)
# ==============================================================================

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Output functions
print_header() {
    echo -e "\n${CYAN}================================================================================"
    echo -e "  $1"
    echo -e "================================================================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}[OK] $1${NC}"
}

print_error() {
    echo -e "${RED}[ERROR] $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}[WARNING] $1${NC}"
}

print_info() {
    echo -e "${WHITE}[INFO] $1${NC}"
}

# ==============================================================================
# Step 1: Check Python Version
# ==============================================================================

print_header "Checking Python Installation"

if ! command -v python &> /dev/null && ! command -v python3 &> /dev/null; then
    print_error "Python not found in PATH"
    print_info "Please install Python 3.10 or 3.11 from https://www.python.org/downloads/"
    print_info "Make sure to check 'Add Python to PATH' during installation"
    exit 1
fi

# Try python3 first, fall back to python
if command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
else
    PYTHON_CMD="python"
fi

PYTHON_VERSION=$($PYTHON_CMD --version 2>&1)
print_info "Found: $PYTHON_VERSION"

# Extract version numbers using regex
if [[ $PYTHON_VERSION =~ Python[[:space:]]([0-9]+)\.([0-9]+)\.([0-9]+) ]]; then
    MAJOR="${BASH_REMATCH[1]}"
    MINOR="${BASH_REMATCH[2]}"

    if [ "$MAJOR" -ne 3 ]; then
        print_error "Python 3 is required. Found Python $MAJOR"
        exit 1
    fi

    if [ "$MINOR" -lt 10 ] || [ "$MINOR" -gt 11 ]; then
        print_warning "Python 3.10 or 3.11 is recommended. You have Python 3.$MINOR"
        print_warning "Some packages may not work correctly with Python 3.$MINOR"
        read -p "$(echo -e ${WHITE}[INFO] Continue anyway? \(Y/N\): ${NC})" response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            print_info "Please install Python 3.10 or 3.11 from https://www.python.org/downloads/"
            exit 1
        fi
    fi

    print_success "Python version is compatible (3.$MINOR)"
else
    print_error "Could not parse Python version"
    exit 1
fi

# ==============================================================================
# Step 2: Check CUDA Installation
# ==============================================================================

print_header "Checking CUDA Installation"

if ! command -v nvidia-smi &> /dev/null; then
    print_error "NVIDIA GPU driver not found"
    print_info "Please install the latest NVIDIA drivers from:"
    print_info "https://www.nvidia.com/Download/index.aspx"
    exit 1
fi

NVIDIA_SMI_OUTPUT=$(nvidia-smi 2>&1)

if [ $? -eq 0 ]; then
    print_success "NVIDIA GPU detected"

    # Extract GPU name
    if [[ $NVIDIA_SMI_OUTPUT =~ (NVIDIA\ GeForce[^|]*) ]]; then
        GPU_NAME=$(echo "${BASH_REMATCH[1]}" | xargs)
        print_info "GPU: $GPU_NAME"
    fi

    # Extract CUDA version
    if [[ $NVIDIA_SMI_OUTPUT =~ CUDA\ Version:\ ([0-9]+\.[0-9]+) ]]; then
        CUDA_VERSION="${BASH_REMATCH[1]}"
        print_info "CUDA Version: $CUDA_VERSION"

        CUDA_MAJOR=$(echo $CUDA_VERSION | cut -d. -f1)
        if [ "$CUDA_MAJOR" -lt 12 ]; then
            print_warning "CUDA 12.1 or higher is recommended for best performance"
            print_warning "You have CUDA $CUDA_VERSION"
        else
            print_success "CUDA version is compatible"
        fi
    fi
else
    print_error "Failed to run nvidia-smi"
    exit 1
fi

# ==============================================================================
# Step 3: Create Virtual Environment
# ==============================================================================

print_header "Creating Virtual Environment"

VENV_PATH="venv"

if [ -d "$VENV_PATH" ]; then
    print_warning "Virtual environment already exists at: $VENV_PATH"
    read -p "$(echo -e ${WHITE}[INFO] Delete and recreate? \(Y/N\): ${NC})" response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        print_info "Deleting existing virtual environment..."
        rm -rf "$VENV_PATH"
        print_success "Deleted"
    else
        print_info "Using existing virtual environment"
    fi
fi

if [ ! -d "$VENV_PATH" ]; then
    print_info "Creating virtual environment..."
    $PYTHON_CMD -m venv "$VENV_PATH"
    print_success "Virtual environment created at: $VENV_PATH"
fi

# ==============================================================================
# Step 4: Activate Virtual Environment
# ==============================================================================

print_header "Activating Virtual Environment"

ACTIVATE_SCRIPT="$VENV_PATH/Scripts/activate"

if [ ! -f "$ACTIVATE_SCRIPT" ]; then
    print_error "Virtual environment activation script not found at: $ACTIVATE_SCRIPT"
    exit 1
fi

print_info "Activating virtual environment..."
source "$ACTIVATE_SCRIPT"

print_success "Virtual environment activated"

# Verify activation
PYTHON_PATH=$(which python)
if [[ "$PYTHON_PATH" == *"$VENV_PATH"* ]]; then
    print_success "Using virtual environment Python: $PYTHON_PATH"
else
    print_warning "Virtual environment may not be activated correctly"
fi

# ==============================================================================
# Step 5: Upgrade pip
# ==============================================================================

print_header "Upgrading pip"

print_info "Upgrading pip to latest version..."
python -m pip install --upgrade pip

print_success "pip upgraded"

# ==============================================================================
# Step 6: Install PyTorch with CUDA 12.1
# ==============================================================================

print_header "Installing PyTorch with CUDA 12.1"

print_info "This will download ~2GB of packages..."
print_info "Installing PyTorch, torchvision, torchaudio..."

# Install PyTorch with CUDA 12.1 support
python -m pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121

if [ $? -ne 0 ]; then
    print_error "Failed to install PyTorch"
    exit 1
fi

print_success "PyTorch installed"

# ==============================================================================
# Step 7: Verify PyTorch CUDA Support
# ==============================================================================

print_header "Verifying PyTorch CUDA Support"

PYTHON_CHECK="import torch
print(f'PyTorch version: {torch.__version__}')
print(f'CUDA available: {torch.cuda.is_available()}')
if torch.cuda.is_available():
    print(f'CUDA device: {torch.cuda.get_device_name(0)}')
    print(f'CUDA version: {torch.version.cuda}')"

CHECK_RESULT=$(python -c "$PYTHON_CHECK")

print_info "$CHECK_RESULT"

if [[ "$CHECK_RESULT" == *"CUDA available: False"* ]]; then
    print_error "PyTorch CUDA support not working"
    print_info "This could mean:"
    print_info "1. NVIDIA drivers are not installed"
    print_info "2. CUDA toolkit version mismatch"
    print_info "3. GPU not compatible with CUDA"
    exit 1
fi

print_success "PyTorch CUDA support verified"

# ==============================================================================
# Step 8: Install Unsloth and Dependencies
# ==============================================================================

print_header "Installing Unsloth and Dependencies"

print_info "This will download ~3GB of packages..."
print_info "Installing from requirements.txt..."

# Check if requirements.txt exists
if [ ! -f "requirements.txt" ]; then
    print_error "requirements.txt not found"
    print_info "Please create requirements.txt first"
    exit 1
fi

# Install from requirements.txt
python -m pip install -r requirements.txt

if [ $? -ne 0 ]; then
    print_error "Failed to install dependencies"
    exit 1
fi

print_success "All dependencies installed"

# ==============================================================================
# Step 9: Verify Unsloth Installation
# ==============================================================================

print_header "Verifying Unsloth Installation"

UNSLOTH_CHECK="try:
    from unsloth import FastLanguageModel
    print('OK: Unsloth imported successfully')
except Exception as e:
    print(f'ERROR: {e}')"

UNSLOTH_RESULT=$(python -c "$UNSLOTH_CHECK")

print_info "$UNSLOTH_RESULT"

if [[ "$UNSLOTH_RESULT" == *"ERROR"* ]]; then
    print_error "Unsloth installation failed"
    exit 1
fi

print_success "Unsloth installation verified"

# ==============================================================================
# Step 10: Check Dataset
# ==============================================================================

print_header "Checking Dataset"

if [ -f "dataset.jsonl" ]; then
    DATASET_SIZE=$(du -m "dataset.jsonl" | cut -f1)
    print_success "Dataset found: dataset.jsonl (${DATASET_SIZE} MB)"

    # Count lines
    LINE_COUNT=$(wc -l < "dataset.jsonl")
    print_info "Dataset contains $LINE_COUNT examples"
else
    print_warning "dataset.jsonl not found"
    print_info "Please run generate_dataset.py first to create training data"
fi

# ==============================================================================
# Step 11: Summary and Next Steps
# ==============================================================================

print_header "Setup Complete!"

print_success "Environment successfully configured for fine-tuning on RTX 4060"

echo -e "\n${CYAN}System Summary:${NC}"
echo -e "  ${WHITE}- Python: $PYTHON_VERSION${NC}"
echo -e "  ${WHITE}- PyTorch: Installed with CUDA support${NC}"
echo -e "  ${WHITE}- Unsloth: Installed and verified${NC}"
echo -e "  ${WHITE}- Virtual environment: $VENV_PATH${NC}"

echo -e "\n${CYAN}Next Steps:${NC}"
echo -e "  ${WHITE}1. Ensure dataset.jsonl exists (run generate_dataset.py if needed)${NC}"
echo -e "  ${WHITE}2. Review training config in train.py${NC}"
echo -e "  ${WHITE}3. Start training: python train.py${NC}"
echo -e "  ${WHITE}4. Wait ~1-2 hours for training to complete${NC}"
echo -e "  ${WHITE}5. Merge and export: python merge_and_export.py${NC}"
echo -e "  ${WHITE}6. Test the model: python test_model.py${NC}"

echo -e "\n${CYAN}Useful Commands:${NC}"
echo -e "  ${WHITE}- Activate environment: source venv/Scripts/activate${NC}"
echo -e "  ${WHITE}- Deactivate environment: deactivate${NC}"
echo -e "  ${WHITE}- Monitor GPU usage: nvidia-smi${NC}"
echo -e "  ${WHITE}- Watch GPU in real-time: watch -n 1 nvidia-smi${NC}"

echo -e "\n${CYAN}Estimated Requirements:${NC}"
echo -e "  ${WHITE}- VRAM usage: ~6-7GB peak${NC}"
echo -e "  ${WHITE}- Disk space: ~20GB (model + checkpoints)${NC}"
echo -e "  ${WHITE}- Training time: ~1-2 hours on RTX 4060${NC}"

echo ""
print_success "Happy fine-tuning!"
