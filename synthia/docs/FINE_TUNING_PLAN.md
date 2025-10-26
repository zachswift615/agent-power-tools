# Comprehensive Plan: Fix UX Issues + Fine-Tune Qwen Model

## Phase 1: Fix Critical UX Issues (Week 1)
**Priority: IMMEDIATE** - Makes Synthia usable

### Task 1.1: Fix Input Line Duplication (High Priority)
- Investigate `render_input()` in `synthia/src/ui/app.rs`
- Add `clear_input_line()` before every render
- Fix cursor positioning logic
- Test with long inputs and paste

### Task 1.2: Fix Tool Output Alignment (Medium Priority)
- Fix `UIUpdate::ToolResult` handler in `synthia/src/ui/app.rs`
- Remove excessive indentation from Command/Output labels
- Ensure left-aligned, readable formatting

**Expected time:** 2-4 hours
**Tools needed:** Rust compiler, terminal testing

---

## Phase 2: Create Fine-Tuning Training Dataset (Week 1-2)
**Priority: HIGH** - Foundation for model improvement

### Dataset Categories (Based on Claude Opus's Recommendations + Superpowers Skills)

#### 1. **Tool Use Excellence** (500-1000 examples)
**Agent:** `.claude/agents/dataset-generator-tool-use.md`

**Sub-categories:**
- **Single Tool Calls** (150 examples): read, write, edit, bash, grep, glob with proper formatting
- **Parallel Tool Execution** (200 examples): Multiple independent tool calls in one response
- **Multi-turn Tool Sequences** (150 examples): read → analyze → edit → test patterns
- **Error Recovery** (100 examples): File not found, malformed JSON, permission denied, retries
- **Parameter Handling** (100 examples): Missing params, invalid paths, edge cases

**Key behaviors:**
- Proper tool selection for each task
- Clear descriptions for bash commands
- Parallel execution when tools are independent
- Graceful error handling and recovery
- Natural conversation flow with tool calls

#### 2. **Agentic Skills** (300-500 examples)
**Agent:** `.claude/agents/dataset-generator-agentic.md`

**Sub-categories:**
- **TDD Workflow** (150 examples): Complete RED-GREEN-REFACTOR cycles
- **Systematic Debugging** (100 examples): Reproduce, isolate, hypothesize, test, fix, verify
- **Planning & Decomposition** (100 examples): Break tasks into subtasks, identify dependencies
- **Code Review** (50 examples): Analyze changes, identify issues, suggest improvements
- **Proactive Exploration** (100 examples): Explore before changing, verify assumptions, test edge cases

**Key behaviors:**
- Think step-by-step (show reasoning)
- Don't give up on first error
- Try multiple approaches
- Self-verification (run tests after changes)
- Learn from failures

#### 3. **Software Craftsmanship** (300-500 examples)
**Agent:** `.claude/agents/dataset-generator-craftsmanship.md`

**Sub-categories:**
- **DRY Principle** (100 examples): Identify duplication, extract common logic, create reusable utilities
- **SOLID Principles** (100 examples): Single Responsibility, Open/Closed, Liskov Substitution, etc.
- **Clean Code Practices** (100 examples): Meaningful names, small functions, clear comments, remove dead code
- **Refactoring Patterns** (100 examples): Extract method, rename, simplify conditionals, consolidate logic

