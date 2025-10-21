"""
Test the 16-bit merged model BEFORE GGUF conversion
This will tell us if the problem is the model itself or the GGUF conversion
"""
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer
import json

print("="*80)
print("TESTING 16-BIT MODEL (BEFORE GGUF CONVERSION)")
print("="*80)

# Load the 16-bit merged model
MODEL_PATH = "./outputs/qwen2.5-coder-synthia-merged/16bit"

print(f"\n1. Loading model from: {MODEL_PATH}")
tokenizer = AutoTokenizer.from_pretrained(MODEL_PATH)
model = AutoModelForCausalLM.from_pretrained(
    MODEL_PATH,
    torch_dtype=torch.float16,
    device_map="auto",  # Automatically use GPU if available
)

print("   ✓ Model loaded")

# Check chat template
print(f"\n2. Checking chat template:")
if tokenizer.chat_template:
    print("   ✓ Chat template found")
    # Show first 200 chars
    template_preview = tokenizer.chat_template[:200] + "..."
    print(f"   Preview: {template_preview}")
else:
    print("   ❌ NO CHAT TEMPLATE!")
    print("   This means the merge step didn't save the template correctly.")
    exit(1)

# Test with a simple message
print(f"\n3. Testing with simple message:")
messages = [
    {"role": "user", "content": "Hello! Can you write a Python function that adds two numbers?"}
]

# Apply chat template
formatted_input = tokenizer.apply_chat_template(
    messages,
    tokenize=False,
    add_generation_prompt=True
)

print("\n   Formatted input:")
print("   " + formatted_input.replace("\n", "\n   "))

# Tokenize
inputs = tokenizer(formatted_input, return_tensors="pt").to(model.device)

print(f"\n4. Generating response...")
with torch.no_grad():
    outputs = model.generate(
        **inputs,
        max_new_tokens=200,
        temperature=0.7,
        do_sample=True,
        pad_token_id=tokenizer.eos_token_id,
    )

# Decode
response = tokenizer.decode(outputs[0], skip_special_tokens=False)

print("\n" + "="*80)
print("FULL OUTPUT (with special tokens):")
print("="*80)
print(response)

# Extract just the assistant response
assistant_response = response.split("<|im_start|>assistant")[-1].split("<|im_end|>")[0].strip()

print("\n" + "="*80)
print("ASSISTANT RESPONSE ONLY:")
print("="*80)
print(assistant_response)

print("\n" + "="*80)
print("ANALYSIS:")
print("="*80)

# Check for hallucination indicators
hallucination_markers = ["𝆣", "NdrFc", "𥖨", "คู่", "ניוזל", "zwłaszc", "ปกคร", "習", "魔龙令牌"]
found_hallucinations = [marker for marker in hallucination_markers if marker in assistant_response]

if found_hallucinations:
    print(f"❌ HALLUCINATIONS DETECTED: {found_hallucinations}")
    print("   Problem: The model itself is corrupted (training or merge issue)")
else:
    print("✓ No obvious hallucination markers detected")

if "<tool_call>" in assistant_response and "</tool_call>" in assistant_response:
    print("✓ Model is using <tool_call> XML format (correct)")
elif "tool_calls" in assistant_response:
    print("⚠️  Model is using OpenAI JSON format instead of XML")
else:
    print("ℹ  No tool calls in this response (expected for simple question)")

print("\n" + "="*80)
print("CONCLUSION:")
print("="*80)
if found_hallucinations:
    print("The 16-bit model is already corrupted BEFORE GGUF conversion.")
    print("This means the problem is in training or the merge step.")
else:
    print("The 16-bit model seems OK. Problem might be in GGUF conversion/quantization.")
    print("Try using the Q5_K_M model (less aggressive quantization).")
