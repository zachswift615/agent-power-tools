# Fine-Tuning Dataset Generation - Setup Complete ✅

## Overview

Comprehensive setup for expanding Synthia's fine-tuning dataset from 250 to **2,525+ examples**, incorporating:
- Claude Opus's agentic capabilities recommendations
- Superpowers skills workflows
- Powertools semantic navigation integration

## 🎯 Complete Setup

### 8 Specialized Agents Created

All agents in `.claude/agents/` ready to use with Task tool:

| # | Agent | Examples | Focus |
|---|-------|----------|-------|
| 1 | dataset-generator-tool-use | 500-1000 | Parallel execution, error recovery, multi-turn sequences |
| 2 | dataset-generator-powertools | 150-200 | Semantic navigation (index, goto_definition, find_references) |
| 3 | dataset-generator-superpowers | 300-500 | Skills wiki workflows (brainstorming, TDD, systematic debugging) |
| 4 | dataset-generator-agentic | 300-500 | TDD, debugging, planning, code review, exploration |
| 5 | dataset-generator-craftsmanship | 300-500 | DRY, SOLID, clean code, refactoring patterns |
| 6 | dataset-generator-collaboration | 200-300 | Communication, clarifying questions, explanations |
| 7 | dataset-generator-problem-solving | 200-300 | When stuck, scale game, root cause tracing |
| 8 | fine-tuning-pipeline | N/A | MLX training execution for Mac M1 Pro |

**Total:** ~2,525 examples (expandable to 3,300+)

### Training Infrastructure

**MLX Pipeline** (`fine-tuning/train_mlx.py`):
- Configured for Mac M1 Pro (16GB RAM)
- 3-stage training approach
- Batch size: 2 (memory optimized)
- LoRA rank: 16 (efficient)
- Total training time: ~2-3 hours

**Stage 1:** Tool use reinforcement (~750 examples, 30-40 min)
**Stage 2:** Agentic skills (~1,050 examples, 40-50 min)
**Stage 3:** Full integration (~2,525 examples, 60-90 min)

## 📋 Training Data Categories

### 1. Tool Use Excellence (700 examples)
- Single tool calls with proper formatting
- Parallel tool execution (P2 feature!)
- Multi-turn tool sequences
- Error recovery and retries
- Parameter handling edge cases

### 2. Powertools Integration (175 examples)
**Synthia-specific** via `src/tools/powertools.rs`:
- ✅ `index` - Index projects
- ✅ `definition` - Go to definition
- ✅ `references` - Find references
- ✅ `functions` - List functions
- ✅ `classes` - List classes
- ✅ `stats` - Project statistics

**Key principle:** NEVER grep → ALWAYS use semantic tools

**Not yet integrated** (future work):
- ❌ batch_replace
- ❌ rename_symbol
- ❌ inline_variable
- ❌ File watcher

### 3. Superpowers Skills (450 examples)
From `/Users/zachswift/.config/superpowers/skills/skills/`:

**Collaboration** (150): Brainstorming, writing plans, executing plans, subagent-driven development, code review
**Debugging** (100): Systematic debugging (RED-YELLOW-GREEN), root cause tracing, verification before completion
**Testing** (50): TDD (RED-GREEN-REFACTOR), testing anti-patterns, condition-based waiting
**Problem-Solving** (100): When stuck, scale game, inversion exercise, collision-zone thinking, simplification cascades
**Architecture** (50): Preserving productive tensions, tracing knowledge lineages

**Key behavior:** Explicit skill references ("Let me use the brainstorming skill...")

### 4. Agentic Skills (400 examples)
- TDD workflows (complete RED-GREEN-REFACTOR cycles)
- Systematic debugging (reproduce, isolate, fix, verify)
- Planning & decomposition
- Code review
- Proactive exploration

### 5. Software Craftsmanship (400 examples)
- DRY principle (identify duplication, refactor)
- SOLID principles (all 5)
- Clean code practices (naming, functions, comments)
- Refactoring patterns (extract method, simplify conditionals)

**Languages:** Rust (primary), TypeScript, Python, Go

### 6. Collaboration & Communication (200 examples)
- Clarifying questions (ask when ambiguous)
- Progress communication (status updates, findings)
- Explaining technical decisions (trade-offs, alternatives)
- Admitting uncertainty (appropriate confidence levels)
- Proactive suggestions (tests, docs, improvements)

### 7. Problem-Solving Patterns (200 examples)
- When stuck (try alternatives, break down differently)
- Scale game (test at 10x, 100x, 1000x)
- Root cause tracing (errors → triggers)
- Self-verification (double-check work)
- Inversion exercise (challenge assumptions)
- Pattern recognition (similar bugs across files)

## 🚀 How to Use

### Generate Examples

**Single agent:**
```
Use the dataset-generator-tool-use agent to create 200 parallel tool execution examples
```

**Multiple agents in parallel:**
```
Launch these agents in parallel:
1. dataset-generator-tool-use - create 200 parallel execution examples
2. dataset-generator-powertools - create 100 semantic navigation examples
3. dataset-generator-superpowers - create 150 collaboration skills examples
```

