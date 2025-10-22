# Synthia Fine-Tuning Troubleshooting Session Summary

**Date:** October 21, 2025
**Status:** Training in progress with all critical fixes applied
**Goal:** Fix hallucinations and infinite generation loops in fine-tuned model

---

## 🎯 Critical Bugs Found and Fixed

### Bug #1: Missing Chat Template (FIXED ✅)
**Problem:** Unsloth 4-bit tokenizer had no chat template configured.
**Impact:** Training data corrupted, model hallucinated random unicode.
**Fix:** Load official Qwen2.5-Coder chat template in `train.py`:
```python
if not tokenizer.chat_template:
    from transformers import AutoTokenizer
    official_tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-7B-Instruct")
    tokenizer.chat_template = official_tokenizer.chat_template
```
**Location:** `train.py` lines 160-169

---

### Bug #2: Missing EOS Token (CRITICAL FIX ✅)
**Problem:** Training data had `<|im_end|>` but NOT `<|endoftext|>` (EOS token).
**Impact:** Model never learned to stop generating → infinite loops and hallucinations.
**Root Cause:** `format_messages_for_training()` didn't append EOS token.

**Evidence:**
- Base model works: Generates in 1.6s with proper EOS
- Fine-tuned model stuck: Even with 50 token hard limit
- Training data: `"...<|im_end|>\n"` (no EOS)
- Model learned: `<|im_end|>` = text, not stop signal

**Fix:** Append EOS token to every training example in `train.py`:
```python
def format_messages_for_training(examples, tokenizer):
    texts = []
    for messages in examples["messages"]:
        text = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=False)
        text = text + tokenizer.eos_token  # CRITICAL: Add <|endoftext|>
        texts.append(text)
    return {"text": texts}
```
**Location:** `train.py` lines 251-265

**This was THE bug** - everything else was correct, but without EOS the model couldn't stop.

---

### Bug #3: Tokenizer Files Not Uploaded to Hugging Face (FIXED ✅)
**Problem:** Only GGUF file uploaded, LM Studio couldn't load chat template.
**Fix:** Updated `upload-to-huggingface.sh` to upload tokenizer files:
- `tokenizer.json`
- `tokenizer_config.json` (contains chat template)
- `config.json`
- `special_tokens_map.json`
- `generation_config.json`

**Location:** `upload-to-huggingface.sh` lines 96-110

---

## 📋 Current Status

### Training Dataset (2,764 examples total)
```python
DATASET_PATHS = [
    "data/train.jsonl",              # 2,372 examples
    "data/flask_templates.jsonl",    # 14 examples
    "data/failure_recovery.jsonl",   # 300 examples
    "data/train_improved.jsonl"      # 78 examples
]
```

### Training Configuration
- **Model:** `unsloth/qwen2.5-coder-7b-bnb-4bit`
- **Method:** QLoRA (4-bit base + LoRA adapters)
- **LoRA Rank:** 16
- **Learning Rate:** 2e-4
- **Batch Size:** 1 (effective 8 with gradient accumulation)
- **Epochs:** 1
- **Expected Time:** ~30-60 minutes

### What's Running Now
```bash
cd /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning
./windows-wsl2/run-full-pipeline.sh
```

This script:
1. ✅ Sets up training environment (PyTorch CUDA)
2. 🔄 **Currently running:** Fine-tuning with EOS token fix
3. ⏳ Pending: Merge LoRA adapters to 16-bit
4. ⏳ Pending: Convert to GGUF (F16, Q4_K_M, Q5_K_M)

---

## 🔧 When Training Finishes

### 1. Test the Model Locally (Optional)
```bash
cd /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning
source ~/synthia-training/venv/bin/activate
python test_16bit_model.py
```

**Expected output:**
- ✅ Clean response (no hallucinations)
- ✅ Generates `<|endoftext|>` to stop
- ✅ No infinite loops

### 2. Upload to Hugging Face
```bash
cd /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning

# Set your HF token (get from https://huggingface.co/settings/tokens)
export HF_TOKEN="hf_your_token_here"

# Run upload script
wsl bash upload-to-huggingface.sh
```

**Files uploaded:**
- `model-q4_k_m.gguf` (4.4 GB) ← Use this in LM Studio
- `tokenizer.json`, `tokenizer_config.json`, `config.json`, etc.

### 3. Download and Test in LM Studio
1. **Delete old model** in LM Studio (if exists)
2. **Search for:** `zachswift615/synthia-coder`
3. **Download** the model (includes tokenizer files)
4. **Load and test** in Synthia

**Expected behavior:**
- ✅ No hallucinations
- ✅ Stops generating properly
- ✅ Clean tool calls with `<tool_call>` XML
- ✅ Coherent, helpful responses

---

## 🧪 Diagnostic Scripts (Created)

All located in `synthia/fine-tuning/`:

### Test Scripts
- **`test_16bit_model.py`** - Test merged model before GGUF conversion
- **`test_base_model.py`** - Verify base Qwen model works (baseline)
- **`diagnose_generation_loop.py`** - Check if model generates EOS token
- **`verify_training_format.py`** - Check training data has proper formatting

