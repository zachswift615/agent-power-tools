"""
Diagnose why the model gets stuck in infinite generation
"""
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer
import time

MODEL_PATH = "./outputs/qwen2.5-coder-synthia-merged/16bit"

print("Loading model and tokenizer...")
tokenizer = AutoTokenizer.from_pretrained(MODEL_PATH)
model = AutoModelForCausalLM.from_pretrained(
    MODEL_PATH,
    torch_dtype=torch.float16,
    device_map="auto",
)

print("\n" + "="*80)
print("TOKENIZER DIAGNOSTICS:")
print("="*80)
print(f"EOS token: {tokenizer.eos_token} (ID: {tokenizer.eos_token_id})")
print(f"BOS token: {tokenizer.bos_token} (ID: {tokenizer.bos_token_id})")
print(f"PAD token: {tokenizer.pad_token} (ID: {tokenizer.pad_token_id})")
print(f"Chat template exists: {tokenizer.chat_template is not None}")

# Check if im_end token exists
im_end_id = tokenizer.convert_tokens_to_ids("<|im_end|>")
print(f"<|im_end|> token ID: {im_end_id}")

print("\n" + "="*80)
print("GENERATION TEST (with streaming and early stop):")
print("="*80)

messages = [{"role": "user", "content": "Hello"}]
formatted_input = tokenizer.apply_chat_template(
    messages,
    tokenize=False,
    add_generation_prompt=True
)

print(f"Input:\n{formatted_input}\n")

inputs = tokenizer(formatted_input, return_tensors="pt").to(model.device)

print("Generating (max 50 tokens)...")
start_time = time.time()

# Generate with strict limits and proper stopping
outputs = model.generate(
    **inputs,
    max_new_tokens=50,  # Strict limit
    temperature=0.7,
    do_sample=True,
    pad_token_id=tokenizer.pad_token_id,
    eos_token_id=tokenizer.eos_token_id,  # Use EOS token
    early_stopping=True,
)

elapsed = time.time() - start_time
print(f"Generation took {elapsed:.2f} seconds")

# Decode and show output
response = tokenizer.decode(outputs[0], skip_special_tokens=False)

print("\n" + "="*80)
print("OUTPUT (with special tokens):")
print("="*80)
print(response)

print("\n" + "="*80)
print("ANALYSIS:")
print("="*80)

# Count tokens
new_tokens = outputs[0][len(inputs['input_ids'][0]):]
print(f"Generated {len(new_tokens)} tokens")

# Check if it hit EOS
if tokenizer.eos_token_id in new_tokens.tolist():
    print("✓ Model generated EOS token (stopped naturally)")
else:
    print("❌ Model did NOT generate EOS token (hit max_new_tokens limit)")
    print("   This means the model doesn't know when to stop!")

# Check if it generated im_end
if im_end_id in new_tokens.tolist():
    print("✓ Model generated <|im_end|> token")
else:
    print("❌ Model did NOT generate <|im_end|> token")

# Check for hallucination markers in just the new tokens
decoded_new = tokenizer.decode(new_tokens, skip_special_tokens=False)
hallucination_markers = ["𝆣", "NdrFc", "𥖨", "คู่", "ניוזל", "zwłaszc"]
found = [m for m in hallucination_markers if m in decoded_new]
if found:
    print(f"❌ HALLUCINATIONS FOUND: {found}")
else:
    print("✓ No hallucinations in generated tokens")

print("\n" + "="*80)
print("DIAGNOSIS:")
print("="*80)

if tokenizer.eos_token_id in new_tokens.tolist():
    print("Model CAN stop properly. Problem might be:")
    print("  - LM Studio not respecting EOS token")
    print("  - Max tokens set too high in LM Studio")
else:
    print("Model CANNOT stop properly. Root causes:")
    print("  1. EOS token not trained correctly")
    print("  2. Chat template issue during training")
    print("  3. Training data didn't include proper endings")
    print("\nFIX: Need to retrain with correct EOS token handling")
