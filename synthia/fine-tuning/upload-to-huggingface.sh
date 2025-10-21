#!/bin/bash

set -e  # Exit on error

VENV_PATH=~/hugging_face_cli_venv
MODEL_PATH="./outputs/qwen2.5-coder-synthia-merged/gguf/model-q4_k_m.gguf"
TOKENIZER_DIR="./outputs/qwen2.5-coder-synthia-merged/16bit"
HF_REPO="zachswift615/synthia-coder"

echo "=== Hugging Face Model Upload Script ==="
echo ""

# Deactivate any currently active virtual environment
if [ -n "$VIRTUAL_ENV" ]; then
    echo "Deactivating current virtual environment: $VIRTUAL_ENV"
    deactivate || true  # Don't fail if deactivate doesn't exist
fi

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

# Check if tokenizer directory exists
if [ ! -d "$TOKENIZER_DIR" ]; then
    echo "❌ Error: Tokenizer directory not found at $TOKENIZER_DIR"
    echo "Please ensure the model has been merged to 16bit."
    exit 1
fi

echo "✓ Tokenizer directory found: $TOKENIZER_DIR"

# Check for required tokenizer files
REQUIRED_FILES=("tokenizer.json" "tokenizer_config.json" "config.json")
MISSING_FILES=()

for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$TOKENIZER_DIR/$file" ]; then
        MISSING_FILES+=("$file")
    fi
done

if [ ${#MISSING_FILES[@]} -gt 0 ]; then
    echo "⚠️  Warning: Some tokenizer files are missing: ${MISSING_FILES[*]}"
    echo "   The model may still work, but chat template might not be loaded correctly."
else
    echo "✓ All required tokenizer files found"
fi

echo ""

# Login to Hugging Face
echo "Logging into Hugging Face..."
echo "Please paste your Hugging Face token (get it from https://huggingface.co/settings/tokens)"
huggingface-cli login

echo ""
echo "=== Uploading model to Hugging Face ==="
echo "Repository: $HF_REPO"
echo ""

# Upload the GGUF model
echo "1. Uploading GGUF model..."
huggingface-cli upload "$HF_REPO" "$MODEL_PATH" --repo-type model
echo "   ✓ GGUF model uploaded"

# Upload tokenizer files
echo ""
echo "2. Uploading tokenizer files..."
for file in tokenizer.json tokenizer_config.json config.json special_tokens_map.json generation_config.json; do
    if [ -f "$TOKENIZER_DIR/$file" ]; then
        echo "   Uploading $file..."
        huggingface-cli upload "$HF_REPO" "$TOKENIZER_DIR/$file" --repo-type model
        echo "   ✓ $file uploaded"
    fi
done

echo ""
echo "=== Upload Complete! ==="
echo "Your model is now available at: https://huggingface.co/$HF_REPO"
echo ""
echo "Files uploaded:"
echo "  - GGUF model (for LM Studio)"
echo "  - Tokenizer files (with Qwen chat template)"
echo ""
echo "To use in LM Studio:"
echo "  1. Search for: $HF_REPO"
echo "  2. Download the model"
echo "  3. LM Studio will automatically use the correct chat template"
