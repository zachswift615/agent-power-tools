#!/bin/bash
# Train Synthia model with the new merged dataset including:
# - Original 2,372 examples
# - 300 failure recovery examples
# - 14 Flask/Jinja2 template examples

set -e

echo "🚀 Training Synthia with new dataset (2,686 total examples)"
echo ""
echo "Dataset composition:"
echo "  - Original examples: 2,372 (88.3%)"
echo "  - Failure recovery: 300 (11.2%)"
echo "  - Flask templates: 14 (0.5%)"
echo ""
echo "Training set: 2,417 examples"
echo "Validation set: 269 examples"
echo ""

# Navigate to fine-tuning directory
cd "$(dirname "$0")"

# Check if merged datasets exist
if [ ! -f "data/train_merged.jsonl" ] || [ ! -f "data/valid_merged.jsonl" ]; then
    echo "❌ Merged datasets not found. Running merge script..."
    python3 merge_datasets.py
fi

echo "✅ Datasets ready"
echo ""
echo "Starting training..."
echo ""

# Train with MLX
python3 train_mlx.py \
  --data data/train_merged.jsonl \
  --valid data/valid_merged.jsonl \
  --model-name Qwen/Qwen2.5-Coder-7B-Instruct \
  --iters 600 \
  --learning-rate 1e-5 \
  --batch-size 4 \
  --val-batches 10

echo ""
echo "✅ Training complete!"
echo ""
echo "Next steps:"
echo "  1. Convert to GGUF: ./convert_to_gguf.sh"
echo "  2. Test with Synthia"
echo "  3. Compare with previous version"
