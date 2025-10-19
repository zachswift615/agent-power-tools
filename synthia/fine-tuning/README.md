# Synthia Fine-Tuning Pipeline (MLX for Mac)

This directory contains the fine-tuning pipeline for Synthia using Apple's MLX framework, optimized for Mac M1 Pro with 16GB RAM.

## Overview

**Base Model:** `zachswift615/qwen2.5-coder-synthia-tool-use` (already fine-tuned once)
**Goal:** Continued fine-tuning with comprehensive agentic skills
**Target Dataset Size:** 1,500-2,600 examples
**Current Dataset Size:** 250 examples

## Hardware Requirements

- **Mac with Apple Silicon** (M1, M1 Pro, M1 Max, M2, M3, etc.)
- **Minimum 16GB RAM** (unified memory)
- **~20GB free disk space** (for models and checkpoints)

## Setup

### 1. Create Python Virtual Environment

```bash
cd fine-tuning
python3 -m venv venv
source venv/bin/activate  # On Mac/Linux
```

### 2. Install Dependencies

```bash
pip install -r requirements.txt
```

### 3. Download Base Model (if needed)

The model is already in LM Studio at:
```
/Users/zachswift/.lmstudio/models/zachswift615/qwen2.5-coder-synthia-tool-use/model-q4_k_m.gguf
```

For MLX, download the original HuggingFace version:
```bash
huggingface-cli download zachswift615/qwen2.5-coder-synthia-tool-use
```

Or use directly from HuggingFace (downloads automatically on first use).

## Dataset Structure

The training dataset is in `dataset.jsonl`. Each line is a complete conversation in OpenAI chat format:

```json
{
  "messages": [
    {
      "role": "system",
      "content": "You are Synthia, a helpful coding assistant..."
    },
    {
      "role": "user",
      "content": "Read the file src/main.rs"
    },
    {
      "role": "assistant",
      "content": "I'll read that file for you.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "read",
            "arguments": "{\"file_path\": \"src/main.rs\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "fn main() { ... }"
    },
    {
      "role": "assistant",
      "content": "This is a Rust main function that..."
    }
  ]
}
```

## Training Stages

The pipeline runs in 3 stages:

### Stage 1: Tool Use Reinforcement
- **Dataset:** ~750 examples (current 250 + new 500)
- **Iterations:** 800
- **Learning Rate:** 2e-5
- **Focus:** Reinforce tool calling accuracy, parallel execution, error recovery
- **Time:** ~30-40 minutes

### Stage 2: Agentic Skills
- **Dataset:** ~1,050 examples (Stage 1 + 300 agentic)
- **Iterations:** 1,000
- **Learning Rate:** 1.5e-5
- **Focus:** TDD workflows, systematic debugging, planning
- **Time:** ~40-50 minutes

### Stage 3: Full Integration
- **Dataset:** ~1,500-2,600 examples (all categories)
- **Iterations:** 1,500
- **Learning Rate:** 1e-5
- **Focus:** Integration of all skills (craftsmanship, collaboration, problem-solving)
- **Time:** ~60-90 minutes

**Total Training Time:** ~2-3 hours

## Running Training

### Run All Stages

```bash
python train_mlx.py --stage all
```

### Run Individual Stage

```bash
python train_mlx.py --stage 1
python train_mlx.py --stage 2
python train_mlx.py --stage 3
```

### Run with Testing

```bash
python train_mlx.py --stage all --test
```

### Custom Options

```bash
python train_mlx.py \
  --stage 1 \
  --base-model zachswift615/qwen2.5-coder-synthia-tool-use \
  --dataset dataset.jsonl \
  --output-dir models \
  --test
```

## Output

Trained models are saved to `fine-tuning/models/`:

```
fine-tuning/models/
├── stage1/
│   ├── adapters.npz       # LoRA weights
│   └── config.json        # Training config
├── synthia-stage1/        # Fused model
├── stage2/
│   ├── adapters.npz
│   └── config.json
├── synthia-stage2/        # Fused model
├── stage3/
│   ├── adapters.npz
│   └── config.json
└── synthia-stage3/        # Final model
```

## Using the Fine-Tuned Model

### Test with MLX

```bash
mlx_lm.generate \
  --model fine-tuning/models/synthia-stage3 \
  --prompt "Read the file src/main.rs and tell me what it does" \
  --max-tokens 500
```

### Convert to GGUF for LM Studio

```bash
# 1. Convert to GGUF (requires llama.cpp)
python llama.cpp/convert.py \
  fine-tuning/models/synthia-stage3 \
  --outfile synthia-stage3-f16.gguf

# 2. Quantize (optional, for smaller file size)
llama.cpp/quantize \
  synthia-stage3-f16.gguf \
  synthia-stage3-q4_k_m.gguf \
  q4_k_m

# 3. Move to LM Studio models directory
cp synthia-stage3-q4_k_m.gguf \
  ~/.lmstudio/models/zachswift615/qwen2.5-coder-synthia-v2/
```

### Upload to HuggingFace

```bash
huggingface-cli login

huggingface-cli upload \
  zachswift615/qwen2.5-coder-synthia-v2 \
  fine-tuning/models/synthia-stage3
```

## Memory Optimization

If you encounter OOM (Out Of Memory) errors:

1. **Reduce batch size:** Change `batch_size: 2` to `batch_size: 1`
2. **Reduce sequence length:** Change `max_seq_length: 2048` to `max_seq_length: 1024`
3. **Close other apps:** Free up memory during training
4. **Monitor memory:** Use Activity Monitor to watch RAM usage

## Validation

After each stage, validate the model:

```python
# Test tool calling
prompt = "Read src/main.rs and find all async functions"

# Test TDD workflow
prompt = "Add a divide function to the calculator library using TDD"

# Test debugging
prompt = "Debug why the server is returning 500 errors"

# Test refactoring
prompt = "Refactor this code to follow DRY principle"
```

## Troubleshooting

### "mlx_lm.lora: command not found"

Make sure MLX is installed and you're in the venv:
```bash
source venv/bin/activate
pip install mlx-lm
```

### "Out of memory"

Reduce batch_size to 1 or max_seq_length to 1024.

### "Invalid JSON in dataset"

Run validation:
```bash
python -c "
import json
with open('dataset.jsonl') as f:
    for i, line in enumerate(f):
        json.loads(line)  # Will error on invalid JSON
print('✓ All valid')
"
```

### Training is slow

This is normal for 7B models on 16GB RAM. Expected times:
- 800 iterations: ~30-40 minutes
- 1500 iterations: ~60-90 minutes

## Next Steps

1. **Generate more training data** - Use the specialized agents in `.claude/agents/`
2. **Run Stage 1** - Start with tool use reinforcement
3. **Validate outputs** - Test model after each stage
4. **Iterate** - Add more examples for weak areas
5. **Upload to HuggingFace** - Share the improved model

## Resources

- [MLX Documentation](https://ml-explore.github.io/mlx/build/html/index.html)
- [MLX Examples](https://github.com/ml-explore/mlx-examples)
- [LoRA Paper](https://arxiv.org/abs/2106.09685)
- [Qwen2.5-Coder](https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct)