**Important:** All agents append to `fine-tuning/dataset.jsonl` - they don't overwrite!

### Run Training

```bash
cd fine-tuning
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
python train_mlx.py --stage all --test
```

## 📊 Expected Improvements

After fine-tuning with full dataset:

| Capability | Before | After |
|------------|--------|-------|
| Tool calling accuracy | Good | 95%+ (reinforced) |
| Multi-tool usage | Basic | Advanced parallel execution |
| Error handling | Basic | Sophisticated retry strategies |
| Code quality | Good | Proactive DRY/SOLID suggestions |
| TDD workflow | None | Natural RED-GREEN-REFACTOR |
| Debugging | Ad-hoc | Systematic RED-YELLOW-GREEN |
| Brainstorming | None | Refines ideas before coding |
| Powertools usage | None | Prefers semantic over grep |
| Communication | Basic | Clear explanations, asks when unclear |
| Problem-solving | Basic | Scale game, inversion, systematic approaches |

## 📁 Files Created

```
synthia/
├── .claude/agents/
│   ├── dataset-generator-tool-use.md ⭐
│   ├── dataset-generator-powertools.md ⭐
│   ├── dataset-generator-superpowers.md ⭐
│   ├── dataset-generator-agentic.md ⭐
│   ├── dataset-generator-craftsmanship.md ⭐
│   ├── dataset-generator-collaboration.md ⭐
│   ├── dataset-generator-problem-solving.md ⭐
│   └── fine-tuning-pipeline.md ⭐
├── fine-tuning/
│   ├── train_mlx.py ⭐ (MLX training script)
│   ├── requirements.txt ⭐
│   ├── README.md ⭐ (comprehensive guide)
│   └── dataset.jsonl (250 → 2,525+ examples)
├── FINE_TUNING_PLAN.md ✅ (updated)
├── SUPERPOWERS_SKILLS_MAPPING.md ⭐ (skills → training data mapping)
└── DATASET_GENERATION_SUMMARY.md ⭐ (this file)
```

## 🎯 Recommended Workflow

### Week 1: Dataset Generation (20-30 hours)

1. **Tool Use** (8-10 hours) → 700 examples
2. **Powertools** (4-5 hours) → 175 examples
3. **Superpowers Skills** (8-10 hours) → 450 examples

### Week 2: Advanced Categories + Training (15-20 hours)

4. **Agentic Skills** (6-8 hours) → 400 examples
5. **Craftsmanship** (6-8 hours) → 400 examples
6. **First Training Run** (3-4 hours) → Stage 1 model

### Week 3: Final Categories + Validation (10-15 hours)

7. **Collaboration & Problem-Solving** (5-7 hours) → 400 examples
8. **Full Training** (4-6 hours) → Stages 2 & 3
9. **Testing & Iteration** (5-10 hours) → Validation, upload to HuggingFace

**Total: 50-70 hours** over 2-3 weeks

## 🔑 Key Innovations

### 1. Powertools Integration ⭐
Train Synthia to prefer semantic tools over text search:
- goto_definition instead of grep for definitions
- find_references instead of grep for usages
- search_ast instead of grep for code patterns

### 2. Superpowers Skills ⭐
Extract workflows from your skills wiki:
- Brainstorming before coding
- Systematic debugging (RED-YELLOW-GREEN)
- TDD discipline (RED-GREEN-REFACTOR)
- Scale game for edge cases
- When stuck patterns

### 3. Multi-Stage Training ⭐
Progressive skill building:
- Stage 1: Reinforce tool use + semantic navigation
- Stage 2: Add agentic workflows
- Stage 3: Full integration with all skills

### 4. Mac-Optimized Pipeline ⭐
MLX configured for M1 Pro:
- Batch size 2 (safe for 16GB)
- Gradient checkpointing
- LoRA rank 16
- ~2-3 hour total training time

## 🚨 Important Notes

1. **Current dataset:** 250 examples (already trained once)
2. **Base model:** `zachswift615/qwen2.5-coder-synthia-tool-use`
3. **This is continued fine-tuning** - lower learning rate to preserve existing knowledge
4. **Always preview batch operations** before applying
5. **File watcher** is NOT yet integrated into Synthia (future work)

## 📚 Documentation References

- **Fine-Tuning Plan:** `FINE_TUNING_PLAN.md`
- **Superpowers Mapping:** `SUPERPOWERS_SKILLS_MAPPING.md`
- **MLX Setup Guide:** `fine-tuning/README.md`
- **Agent Descriptions:** `.claude/agents/*.md`

## ✅ Setup Status

- [x] 8 specialized agents created
- [x] MLX training pipeline configured
- [x] Comprehensive plan documented
- [x] Superpowers skills mapped
- [x] Powertools integration identified
- [ ] Dataset generation (pending)
- [ ] Training execution (pending)
- [ ] Model validation (pending)
- [ ] HuggingFace upload (pending)

**You're ready to start generating training data!** 🚀

## 💡 Next Step

Start generating examples with the first agent:

```
Use the dataset-generator-tool-use agent to create 200 examples of parallel tool execution patterns
```

Or dispatch multiple agents in parallel for faster progress.
