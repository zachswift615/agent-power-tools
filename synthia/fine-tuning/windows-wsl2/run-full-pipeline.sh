#!/bin/bash
# =============================================================================
# Synthia Fine-Tuning Complete Pipeline
# Handles training, merging, and conversion with isolated environments
# =============================================================================

set -e  # Exit on error

# Color codes
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

# =============================================================================
# Configuration
# =============================================================================

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
FINE_TUNING_DIR="$(dirname "$SCRIPT_DIR")"  # Parent directory (fine-tuning)
PROJECT_ROOT="$HOME/synthia-training"
TRAINING_VENV="$PROJECT_ROOT/venv"
CONVERSION_VENV="$PROJECT_ROOT/venv-conversion"
OUTPUT_BASE="$FINE_TUNING_DIR/outputs/qwen2.5-coder-synthia-merged"

# =============================================================================
# Cleanup Check
# =============================================================================

if [ -d "$OUTPUT_BASE" ]; then
    print_header "Cleanup Check"
    print_warning "Existing output directory found: $OUTPUT_BASE"

    # Show what will be deleted
    OUTPUT_SIZE=$(du -sh "$OUTPUT_BASE" 2>/dev/null | cut -f1 || echo "unknown")
    print_info "Current size: $OUTPUT_SIZE"

    echo -e "${WHITE}This directory will be deleted to ensure a clean merge.${NC}"
    echo -e "${YELLOW}Do you want to delete it and continue? (y/N):${NC} "
    read -r response

    if [[ "$response" =~ ^[Yy]$ ]]; then
        print_info "Deleting existing output directory..."
        rm -rf "$OUTPUT_BASE"
        print_success "Output directory cleaned"
    else
        print_error "User chose not to delete existing outputs"
        print_info "Pipeline cancelled. Please manually clean or backup:"
        print_info "  $OUTPUT_BASE"
        exit 0
    fi
fi

# =============================================================================
# Step 1: Setup Training Environment
# =============================================================================

print_header "Step 1: Setting Up Training Environment"

if [ ! -d "$TRAINING_VENV" ]; then
    print_info "Creating training virtual environment..."
    python3.11 -m venv "$TRAINING_VENV"
    print_success "Training venv created"
else
    print_info "Training venv already exists"
fi

print_info "Activating training environment..."
source "$TRAINING_VENV/bin/activate"

# Add CUDA paths to current session
export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="/usr/local/cuda/lib64:$LD_LIBRARY_PATH"

print_info "Installing/updating training dependencies..."
pip install -q --upgrade pip

# Install PyTorch nightly first (from special index)
print_info "Installing PyTorch nightly with CUDA 12.1..."
pip install -q --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/nightly/cu121

# Pin triton to compatible version (fixes AttrsDescriptor import error)
print_info "Installing compatible triton version..."
pip install -q "triton>=3.0.0,<3.2.0"

# Then install other dependencies (from PyPI)
print_info "Installing Unsloth and other dependencies..."
pip install -q "unsloth[colab-new] @ git+https://github.com/unslothai/unsloth.git"
pip install -q transformers datasets accelerate peft trl bitsandbytes scipy sentencepiece protobuf tqdm rich psutil py-cpuinfo

print_success "Training environment ready"

# Apply Unsloth patch for PyTorch nightly compatibility (if needed)
print_info "Checking if Unsloth patch is needed..."
VISION_PY="$TRAINING_VENV/lib/python3.11/site-packages/unsloth/models/vision.py"
if [ -f "$VISION_PY" ]; then
    # Check if the problematic parameter exists
    if grep -q "skip_guard_eval_unsafe = False" "$VISION_PY"; then
        print_info "Applying Unsloth compatibility patch..."
        sed -i 's/torch_compiler_set_stance(stance = "default", skip_guard_eval_unsafe = False)/torch_compiler_set_stance(stance = "default")/g' "$VISION_PY"
        print_success "Unsloth patched for PyTorch nightly compatibility"
    else
        print_success "Unsloth patch not needed (already compatible)"
    fi
else
    print_info "vision.py not found - skipping patch"
fi

# Verify PyTorch CUDA
CUDA_CHECK=$(python -c "import torch; print(torch.cuda.is_available())" 2>/dev/null || echo "False")
if [ "$CUDA_CHECK" != "True" ]; then
    print_error "PyTorch CUDA not available!"
    print_info "Run: pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/nightly/cu121"
    exit 1
fi
print_success "PyTorch CUDA verified"

# =============================================================================
# Step 2: Run Training
# =============================================================================

print_header "Step 2: Running Fine-Tuning"

cd "$FINE_TUNING_DIR"

print_info "Starting training (this takes ~30-60 minutes)..."
python train.py

if [ $? -ne 0 ]; then
    print_error "Training failed!"
    exit 1
fi

print_success "Training completed"

# =============================================================================
# Step 3: Merge LoRA Adapters
# =============================================================================

print_header "Step 3: Merging LoRA Adapters"

print_info "Merging adapters into base model..."
python merge_and_export.py

if [ $? -ne 0 ]; then
    print_error "Merge failed!"
    exit 1
fi

print_success "Merge completed"

# Verify 16bit model exists
if [ ! -d "$OUTPUT_BASE/16bit" ]; then
    print_error "16-bit model directory not found!"
    exit 1
fi

# Check actual model files exist (not just config)
MODEL_SIZE=$(du -sh "$OUTPUT_BASE/16bit" | cut -f1)
print_info "16-bit model size: $MODEL_SIZE"

# Deactivate training venv
deactivate

