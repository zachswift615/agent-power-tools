# Synthia Hallucination Bug - Root Cause and Fix

## The Problem

When testing the fine-tuned Synthia model, it produced severe hallucinations:
- Random unicode characters (𤫉, ปกคร, 𨭉, 魔龙令牌, etc.)
- Mixed languages (Thai, Hebrew, Chinese, Polish, Turkish)
- Malformed tool calls with excessive markup (`<LM>`, `{lng}`, `<lemma>`)
- Wrong tool usage and confused context
- Repetitive, looping behavior

**This was a catastrophic failure**, not just a minor issue.

## Root Cause

The **Unsloth 4-bit version of Qwen2.5-Coder** (`unsloth/qwen2.5-coder-7b-bnb-4bit`) does not have a chat template configured.

When `train.py` called `tokenizer.apply_chat_template()`:
1. The function failed silently (no chat template set)
2. Training data was corrupted or produced as garbage
3. The model learned these corrupted patterns
4. During inference, it reproduced the hallucinations it learned

### Evidence

Running diagnostic script showed:
```
CHAT TEMPLATE:
No chat template found!

AFTER CHAT TEMPLATE:
ERROR: Cannot use chat template functions because tokenizer.chat_template is not set
```

## The Fix

### What Changed

Updated `train.py` to:

1. **Load official Qwen2.5-Coder chat template** after loading the model:
   ```python
   if not tokenizer.chat_template or not tokenizer.chat_template.strip():
       from transformers import AutoTokenizer
       official_tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-7B-Instruct")
       tokenizer.chat_template = official_tokenizer.chat_template
   ```

2. **Verify chat template works** before training starts:
   ```python
   try:
       test_messages = [{"role": "user", "content": "test"}]
       test_output = tokenizer.apply_chat_template(test_messages, tokenize=False)
   except Exception as e:
       raise  # STOP training if chat template broken
   ```

3. **Applied fix to both code paths**:
   - Fresh training (loading base model)
   - Checkpoint resuming (loading from checkpoint)

### What the Official Template Does

Qwen2.5-Coder uses:
- `<|im_start|>` and `<|im_end|>` markers for messages
- `<tool_call>` and `</tool_call>` XML tags for function calls
- `<tool_response>` and `</tool_response>` for tool outputs
- Proper formatting for system, user, assistant roles

**Example formatted output:**
```
<|im_start|>user
Find files<|im_end|>
<|im_start|>assistant
I'll search for files.
<tool_call>
{"name": "glob", "arguments": "{\"pattern\": \"*.py\"}"}
</tool_call><|im_end|>
<|im_start|>user
<tool_response>
main.py
test.py
</tool_response><|im_end|>
<|im_start|>assistant
Found 2 Python files.<|im_end|>
```

## Next Steps

### 1. Delete Corrupted Model

The existing trained model is corrupted and cannot be fixed. Delete it:

```bash
rm -rf ~/synthia-training/outputs/qwen2.5-coder-synthia-tool-use
rm -rf ~/synthia-training/outputs/qwen2.5-coder-synthia-merged
```

### 2. Re-run Training with Fixed Script

The fixed `train.py` will now:
- Load the official chat template automatically
- Verify it works before training starts
- Fail loudly if there's a problem (instead of silently producing garbage)

Run the full pipeline again:

```bash
cd /mnt/c/Users/crasz/projects/agent-power-tools/synthia/fine-tuning
./windows-wsl2/run-full-pipeline.sh
```

This time you should see:
```
⚠️  Tokenizer missing chat template - loading from official Qwen2.5-Coder...
✓ Chat template loaded from official Qwen2.5-Coder
🔍 Verifying chat template...
✓ Chat template verified and working correctly
```

### 3. Additional Recommendations

While fixing the chat template is the critical fix, consider these improvements for even better results:

#### A. Increase Warmup Steps
Current: `WARMUP_STEPS = 10` (too low)
Recommended: `WARMUP_STEPS = 50` or `WARMUP_STEPS = 100`

Why: Reduces learning rate shock at the start of training.

#### B. Consider Lower Learning Rate
Current: `LEARNING_RATE = 2e-4`
Alternative: `LEARNING_RATE = 1e-4` or `LEARNING_RATE = 5e-5`

Why: More conservative, less likely to cause catastrophic forgetting.

#### C. Review Training Data Quality

Check your training data:
```bash
# Look for any corrupted examples
python -c "
import json
with open('data/train.jsonl', 'r') as f:
    for i, line in enumerate(f, 1):
        try:
            data = json.loads(line)
            assert 'messages' in data
            for msg in data['messages']:
                assert 'role' in msg
                assert 'content' in msg or 'tool_calls' in msg
        except Exception as e:
            print(f'Error in line {i}: {e}')
"
```

#### D. Add Validation Dataset (Optional)

Split your data into train/validation to monitor overfitting:
- Train: 90% of examples
- Validation: 10% of examples

Add to `train.py`:
```python
VALIDATION_SPLIT = 0.1  # 10% for validation
```

## Technical Details

### Why Did the Original Code Fail?

The original `train.py` had this check:
```python
if tokenizer.chat_template is None:
    from unsloth.chat_templates import get_chat_template
    tokenizer = get_chat_template(tokenizer, chat_template="qwen-2.5")
```

This failed because:
1. `tokenizer.chat_template` might not be `None` - could be empty string, whitespace, or invalid value
2. Unsloth's `get_chat_template()` might not work correctly for Qwen2.5-Coder
3. No verification that the template actually works

The fix:
1. Checks for `not tokenizer.chat_template or not tokenizer.chat_template.strip()` (catches all cases)
2. Uses official Qwen2.5-Coder template (guaranteed to work)
3. Verifies template works before continuing (fail-fast)

## Verification

After retraining, test the model with a simple prompt:

```python
# Test script
from unsloth import FastLanguageModel

model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="outputs/qwen2.5-coder-synthia-tool-use",
    max_seq_length=2048,
    dtype=None,
    load_in_4bit=True,
)
FastLanguageModel.for_inference(model)

messages = [{"role": "user", "content": "Write a hello world script"}]
inputs = tokenizer.apply_chat_template(messages, tokenize=True, return_tensors="pt").to("cuda")
outputs = model.generate(inputs, max_new_tokens=200)
print(tokenizer.decode(outputs[0]))
```

Expected: Clean, coherent response about writing a Python hello world script
Not expected: Random unicode, mixed languages, or hallucinations

## Summary

- **Bug**: Unsloth 4-bit tokenizer missing chat template
- **Impact**: Training produced corrupted data → model hallucinated
- **Fix**: Load official Qwen2.5-Coder template + verify it works
- **Action**: Delete old model, retrain with fixed script
- **Result**: Clean, properly formatted training data → coherent model

The model should now work correctly for tool use and code generation tasks!
