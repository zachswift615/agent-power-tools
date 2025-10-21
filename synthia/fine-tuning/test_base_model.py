"""
Test the BASE Qwen2.5-Coder model (before fine-tuning)
to see if the generation loop issue exists in the base model
"""
import torch
from unsloth import FastLanguageModel
from transformers import AutoTokenizer

print("="*80)
print("TESTING BASE MODEL (before fine-tuning)")
print("="*80)

# Load the base model that we fine-tuned FROM
print("\n1. Loading base Qwen2.5-Coder 4-bit model...")
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="unsloth/qwen2.5-coder-7b-bnb-4bit",
    max_seq_length=2048,
    dtype=None,
    load_in_4bit=True,
)

# Load official Qwen tokenizer with chat template
print("\n2. Loading chat template from official Qwen...")
official_tokenizer = AutoTokenizer.from_pretrained("Qwen/Qwen2.5-Coder-7B-Instruct")
tokenizer.chat_template = official_tokenizer.chat_template

print(f"   ✓ Chat template loaded")
print(f"   EOS token: {tokenizer.eos_token} (ID: {tokenizer.eos_token_id})")

# Prepare for inference
FastLanguageModel.for_inference(model)

print("\n3. Testing generation with 'Hello' (max 50 tokens)...")
messages = [{"role": "user", "content": "Hello"}]

formatted = tokenizer.apply_chat_template(
    messages,
    tokenize=False,
    add_generation_prompt=True
)

print(f"\nFormatted input:\n{formatted}\n")

inputs = tokenizer(formatted, return_tensors="pt").to("cuda")

print("Generating...")
import time
start = time.time()

outputs = model.generate(
    **inputs,
    max_new_tokens=50,
    temperature=0.7,
    do_sample=True,
    pad_token_id=tokenizer.pad_token_id,
    eos_token_id=tokenizer.eos_token_id,
)

elapsed = time.time() - start
print(f"✓ Generation completed in {elapsed:.2f} seconds")

response = tokenizer.decode(outputs[0], skip_special_tokens=False)

print("\n" + "="*80)
print("OUTPUT:")
print("="*80)
print(response)

print("\n" + "="*80)
print("CONCLUSION:")
print("="*80)

if elapsed > 30:
    print("❌ BASE MODEL ALSO GETS STUCK!")
    print("   The Unsloth 4-bit base model has generation issues.")
    print("   Solution: Fine-tune from the official 16-bit model instead.")
else:
    print("✓ Base model generates fine.")
    print("   Problem is specific to our fine-tuned model.")
    print("   Issue likely in training or merge step.")
