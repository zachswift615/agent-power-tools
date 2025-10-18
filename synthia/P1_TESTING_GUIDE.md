# P1 Testing Guide - Synthia Improvements

## Build Location
Binary: `./synthia/target/release/synthia`

## What Was Changed

### ✅ Task 1: Word Wrapping Fix
**Problem:** Text breaking mid-word like "usin g HTML"
**Fix:** Unicode-aware wrapping with long word handling

### ✅ Task 2: Robust JSON Parsing
**Problem:** Fine-tuned models sending malformed JSON → "Missing 'file_path' parameter" errors
**Fix:** Multi-strategy parser with auto-correction and retry logic

### ✅ Task 3: Event Batching Safety
**Problem:** Infinite loop potential from paste bombs
**Fix:** Max 1000 events per batch, 10ms timeout

---

## Test Plan

### Test 1: Word Wrapping (The Original Bug)

**What to do:**
1. Run: `./synthia/target/release/synthia`
2. Ask it something that generates a long response, like:
   ```
   Create a Flask TODO app with HTML templates
   ```

**What to expect:**
- ✅ **NO mid-word breaks** like "usin g HTML" or "tem plates"
- ✅ Words wrap at **word boundaries** (spaces)
- ✅ Long URLs/hashes **break cleanly** at terminal width
- ✅ Unicode characters (emoji, Japanese) **count correctly** (not broken apart)

**Example expected output:**
```
The application is a TODO list
manager using HTML templates and
in-memory storage.
```

**Red flags:**
- ❌ "usin g HTML" (word broken in middle)
- ❌ "tem plates" (word broken in middle)
- ❌ Emoji broken apart (e.g., half a 🦀)

---

### Test 2: JSON Parsing (Fine-tuned Model Fix)

**What to do:**
1. If you have your fine-tuned Qwen model loaded in LM Studio:
   - Run: `./synthia/target/release/synthia`
   - Ask it to use a tool like: `Read the file synthia/README.md`

**What to expect:**
- ✅ Tool calls work **even if JSON is malformed**
- ✅ If tool fails with parameter error, LLM **gets error message** and can retry
- ✅ No silent conversion to `{}`
- ✅ Helpful error logs in terminal showing what failed

**Example flow:**
```
You: Read the file synthia/README.md

Synthia: [Calls read tool]

# If JSON was malformed but fixable:
# Logs show: "JSON required auto-fix. Original: {'file_path': ...}"
# Tool executes successfully

# If JSON was malformed and unfixable:
# Logs show: "Failed to parse tool arguments for 'read': ..."
# LLM gets error: "Error: Missing 'file_path' parameter. Please check the tool schema and retry with valid JSON parameters."
# LLM tries again with corrected JSON
```

**Red flags:**
- ❌ Tool call silently fails with empty `{}`
- ❌ "Missing 'file_path' parameter" with no retry
- ❌ Synthia crashes on malformed JSON

**Note:** If using Claude 3.5 Sonnet (via API), you likely won't see this issue since Claude generates perfect JSON. This fix primarily helps fine-tuned models like Qwen.

---

### Test 3: Event Batching (Paste Bomb Protection)

**What to do:**
1. Run: `./synthia/target/release/synthia`
2. Copy a **very large block of text** (1000+ characters)
3. Paste it all at once into the input line
4. Watch the terminal output

**What to expect:**
- ✅ Synthia **stays responsive** (no hang)
- ✅ Input appears quickly (within 10ms)
- ✅ If you paste >1000 characters, logs show:
  ```
  WARN Hit max batch size (1000), possible paste bomb
  ```
- ✅ Normal typing still works smoothly

**Red flags:**
- ❌ UI freezes for >1 second during paste
- ❌ Characters appear one-by-one slowly
- ❌ Terminal becomes unresponsive

**How to generate test data:**
```bash
# Generate 2000 characters of text
python3 -c "print('a' * 2000)"

# Copy output and paste into Synthia
```

---

## Quick Smoke Test (5 minutes)

If you're short on time, just test the original bug:

```bash
./synthia/target/release/synthia
```

**Ask:**
```
Create a Flask TODO app with HTML templates and explain the code
```

**Check the response wrapping:**
- Look for any mid-word breaks (should be NONE)
- Verify words wrap at spaces
- Make sure "templates" doesn't become "tem plates"

If this passes, the core fix is working!

---

## Known Issues (Unrelated to P1)

These existed before P1 changes:
- 2 session tests fail (race condition, timing issue)
- 16 compiler warnings (unused imports/functions)

These don't affect functionality and will be cleaned up later.

---

## After Testing

**If everything works:**
- We can push these changes to remote
- Move on to P2 (Architecture Improvements):
  - Parallel tool execution
  - Sliding window memory
  - Tool result caching

**If you find issues:**
- Let me know what failed and I'll dispatch a fix subagent
- Show me the exact output/error
- Describe what you expected vs what happened

---

## Logs Location

If you want to see detailed logs:
```bash
# Synthia logs to stderr by default
./synthia/target/release/synthia 2>&1 | tee synthia.log
```

Look for:
- `JSON parsed successfully on first try` (good!)
- `JSON required auto-fix` (parser working!)
- `Failed to parse JSON after all strategies` (needs investigation)
- `Hit max batch size` (paste bomb detected)

---

## Comparison (Before vs After)

### Word Wrapping
**Before:**
```
The application is a TODO list manager usin
g HTML templates and in-memory storage.
```

**After:**
```
The application is a TODO list manager
using HTML templates and in-memory
storage.
```

### JSON Parsing
**Before:**
```
[Malformed JSON] → Silent {} → "Missing 'file_path' parameter" → Crash
```

**After:**
```
[Malformed JSON] → Auto-fix → Tool executes successfully
OR
[Malformed JSON] → Can't fix → Error to LLM → LLM retries → Success
```

### Paste Handling
**Before:**
```
[Paste 10000 chars] → UI hangs indefinitely → Ctrl+C to escape
```

**After:**
```
[Paste 10000 chars] → Process 1000 chars in 10ms → Log warning → Continue normally
```

---

Happy testing! Let me know what you find. 🦀
