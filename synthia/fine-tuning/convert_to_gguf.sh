#!/bin/bash
set -e  # Exit on error

echo "========================================"
echo "Converting Model to GGUF Format"
echo "========================================"
echo ""

# Activate virtual environment
source ~/synthia-training/venv/bin/activate

# Step 1: Install cmake if needed
echo "Step 1: Checking for cmake..."
if ! command -v cmake &> /dev/null; then
    echo "Installing cmake..."
    sudo apt-get update
    sudo apt-get install -y cmake
fi
echo "✓ cmake available"

# Step 2: Clone/update llama.cpp
echo ""
echo "Step 2: Setting up llama.cpp..."
if [ ! -d ~/llama.cpp ]; then
    git clone https://github.com/ggerganov/llama.cpp ~/llama.cpp
    echo "✓ llama.cpp cloned"
else
    echo "✓ llama.cpp already exists"
fi

# Step 3: Install Python requirements
echo ""
echo "Step 3: Installing conversion requirements..."
pip install -q -r ~/llama.cpp/requirements.txt
echo "✓ Requirements installed"

# Step 4: Build llama.cpp with CMake
echo ""
echo "Step 4: Building llama-quantize tool with CMake..."
cd ~/llama.cpp
if [ ! -f build/bin/llama-quantize ]; then
    echo "Building llama.cpp (this takes 5-10 minutes)..."
    cmake -B build
    cmake --build build --config Release -j12
    echo "✓ llama-quantize built"
else
    echo "✓ llama-quantize already built"
fi

# Step 5: Convert to FP16 GGUF
echo ""
echo "Step 5: Converting model to FP16 GGUF format..."
echo "This will take 10-15 minutes and use ~15GB RAM"
cd ~/llama.cpp
python3 convert_hf_to_gguf.py     /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/16bit     --outfile /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/gguf/model-f16.gguf     --outtype f16

OUTPUT_DIR="/mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/gguf"
if [ -f "/model-f16.gguf" ]; then
    SIZE=
    echo "✓ FP16 GGUF created: "
else
    echo "✗ FP16 conversion failed"
    exit 1
fi

# Step 6: Quantize to Q4_K_M (recommended for LM Studio)
echo ""
echo "Step 6: Quantizing to Q4_K_M (4-bit, balanced quality/size)..."
echo "This will take 5-10 minutes"
cd ~/llama.cpp
./build/bin/llama-quantize     "/model-f16.gguf"     "/model-q4_k_m.gguf"     Q4_K_M

if [ -f "/model-q4_k_m.gguf" ]; then
    SIZE=
    echo "✓ Q4_K_M GGUF created: "
else
    echo "✗ Quantization failed"
    exit 1
fi

# Step 7: Optional - Create Q5_K_M for better quality
echo ""
echo "Step 7: Creating Q5_K_M quantization (optional, better quality)..."
./build/bin/llama-quantize     "/model-f16.gguf"     "/model-q5_k_m.gguf"     Q5_K_M

if [ -f "/model-q5_k_m.gguf" ]; then
    SIZE=
    echo "✓ Q5_K_M GGUF created: "
fi

# Summary
echo ""
echo "========================================"
echo "Conversion Complete!"
echo "========================================"
echo ""
echo "Generated GGUF files:"
ls -lh ""/*.gguf
echo ""
echo "Recommended for LM Studio: model-q4_k_m.gguf"
echo "Better quality (larger): model-q5_k_m.gguf"
echo ""
echo "Next step: Upload to Hugging Face or transfer to MacBook"