# =============================================================================
# Step 4: Setup Conversion Environment
# =============================================================================

print_header "Step 4: Setting Up Conversion Environment"

if [ ! -d "$CONVERSION_VENV" ]; then
    print_info "Creating conversion virtual environment..."
    python3.11 -m venv "$CONVERSION_VENV"
    print_success "Conversion venv created"
else
    print_info "Conversion venv already exists"
fi

print_info "Activating conversion environment..."
source "$CONVERSION_VENV/bin/activate"

print_info "Installing/updating conversion dependencies..."
pip install -q --upgrade pip
pip install -q -r "$SCRIPT_DIR/requirements-conversion.txt"

# Setup llama.cpp if needed
LLAMA_CPP_DIR="$HOME/llama.cpp"
if [ ! -d "$LLAMA_CPP_DIR" ]; then
    print_info "Cloning llama.cpp..."
    git clone https://github.com/ggerganov/llama.cpp "$LLAMA_CPP_DIR"
fi

cd "$LLAMA_CPP_DIR"

# Install llama.cpp Python requirements (CPU-only torch)
pip install -q -r requirements.txt

# Build llama.cpp if needed
if [ ! -f "build/bin/llama-quantize" ]; then
    print_info "Building llama.cpp (this takes 5-10 minutes)..."
    cmake -B build
    cmake --build build --config Release -j$(nproc)
    print_success "llama.cpp built"
else
    print_info "llama.cpp already built"
fi

print_success "Conversion environment ready"

# =============================================================================
# Step 5: Convert to GGUF
# =============================================================================

print_header "Step 5: Converting to GGUF Format"

GGUF_OUTPUT_DIR="$OUTPUT_BASE/gguf"
mkdir -p "$GGUF_OUTPUT_DIR"

MODEL_INPUT="$OUTPUT_BASE/16bit"
F16_OUTPUT="$GGUF_OUTPUT_DIR/model-f16.gguf"
Q4_OUTPUT="$GGUF_OUTPUT_DIR/model-q4_k_m.gguf"
Q5_OUTPUT="$GGUF_OUTPUT_DIR/model-q5_k_m.gguf"

# Step 5a: Convert to F16 GGUF
print_info "Converting to F16 GGUF (this takes 10-15 minutes)..."
cd "$LLAMA_CPP_DIR"
python convert_hf_to_gguf.py "$MODEL_INPUT" --outfile "$F16_OUTPUT" --outtype f16

if [ ! -f "$F16_OUTPUT" ]; then
    print_error "F16 GGUF conversion failed!"
    exit 1
fi

F16_SIZE=$(du -sh "$F16_OUTPUT" | cut -f1)
print_success "F16 GGUF created: $F16_SIZE"

# Step 5b: Quantize to Q4_K_M
print_info "Quantizing to Q4_K_M (this takes 5-10 minutes)..."
./build/bin/llama-quantize "$F16_OUTPUT" "$Q4_OUTPUT" Q4_K_M

if [ ! -f "$Q4_OUTPUT" ]; then
    print_error "Q4_K_M quantization failed!"
    exit 1
fi

Q4_SIZE=$(du -sh "$Q4_OUTPUT" | cut -f1)
print_success "Q4_K_M GGUF created: $Q4_SIZE"

# Step 5c: Quantize to Q5_K_M (optional, better quality)
print_info "Quantizing to Q5_K_M (this takes 5-10 minutes)..."
./build/bin/llama-quantize "$F16_OUTPUT" "$Q5_OUTPUT" Q5_K_M

if [ ! -f "$Q5_OUTPUT" ]; then
    print_warning "Q5_K_M quantization failed (optional)"
else
    Q5_SIZE=$(du -sh "$Q5_OUTPUT" | cut -f1)
    print_success "Q5_K_M GGUF created: $Q5_SIZE"
fi

# =============================================================================
# Step 6: Summary
# =============================================================================

print_header "Pipeline Complete! 🎉"

echo -e "${WHITE}Output Files:${NC}\n"

echo -e "${CYAN}1. 16-bit Merged Model (for further training):${NC}"
echo -e "   ${WHITE}Location: $OUTPUT_BASE/16bit/${NC}"
echo -e "   ${WHITE}Size: $MODEL_SIZE${NC}"
echo -e ""

echo -e "${CYAN}2. F16 GGUF (full precision):${NC}"
echo -e "   ${WHITE}Location: $F16_OUTPUT${NC}"
echo -e "   ${WHITE}Size: $F16_SIZE${NC}"
echo -e ""

echo -e "${CYAN}3. Q4_K_M GGUF (recommended for LM Studio):${NC}"
echo -e "   ${WHITE}Location: $Q4_OUTPUT${NC}"
echo -e "   ${WHITE}Size: $Q4_SIZE${NC}"
echo -e ""

if [ -f "$Q5_OUTPUT" ]; then
    echo -e "${CYAN}4. Q5_K_M GGUF (better quality, larger file):${NC}"
    echo -e "   ${WHITE}Location: $Q5_OUTPUT${NC}"
    echo -e "   ${WHITE}Size: $Q5_SIZE${NC}"
    echo -e ""
fi

echo -e "${WHITE}Next Steps:${NC}"
echo -e "  1. Test the model: python test_model.py"
echo -e "  2. Import Q4_K_M into LM Studio:"
echo -e "     ${YELLOW}$Q4_OUTPUT${NC}"
echo -e "  3. Or upload to Hugging Face for sharing"
echo -e ""

deactivate
print_success "All environments deactivated"
