#!/bin/bash

set -e  # Exit on error

VENV_PATH=~/hugging_face_cli_venv
MODEL_PATH="./outputs/qwen2.5-coder-synthia-merged/gguf/model-q4_k_m.gguf"
HF_REPO="zachswift615/synthia-coder"

echo "=== Hugging Face Model Upload Script ==="
echo ""

# Check if venv exists, create if it doesn't
if [ ! -d "$VENV_PATH" ]; then
    echo "Virtual environment not found. Creating at $VENV_PATH..."
    python3 -m venv "$VENV_PATH"
    echo "✓ Virtual environment created"
else
    echo "✓ Virtual environment found at $VENV_PATH"
fi

# Activate virtual environment
echo "Activating virtual environment..."
source "$VENV_PATH/bin/activate"

# Check if huggingface-hub is installed, install if needed
if ! pip show huggingface-hub &> /dev/null; then
    echo "Installing huggingface-hub..."
    pip install huggingface-hub
    echo "✓ huggingface-hub installed"
else
    echo "✓ huggingface-hub already installed"
fi

# Check if model file exists
if [ ! -f "$MODEL_PATH" ]; then
    echo "❌ Error: Model file not found at $MODEL_PATH"
    echo "Please ensure the model has been converted to GGUF format."
    exit 1
fi

echo "✓ Model file found: $MODEL_PATH"
echo ""

# Login to Hugging Face
echo "Logging into Hugging Face..."
echo "Please paste your Hugging Face token (get it from https://huggingface.co/settings/tokens)"
huggingface-cli login

echo ""
echo "=== Uploading model to Hugging Face ==="
echo "Repository: $HF_REPO"
echo "Model: $MODEL_PATH"
echo ""

# Upload the model
huggingface-cli upload "$HF_REPO" "$MODEL_PATH" --repo-type model

echo ""
echo "=== Upload Complete! ==="
echo "Your model is now available at: https://huggingface.co/$HF_REPO"
echo "You can download it in LM Studio by searching for: $HF_REPO"
