# Synthia Fine-Tuning Dataset

## Overview

This directory contains a training dataset for fine-tuning Qwen 2.5 Coder 7B to use Synthia's tools proactively and effectively.

## Dataset Details

- **Format**: JSON Lines (.jsonl)
- **Total Examples**: 250
- **File Size**: ~108 KB
- **Purpose**: Teach the model to use tools proactively without being prompted

## Tool Distribution

The dataset includes realistic examples across all Synthia tools:

- **bash**: 116 calls (43.3%) - System commands, build scripts, testing, monitoring
- **read**: 68 calls (25.4%) - Reading config files, source code, logs, documentation
- **glob**: 23 calls (8.6%) - Finding files by pattern (*.py, **/*.ts, etc.)
- **write**: 19 calls (7.1%) - Creating new files (.gitignore, endpoints, models)
- **edit**: 18 calls (6.7%) - Modifying existing files (config updates, code changes)
- **grep**: 14 calls (5.2%) - Searching code patterns (TODOs, functions, imports)
- **powertools**: 10 calls (3.7%) - Code navigation, refactoring, semantic search
  - goto_definition, find_references, list_functions, rename_symbol
  - search_ast, list_classes, project_stats, inline_variable
  - batch_replace, index_project

## Example Scenarios

### Simple Single-Tool Use (50%)
- Check server status
- Read configuration
- Find files by pattern
- Run tests
- Check git status

### Multi-Step Workflows (30%)
- Debug errors (read logs → check code → verify config)
- Update configuration (read → edit → verify)
- Setup features (write routes → add tests → update docs)

### Complex Workflows (20%)
- Full debugging sessions
- Feature implementation (multiple files)
- Refactoring operations
- Project setup and configuration

## Training Characteristics

### Proactive Behavior
- Model initiates tool use without "Would you like me to..."
- Direct action: "I'll check the logs" → tool call
- No unnecessary permission seeking

### Natural Language
- Varied user phrasings (not robotic)
- Real-world programming tasks
- Authentic file paths and code snippets

### Error Handling
- Realistic tool outputs (success and failure)
- Multi-step problem solving
- Context-aware follow-up actions

## Quality Guidelines

1. **Realistic**: Based on actual development workflows
2. **Diverse**: Covers all major programming tasks
3. **Proactive**: Model acts independently
4. **Natural**: Human-like conversation flow
5. **Educational**: Clear cause-and-effect patterns

## File Structure

```
fine-tuning/
├── dataset.jsonl          # Main training dataset (250 examples)
├── generate_dataset.py    # Generation script (archived)
├── train.py              # Main training script (Unsloth + QLoRA)
├── merge_and_export.py   # Merge LoRA adapters and export model
├── test_model.py         # Quick inference test script
├── setup.ps1             # Windows PowerShell setup script
├── requirements.txt      # Python dependencies
└── README.md             # This file
```

## Quick Start (Windows + RTX 4060)

### Prerequisites
- Windows 10/11
- Python 3.10 or 3.11
- NVIDIA RTX 4060 (8GB VRAM)
- ~20GB free disk space

### Step 1: Setup Environment

```powershell
# Run the automated setup script
.\setup.ps1
```

This script will:
- Check Python and CUDA installation
- Create virtual environment
- Install PyTorch with CUDA 12.1
- Install Unsloth and all dependencies
- Verify installation

### Step 2: Train the Model

```powershell
# Activate virtual environment
.\venv\Scripts\Activate.ps1

# Start training (~1-2 hours on RTX 4060)
python train.py
```

Training settings (optimized for 8GB VRAM):
- Model: Qwen2.5-Coder-7B (4-bit quantized)
- LoRA rank: 16
- Batch size: 1 with 8x gradient accumulation
- Max sequence length: 2048 tokens
- Expected VRAM: ~6-7GB peak
- Checkpoints saved every 50 steps

### Step 3: Merge and Export

```powershell
# Merge LoRA adapters into base model
python merge_and_export.py
```

This will create:
- 16-bit merged model (for further training)
- GGUF format (for LM Studio, llama.cpp)
- q4_k_m and q5_k_m quantizations

### Step 4: Test the Model

```powershell
# Quick inference test
python test_model.py
```

Tests with sample prompts to verify tool use behavior.

## Training Configuration

### Memory Optimizations for 8GB VRAM

The training script uses multiple techniques to fit on RTX 4060:

1. **4-bit Quantization**: Reduces model memory by ~75%
2. **QLoRA**: Only trains small adapter layers (0.5% of parameters)
3. **Gradient Checkpointing**: Trades compute for memory
4. **8-bit Optimizer**: Reduces optimizer state memory by ~50%
5. **Small Batch Size**: 1 per device with gradient accumulation

### Adjustable Parameters in train.py

```python
# Model settings
MAX_SEQ_LENGTH = 2048          # Reduce if OOM (1024, 1536)
LORA_RANK = 16                 # Lower for less VRAM (8, 12)

# Training settings
PER_DEVICE_BATCH_SIZE = 1      # Keep at 1 for 8GB
GRADIENT_ACCUMULATION_STEPS = 8 # Adjust for effective batch size
NUM_TRAIN_EPOCHS = 1           # Increase for more training
MAX_STEPS = -1                 # Set to limit steps (e.g., 200)

# Learning rate
LEARNING_RATE = 2e-4           # 2e-4 is good default for LoRA
```

### Troubleshooting

**Out of Memory (OOM) Errors:**
- Reduce `MAX_SEQ_LENGTH` to 1024 or 1536
- Keep `PER_DEVICE_BATCH_SIZE = 1`
- Reduce `LORA_RANK` to 8 or 12
- Close other applications using GPU

**Slow Training:**
- Training should take ~1-2 hours for 250 examples
- If much slower, check GPU is being used: `nvidia-smi`
- Ensure CUDA is available in PyTorch

**Import Errors:**
- Reinstall dependencies: `pip install -r requirements.txt --force-reinstall`
- Check PyTorch CUDA: `python -c "import torch; print(torch.cuda.is_available())"`

## Usage

This dataset can be used to fine-tune Qwen 2.5 Coder 7B (or similar models) to:
- Use tools proactively without prompting
- Choose appropriate tools for tasks
- Chain tools together for complex workflows
- Maintain natural conversation while using tools

## Output Formats

### 16-bit Merged Model
- Location: `./outputs/qwen2.5-coder-synthia-merged/16bit/`
- Size: ~14GB
- Use for: Further fine-tuning, full-precision inference

### GGUF Models
- Location: `./outputs/qwen2.5-coder-synthia-merged/gguf/`
- Formats: q4_k_m (~4GB), q5_k_m (~5GB)
- Use for: LM Studio, llama.cpp, Ollama

Import into LM Studio:
1. Open LM Studio
2. Click "Import Model"
3. Select: `outputs/.../gguf/model-q4_k_m.gguf`
4. Start chatting with your fine-tuned Synthia!

## Example Entry

```json
{
  "messages": [
    {
      "role": "user",
      "content": "Check if the server is running on port 3000"
    },
    {
      "role": "assistant",
      "content": "I'll check what's running on port 3000.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"lsof -i :3000\", \"description\": \"Check port 3000\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "name": "bash",
      "content": "node    12345 user   20u  IPv4 *:3000 (LISTEN)"
    },
    {
      "role": "assistant",
      "content": "Yes, there's a Node.js server running on port 3000 (PID 12345)."
    }
  ]
}
```

## Next Steps

1. **Fine-tune** the model using this dataset
2. **Evaluate** on held-out test cases
3. **Iterate** based on performance metrics
4. **Deploy** the fine-tuned model as Synthia v2

## License

This dataset is part of the Synthia project and follows the same license terms.