**Languages covered:**
- Rust (primary - Synthia's codebase)
- TypeScript/JavaScript
- Python
- Go (bonus)

**Key behaviors:**
- Identify code smells proactively
- Explain WHY refactoring improves code
- Show incremental improvements
- Maintain tests during refactoring
- Use language-specific idioms

#### 4. **Collaboration & Communication** (200-300 examples)
**Agent:** `.claude/agents/dataset-generator-collaboration.md`

**Sub-categories:**
- **Clarifying Questions** (100 examples): Ask when ambiguous, present multiple approaches, probe constraints
- **Progress Communication** (75 examples): Status updates, explain reasoning, highlight findings, warn about issues
- **Explaining Technical Decisions** (50 examples): Clear rationale, trade-offs, alternatives, implications
- **Admitting Uncertainty** (25 examples): Express confidence levels, suggest alternatives, ask instead of guess
- **Proactive Suggestions** (50 examples): Suggest tests, docs, improvements, PRs, commit messages

**Key behaviors:**
- Helpful without being presumptuous
- Clear communication without verbosity
- Proactive but not overstepping
- Confident but humble
- Technical but accessible

#### 5. **Problem-Solving Patterns** (200-300 examples)
**Agent:** `.claude/agents/dataset-generator-problem-solving.md`

**Sub-categories:**
- **When Stuck** (50 examples): Try alternatives, break down differently, search patterns, consult docs
- **Scale Game / Edge Cases** (75 examples): Empty inputs, large inputs, special chars, concurrent access, network failures
- **Root Cause Tracing** (50 examples): Trace errors back to triggers, follow propagation, identify original invalid input
- **Self-Verification** (50 examples): Double-check work, run tests, validate assumptions, catch own mistakes
- **Inversion Exercise** (25 examples): Challenge assumptions, question "the only way", consider opposites
- **Pattern Recognition** (50 examples): Notice similar bugs, recognize anti-patterns, draw parallels

**Key behaviors:**
- Persistence through challenges
- Systematic rather than random attempts
- Learning from failed attempts
- Creative problem-solving
- Knowing when to escalate

#### 6. **Superpowers Skills** (300-500 examples)
**Agent:** `.claude/agents/dataset-generator-superpowers.md`

**Sub-categories:**
- **Collaboration Skills** (150 examples): Brainstorming, writing plans, executing plans, subagent-driven development, code review workflows, git worktrees
- **Debugging Skills** (100 examples): Systematic debugging (RED-YELLOW-GREEN), root cause tracing, verification before completion, defense in depth
- **Testing Skills** (50 examples): TDD (RED-GREEN-REFACTOR), testing anti-patterns, condition-based waiting
- **Problem-Solving Skills** (100 examples): When stuck, scale game, inversion exercise, collision-zone thinking, simplification cascades, meta-pattern recognition
- **Architecture Skills** (50 examples): Preserving productive tensions, tracing knowledge lineages

**Skills location:** `/Users/zachswift/.config/superpowers/skills/skills/`

**Key behaviors:**
- Reference skills by name ("Let me use the brainstorming skill...")
- Follow exact workflows from skill documentation
- Show **when** to use each skill (trigger conditions)
- Demonstrate **why** behind each step
- Include both success and error cases
- Multi-turn conversations showing skills in action

**Example workflows:**
- Brainstorming before coding (refine vague ideas into solid designs)
- Systematic debugging (reproduce → isolate → hypothesize → test → fix → verify)
- TDD workflow (write test → see it fail → implement → see it pass → refactor)
- Scale game (test at 10x, 100x, 1000x to find breaking points)
- When stuck (try alternative approaches, break down differently, search patterns)

### Advanced Capabilities (From Claude Opus)

#### 7. **Meta-Learning & Adaptation** (Integrated into all categories)
- Adjust to user's expertise level
- Learn from corrections
- Recognize patterns in user preferences
- Adapt verbosity based on context
- Remember user's preferred tools/libraries

#### 8. **Domain-Specific Intelligence** (Integrated into craftsmanship examples)
- Language/framework idioms and best practices
- Security consciousness (input validation, SQL injection prevention, XSS protection)
- Performance optimization (algorithm complexity, caching, query optimization)

#### 9. **Powertools Integration** (150-200 examples)
**Agent:** `.claude/agents/dataset-generator-powertools.md`

**Sub-categories:**
- **Semantic Navigation** (100 examples): index, goto_definition, find_references, search_ast, list_functions, list_classes, project_stats
- **Tool Parameter Variations** (50 examples): Different parameter combinations for each operation
- **Combined Workflows** (50 examples): Powertools + TDD, Powertools + debugging, Powertools + refactoring

**Synthia's powertools integration:**
- Direct integration via `src/tools/powertools.rs`
- Shells out to `../powertools-cli/target/release/powertools`
- Currently supports: index, definition, references, functions, classes, stats
- Advanced features (batch_replace, rename_symbol, inline_variable, watcher) not yet integrated

**Key behaviors:**
- NEVER grep for definitions → ALWAYS use powertools definition
- NEVER grep for usages → ALWAYS use powertools references
- NEVER grep for code structures → ALWAYS use powertools search_ast
- Use semantic tools for precision, not text matching
- Explain WHY powertools are better (semantic vs text, precision vs guessing)

**Example workflows:**
- Find where function is defined: goto_definition instead of grep
- Find all usages: find_references instead of grep
- List all async functions: search_ast instead of grep pattern matching
- Get codebase stats: project_stats instead of manual counting

**Total: ~2,000-3,300 training examples** (increased with powertools integration)

### Specialized Agents Created

All agents are located in `.claude/agents/` and can be dispatched with the Task tool:

1. **dataset-generator-tool-use.md** - Tool usage patterns (parallel execution, error recovery)
2. **dataset-generator-powertools.md** - Powertools semantic navigation (goto_definition, find_references, etc.)
3. **dataset-generator-superpowers.md** - Superpowers skills workflows (brainstorming, systematic debugging, etc.)
4. **dataset-generator-agentic.md** - TDD, debugging, planning, code review
5. **dataset-generator-craftsmanship.md** - DRY, SOLID, clean code, refactoring
6. **dataset-generator-collaboration.md** - Communication, clarifying questions, explanations
7. **dataset-generator-problem-solving.md** - Problem-solving techniques (when stuck, scale game)
8. **fine-tuning-pipeline.md** - MLX setup and training execution

### Data Format
Convert to OpenAI chat format:
```json
{
  "messages": [
    {"role": "system", "content": "You are Synthia..."},
    {"role": "user", "content": "Read the file..."},
    {"role": "assistant", "content": "I'll read that file for you.", "tool_calls": [...]},
    {"role": "tool", "content": "...", "tool_call_id": "..."},
    {"role": "assistant", "content": "The file contains..."}
  ]
}
```

---

## Phase 3: Setup Fine-Tuning Environment (Mac M1 Pro)
**Priority: HIGH** - Technical setup complete!

### MLX on Mac M1 Pro (CHOSEN APPROACH)
**Agent:** `.claude/agents/fine-tuning-pipeline.md`

**Hardware Specs:**
- MacBook Pro M1 Pro
- 10-core CPU (8 performance + 2 efficiency)
- 16-core GPU
- 16 GB unified memory

**Why MLX:**
- All-in-one machine (no computer switching)
- Optimized for Apple Silicon (fast training)
- Fast iteration (no file transfers)
- Native Metal acceleration

**Setup (Complete):**

1. **Create virtual environment:**
```bash
cd fine-tuning
python3 -m venv venv
source venv/bin/activate
```

2. **Install dependencies:**
```bash
pip install -r requirements.txt
```

3. **Base model:**
   - Already fine-tuned once: `zachswift615/qwen2.5-coder-synthia-tool-use`
   - Local GGUF: `/Users/zachswift/.lmstudio/models/zachswift615/qwen2.5-coder-synthia-tool-use/model-q4_k_m.gguf`
   - HuggingFace: Downloads automatically on first use

4. **Training script:**
   - Location: `fine-tuning/train_mlx.py`
   - Executable: `chmod +x fine-tuning/train_mlx.py`
   - Configured for 16GB RAM with batch_size=2

**Memory Optimization:**
- Batch size: 2 (safe for 16GB)
- Max sequence length: 2048
- Gradient checkpointing: Enabled
- LoRA: 16-layer, rank 16 (memory efficient)

**Resources:**
- Tutorial: https://apeatling.com/articles/simple-guide-to-local-llm-fine-tuning-on-a-mac-with-mlx/
- MLX repo: https://github.com/ml-explore/mlx-lm
- Qwen fine-tuning: https://unsloth.ai/blog/qwen-coder

---

## Phase 4: Fine-Tune Qwen Model (Week 2-3)
**Priority: HIGH** - The game changer

### Fine-Tuning Approach
**Method:** LoRA (Low-Rank Adaptation) with MLX
- Only trains ~0.1% of parameters
- Fast training on M1 Pro (30-90 min per stage)
- Efficient memory usage (optimized for 16GB RAM)
- Continued fine-tuning from already-tuned model

### Training Configuration (MLX for Mac M1 Pro)
```python
{
  "model": "zachswift615/qwen2.5-coder-synthia-tool-use",  # Continued fine-tuning
  "data": "fine-tuning/dataset.jsonl",
  "batch_size": 2,  # Optimized for 16GB RAM
  "learning_rate": 2e-5,  # Lower LR for continued fine-tuning
  "max_seq_length": 2048,
  "grad_checkpoint": True,

  # LoRA config
  "lora_layers": 16,
  "lora_rank": 16,
  "lora_alpha": 32,
  "lora_dropout": 0.1
}
```

### Multi-Stage Training Strategy

**Stage 1: Tool Use Reinforcement**
- Dataset: 250 existing + 500 new = ~750 examples
- Iterations: 800
- Learning rate: 2e-5
- Focus: Reinforce tool calling, parallel execution, error recovery
- Time: ~30-40 minutes
- Output: `fine-tuning/models/synthia-stage1`

**Stage 2: Agentic Skills**
- Dataset: Stage 1 + 300 agentic = ~1,050 examples
- Iterations: 1,000
- Learning rate: 1.5e-5
- Focus: TDD workflows, systematic debugging, planning
- Time: ~40-50 minutes
- Output: `fine-tuning/models/synthia-stage2`

**Stage 3: Full Integration**
- Dataset: All categories = ~1,500-2,600 examples
- Iterations: 1,500
- Learning rate: 1e-5
- Focus: Craftsmanship, collaboration, problem-solving
- Time: ~60-90 minutes
- Output: `fine-tuning/models/synthia-stage3`

**Total Training Time:** ~2-3 hours on Mac M1 Pro

### Running Training

**All stages:**
```bash
cd fine-tuning
source venv/bin/activate
python train_mlx.py --stage all --test
```

**Individual stage:**
```bash
python train_mlx.py --stage 1
python train_mlx.py --stage 2
python train_mlx.py --stage 3
```

### Expected Improvements
- **Tool calling accuracy:** Current good → 95%+ (reinforcement)
- **Multi-tool usage:** Improved parallel execution patterns
- **Error handling:** Better recovery from errors, retry strategies
- **Code quality:** Proactive DRY/SOLID suggestions
- **Proactiveness:** Suggests tests, asks clarifying questions, explores before coding
- **TDD workflow:** Natural RED-GREEN-REFACTOR cycles
- **Debugging:** Systematic approach, root cause tracing
- **Communication:** Clear explanations, admits uncertainty appropriately

### Validation Tests

After each stage, test key behaviors:

```bash
# Tool calling
mlx_lm.generate --model models/synthia-stage1 --prompt "Read src/main.rs and find async functions"

# TDD workflow
mlx_lm.generate --model models/synthia-stage2 --prompt "Add a divide function using TDD"

# Refactoring
mlx_lm.generate --model models/synthia-stage3 --prompt "Refactor this code to follow DRY"
```

### Converting to GGUF (for LM Studio)

After training, convert to GGUF format:

```bash
# Requires llama.cpp
python llama.cpp/convert.py \
  fine-tuning/models/synthia-stage3 \
  --outfile synthia-v2-f16.gguf

# Quantize for smaller size
llama.cpp/quantize synthia-v2-f16.gguf synthia-v2-q4_k_m.gguf q4_k_m

# Move to LM Studio
cp synthia-v2-q4_k_m.gguf ~/.lmstudio/models/zachswift615/qwen2.5-coder-synthia-v2/
```

---

## Phase 5: Test & Iterate (Week 3)
**Priority: HIGH** - Validation

### Testing Protocol
1. **Baseline comparison:** Test old vs new model
2. **Tool calling tests:** Single, multiple, parallel, error cases
3. **Agentic workflow tests:** TDD, debugging, planning
4. **Code quality tests:** Refactoring, testing, documentation
5. **UX validation:** Does it feel better to use?

### Iteration Loop
- Identify failure patterns
- Add more training examples for weak areas
- Re-train with augmented dataset
- Repeat until satisfied

---

## Success Metrics

### UX Metrics (Phase 1)
- ✅ No input line duplication
- ✅ Tool output properly aligned
- ✅ Readable, non-dizzying interface

### Model Metrics (Phase 4-5)
- ✅ Tool calling success rate >95%
- ✅ Follows TDD workflow without prompting
- ✅ Writes clean, DRY code
- ✅ Asks clarifying questions proactively
- ✅ Handles errors gracefully

### Overall Goal
**Synthia becomes your go-to local coding assistant** - better than default Qwen, competitive with Claude for local use cases.

---

## Why This Approach Works

1. **Fix UX first** - Can't evaluate model if interface is unusable
2. **Comprehensive dataset** - Covers all aspects of good AI assistant behavior
3. **LoRA efficiency** - Fast iterations, low resource usage
4. **Superpowers integration** - Training data based on proven best practices
5. **Existing pipeline** - Use what works, avoid unnecessary setup

---

## Estimated Timeline
- **Week 1:** Fix UX (2-4 hours) + Create dataset (20-30 hours)
- **Week 2:** Train on existing Windows pipeline (4-6 hours total)
- **Week 3:** Test & iterate (10-15 hours)

**Total: ~40-60 hours of focused work**

---

## Training Dataset Structure

### Example 1: Single Tool Call with Good Practices
```json
{
  "messages": [
    {
      "role": "system",
      "content": "You are Synthia, a helpful coding assistant. Follow TDD, DRY, and SOLID principles. Ask clarifying questions when needed."
    },
    {
      "role": "user",
      "content": "Read the file src/main.rs and tell me what it does"
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
      "content": "fn main() {\n    println!(\"Hello, world!\");\n}"
    },
    {
      "role": "assistant",
      "content": "This is the main entry point for a Rust application. It contains a simple `main()` function that prints 'Hello, world!' to stdout. This is a minimal Rust program - just the standard 'Hello World' example."
    }
  ]
}
```

### Example 2: Multiple Parallel Tool Calls
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Compare src/lib.rs and tests/test.rs - what's the relationship?"
    },
    {
      "role": "assistant",
      "content": "I'll read both files to analyze their relationship.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "read",
            "arguments": "{\"file_path\": \"src/lib.rs\"}"
          }
        },
        {
          "id": "call_2",
          "type": "function",
          "function": {
            "name": "read",
            "arguments": "{\"file_path\": \"tests/test.rs\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}"
    },
    {
      "role": "tool",
      "tool_call_id": "call_2",
      "content": "use mylib::add;\n\n#[test]\nfn test_add() {\n    assert_eq!(add(2, 2), 4);\n}"
    },
    {
      "role": "assistant",
      "content": "The relationship is clear:\n\n- **src/lib.rs** defines a public `add()` function that adds two integers\n- **tests/test.rs** imports and tests that function\n\nThis follows Rust's standard testing pattern where:\n1. Library code goes in `src/lib.rs`\n2. Integration tests go in `tests/`\n3. Tests import the library with `use mylib::add`\n\nThe test verifies `add(2, 2) == 4`, which is a good basic sanity check."
    }
  ]
}
```

### Example 3: TDD Workflow
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Add a subtract function to our calculator library"
    },
    {
      "role": "assistant",
      "content": "I'll follow TDD - test first, then implementation. Let me write the test.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "edit",
            "arguments": "{\"file_path\": \"tests/test.rs\", \"old_string\": \"#[test]\\nfn test_add() {\\n    assert_eq!(add(2, 2), 4);\\n}\", \"new_string\": \"#[test]\\nfn test_add() {\\n    assert_eq!(add(2, 2), 4);\\n}\\n\\n#[test]\\nfn test_subtract() {\\n    assert_eq!(subtract(5, 3), 2);\\n}\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "File edited successfully"
    },
    {
      "role": "assistant",
      "content": "Test written. Now let me run it to see it fail (RED phase).",
      "tool_calls": [
        {
          "id": "call_2",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"cargo test test_subtract\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_2",
      "content": "error[E0425]: cannot find function `subtract` in this scope"
    },
    {
      "role": "assistant",
      "content": "Perfect - test fails as expected (RED). Now implementing the function (GREEN phase).",
      "tool_calls": [
        {
          "id": "call_3",
          "type": "function",
          "function": {
            "name": "edit",
            "arguments": "{\"file_path\": \"src/lib.rs\", \"old_string\": \"pub fn add(a: i32, b: i32) -> i32 {\\n    a + b\\n}\", \"new_string\": \"pub fn add(a: i32, b: i32) -> i32 {\\n    a + b\\n}\\n\\npub fn subtract(a: i32, b: i32) -> i32 {\\n    a - b\\n}\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_3",
      "content": "File edited successfully"
    },
    {
      "role": "assistant",
      "content": "Implementation added. Running tests to verify (GREEN phase).",
      "tool_calls": [
        {
          "id": "call_4",
          "type": "function",
          "function": {
            "name": "bash",
            "arguments": "{\"command\": \"cargo test\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_4",
      "content": "test test_add ... ok\ntest test_subtract ... ok\n\ntest result: ok. 2 passed"
    },
    {
      "role": "assistant",
      "content": "✅ All tests pass! TDD cycle complete:\n\n1. **RED:** Wrote failing test\n2. **GREEN:** Implemented minimal code to pass\n3. **REFACTOR:** Not needed - code is already simple\n\nThe `subtract` function is now part of your calculator library."
    }
  ]
}
```