### How We Diagnosed
1. **LM Studio hallucinations** → Thought it was chat template
2. **Fixed chat template** → Still hallucinating
3. **Uploaded tokenizer files** → Still hallucinating
4. **Tested GGUF conversion** → Still hallucinating
5. **Tested 16-bit model** → Still stuck (not GGUF issue)
6. **Tested base model** → Works perfectly! (1.6s generation)
7. **Checked training data** → Has `<|im_end|>` but NO `<|endoftext|>`
8. **Found root cause** → EOS token never added to training examples

**Key insight:** Base model worked, fine-tuned model didn't → problem is in training, not inference.

---

## ⚙️ Important Files and Locations

### Training Pipeline
- **Main script:** `windows-wsl2/run-full-pipeline.sh`
- **Training script:** `train.py` (has both chat template + EOS fixes)
- **Merge script:** `merge_and_export.py`
- **Upload script:** `upload-to-huggingface.sh`

### Training Environments (WSL2)
- **Training venv:** `~/synthia-training/venv` (PyTorch CUDA)
- **Conversion venv:** `~/synthia-training/venv-conversion` (llama.cpp tools)
- **llama.cpp:** `~/llama.cpp` (for GGUF conversion)

### Output Locations
```
synthia/fine-tuning/outputs/qwen2.5-coder-synthia-merged/
├── 16bit/                    # Merged 16-bit model (~15 GB)
│   ├── model-*.safetensors
│   ├── tokenizer.json
│   ├── tokenizer_config.json
│   └── config.json
└── gguf/                     # GGUF files for LM Studio
    ├── model-f16.gguf        # 15 GB - Full precision
    ├── model-q4_k_m.gguf     # 4.4 GB - Recommended ⭐
    └── model-q5_k_m.gguf     # 5.1 GB - Better quality
```

---

## 🚨 Known Issues and Solutions

### Issue: Training gets stuck
**Solution:** Check CUDA is available:
```bash
source ~/synthia-training/venv/bin/activate
python -c "import torch; print(torch.cuda.is_available())"
```
Should print `True`. If `False`, reinstall PyTorch nightly.

### Issue: Line endings error "cannot execute: required file not found"
**Solution:** Convert line endings:
```bash
wsl bash -c "dos2unix /path/to/script.sh"
```

### Issue: "Dataset not found: data/train_improved.jsonl"
**Solution:** Comment out in `train.py` if file doesn't exist:
```python
# "data/train_improved.jsonl"  # Uncomment when ready
```

### Issue: Model still hallucinating after retraining
**Checklist:**
1. ✅ Did training script show "✓ Chat template verified"?
2. ✅ Did you delete old checkpoint before training?
3. ✅ Did training loss decrease? (Check logs: 2.7 → 0.5-0.7)
4. ✅ Did you upload tokenizer files with GGUF?
5. ✅ Did you delete old model in LM Studio and re-download?

---

## 📊 Training Loss Reference

**Healthy training loss progression:**
```
Step 10:  loss: 2.718
Step 50:  loss: 1.587
Step 100: loss: 1.213
Step 150: loss: 1.016
Step 200: loss: 0.849
Step 250: loss: 0.810
Step 300: loss: 0.647
Final:    loss: ~0.5-0.7
```

If loss doesn't decrease or stays > 2.0, training failed.

---

## 🔑 Key Takeaways

1. **Chat template was necessary but not sufficient** - Needed EOS token too
2. **Base model test was critical** - Proved problem was in fine-tuning, not inference
3. **GGUF files embed chat template** - But LM Studio needs separate tokenizer files
4. **EOS token must be in training data** - Model can't learn what it never saw
5. **Diagnostic scripts saved us** - Systematic testing isolated the exact bug

---

## 📝 Next Session TODO

- [ ] Wait for training to complete (~30-60 min from start)
- [ ] Test 16-bit model with `test_16bit_model.py`
- [ ] Upload to Hugging Face with `upload-to-huggingface.sh`
- [ ] Download in LM Studio and test with Synthia
- [ ] If model works: Celebrate! 🎉
- [ ] If still broken: Review training logs for errors

---

## 💡 Quick Commands Reference

```bash
# Start training
cd /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning
./windows-wsl2/run-full-pipeline.sh

# Test 16-bit model
source ~/synthia-training/venv/bin/activate
python test_16bit_model.py

# Upload to HuggingFace
export HF_TOKEN="hf_your_token_here"
wsl bash upload-to-huggingface.sh

# Fix line endings (if needed)
wsl bash -c "dos2unix ./path/to/script.sh"

# Check CUDA
python -c "import torch; print(torch.cuda.is_available())"
```

---

## 🔗 Useful Links

- **Hugging Face Tokens:** https://huggingface.co/settings/tokens
- **Your Model Repo:** https://huggingface.co/zachswift615/synthia-coder
- **Qwen2.5-Coder:** https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct
- **Unsloth:** https://github.com/unslothai/unsloth
- **llama.cpp:** https://github.com/ggerganov/llama.cpp

---

**Last Updated:** October 21, 2025 (during active training session)
**Training Status:** In progress with all critical fixes applied
**Expected Completion:** 30-60 minutes from start
**Confidence Level:** HIGH - Root cause identified and fixed