### Example 4: Error Handling and Recovery
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Read config.toml"
    },
    {
      "role": "assistant",
      "content": "I'll read that file.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "read",
            "arguments": "{\"file_path\": \"config.toml\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "Error: No such file or directory (os error 2)"
    },
    {
      "role": "assistant",
      "content": "The file `config.toml` doesn't exist in the current directory. Would you like me to:\n\n1. Create a new `config.toml` file\n2. Search for config files in subdirectories\n3. List what files are in the current directory\n\nWhich would be most helpful?"
    }
  ]
}
```

### Example 5: Asking Clarifying Questions
```json
{
  "messages": [
    {
      "role": "user",
      "content": "Add logging to the server"
    },
    {
      "role": "assistant",
      "content": "I can help with that! Before I start, I need to clarify a few things:\n\n1. **Which logging library?** (e.g., `tracing`, `env_logger`, `log`, `slog`)\n2. **What level of detail?** (debug, info, warn, error)\n3. **Where should it log?** (stdout, file, both)\n4. **Which parts of the server?** (requests, errors, startup, all events)\n\nLet me check what's currently in the project to make a recommendation.",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "read",
            "arguments": "{\"file_path\": \"Cargo.toml\"}"
          }
        }
      ]
    }
  ]
}
```

---

## Dataset Generation Strategy

### Automated Generation
1. **Extract from Superpowers Skills**
   - Parse all skill markdown files
   - Convert examples into training format
   - Add tool call examples where appropriate

2. **Claude Code Transcripts**
   - Mine your existing Claude Code sessions
   - Extract good tool use patterns
   - Clean and format for training

3. **Synthetic Examples**
   - Use Claude API to generate variations
   - Cover edge cases and error scenarios
   - Ensure diverse tool combinations

### Manual Curation
1. **High-quality examples** for critical patterns
2. **Edge cases** that models often get wrong
3. **Domain-specific** Rust/Synthia scenarios

### Quality Criteria
- ✅ Correct JSON formatting
- ✅ Realistic user requests
- ✅ Helpful, clear responses
- ✅ Proper error handling
- ✅ Follows best practices (TDD, DRY, SOLID)

---

## Quick Start: Generating Training Data

### Using the Specialized Agents

All dataset generation agents are in `.claude/agents/`. Use the Task tool to dispatch them:

```
# Example: Generate 100 tool use examples
"Use the dataset-generator-tool-use agent to create 100 examples of parallel tool execution patterns"

# Example: Generate TDD examples
"Use the dataset-generator-agentic agent to create 50 complete RED-GREEN-REFACTOR TDD workflow examples"

# Example: Generate refactoring examples
"Use the dataset-generator-craftsmanship agent to create 75 DRY principle examples in Rust and TypeScript"
```

**Important:** Agents append to `fine-tuning/dataset.jsonl` - they don't overwrite!

### Recommended Order

1. **Tool Use** (500-1000 examples) - Highest priority, reinforces existing strength
2. **Powertools** (150-200 examples) - Semantic navigation (Synthia-specific integration)
3. **Superpowers Skills** (300-500 examples) - Core workflows from skills wiki (brainstorming, systematic debugging, TDD)
4. **Agentic Skills** (300-500 examples) - General agentic behaviors (planning, code review, exploration)
5. **Craftsmanship** (300-500 examples) - Code quality improvements (DRY, SOLID, refactoring)
6. **Collaboration** (200-300 examples) - Communication patterns
7. **Problem-Solving** (200-300 examples) - Advanced techniques

### Parallelizing Work

You can dispatch multiple agents in parallel:

```
"Launch these agents in parallel:
1. dataset-generator-tool-use - create 200 parallel tool execution examples
2. dataset-generator-agentic - create 100 TDD workflow examples
3. dataset-generator-craftsmanship - create 100 DRY refactoring examples"
```

## Next Steps

### Week 1: Dataset Generation (20-30 hours)

1. **Phase 1: Tool Use Examples** (8-10 hours)
   - 150 single tool calls
   - 200 parallel tool executions
   - 150 multi-turn sequences
   - 100 error recovery examples
   - 100 parameter handling examples
   - **Deliverable:** 700 new tool use examples

2. **Phase 2: Agentic Skills** (6-8 hours)
   - 150 TDD workflows
   - 100 systematic debugging examples
   - 100 planning/decomposition examples
   - 50 code review examples
   - **Deliverable:** 400 agentic examples

3. **Phase 3: Superpowers Skills** (8-10 hours)
   - 150 collaboration skills (brainstorming, plans, subagents, code review)
   - 100 debugging skills (systematic debugging, root cause tracing)
   - 50 testing skills (TDD, anti-patterns, condition-based waiting)
   - 100 problem-solving skills (when stuck, scale game, inversion, simplification)
   - 50 architecture skills (productive tensions, knowledge lineages)
   - **Deliverable:** 450 Superpowers skills examples

### Week 2: Advanced Categories + Training (15-20 hours)

4. **Phase 4: Craftsmanship & Communication** (6-8 hours)
   - 100 DRY examples
   - 100 SOLID examples
   - 100 clean code examples
   - 100 refactoring patterns
   - 200 collaboration/communication examples
   - **Deliverable:** 600 craftsmanship + communication examples

5. **Phase 5: Problem-Solving Patterns** (4-5 hours)
   - 200 general problem-solving examples
   - **Deliverable:** 200 problem-solving examples

6. **Phase 6: First Training Run** (3-4 hours)
   - Run Stage 1 with ~1,200 examples (tool use + Superpowers skills)
   - Validate tool calling improvements
   - Test parallel execution behavior
   - Test skill workflows (brainstorming, systematic debugging)
   - **Deliverable:** synthia-stage1 model

7. **Phase 7: Full Training** (4-6 hours)
   - Run Stage 2 with full agentic dataset (~2,000 examples)
   - Run Stage 3 with all categories (~2,400-3,100 examples)
   - Validate end-to-end behavior
   - **Deliverable:** synthia-stage3 final model

### Week 3: Testing & Iteration (10-15 hours)

8. **Validation & Benchmarking**
   - Test against baseline model
   - Measure tool calling accuracy
   - Validate TDD workflow adoption (RED-GREEN-REFACTOR)
   - Test systematic debugging (RED-YELLOW-GREEN)
   - Test brainstorming workflow (refine before implementing)
   - Test code quality suggestions (DRY, SOLID)
   - Test scale game (edge case identification)
   - Identify weak areas

9. **Iteration**
   - Add targeted examples for weak areas
   - Re-train with augmented dataset
   - Upload to HuggingFace as v2
   - Convert to GGUF for LM Studio

### Total Timeline: 2-3 weeks (50-70 hours)

**Dataset breakdown:**
- Tool Use: 700 examples
- Powertools: 175 examples (avg of 150-200)
- Superpowers Skills: 450 examples
- Agentic Skills: 400 examples
- Craftsmanship: 400 examples
- Collaboration: 200 examples
- Problem-Solving: 200 examples
- **Total: ~2,525 examples** (expandable to 3,300+)

---

## Resources

### Fine-Tuning Tutorials
- OpenAI Function Calling: https://cookbook.openai.com/examples/fine_tuning_for_function_calling
- MLX on Mac: https://apeatling.com/articles/simple-guide-to-local-llm-fine-tuning-on-a-mac-with-mlx/
- Qwen2.5-Coder Fine-tuning: https://unsloth.ai/blog/qwen-coder

### Datasets
- Glaive Function Calling v2: https://huggingface.co/datasets/glaiveai/glaive-function-calling-v2
- LLM Datasets Repo: https://github.com/mlabonne/llm-datasets

### Tools
- MLX Framework: https://github.com/ml-explore/mlx-lm
- Hugging Face Hub: https://huggingface.co/docs/hub

---

**Status:** Ready to execute
**Next Action:** Fix UX issues, then start building dataset
