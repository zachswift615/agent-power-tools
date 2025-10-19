# Superpowers Skills → Fine-Tuning Training Data Mapping

This document maps the Superpowers skills wiki to training data categories for Synthia fine-tuning.

## Skills Location

All skills are located at: `/Users/zachswift/.config/superpowers/skills/skills/`

## Category Mapping

### 1. Collaboration Skills → Training Examples (150 examples)

| Skill | When to Use | Training Focus |
|-------|-------------|----------------|
| `collaboration/brainstorming/SKILL.md` | Partner describes any feature/project idea, before writing code | Refining vague ideas into solid designs with clarifying questions |
| `collaboration/writing-plans/SKILL.md` | Design is complete, need detailed implementation tasks | Breaking down complex features into step-by-step plans |
| `collaboration/executing-plans/SKILL.md` | Partner provides complete implementation plan | Executing plans in controlled batches with review checkpoints |
| `collaboration/subagent-driven-development/SKILL.md` | Executing plans with independent tasks in current session | Using Task tool to dispatch specialized subagents |
| `collaboration/requesting-code-review/SKILL.md` | Completing tasks, implementing major features, before merging | Verifying work meets requirements, asking for review |
| `collaboration/receiving-code-review/SKILL.md` | Receiving code review feedback | Processing feedback, asking clarifying questions |
| `collaboration/finishing-a-development-branch/SKILL.md` | Implementation complete, all tests pass | Deciding how to integrate work (merge, PR, squash) |
| `collaboration/using-git-worktrees/SKILL.md` | Starting feature work that needs isolation | Using git worktrees for parallel development |

**Training Pattern Example:**
```
User: "I want to add caching to the API"
Assistant: "Before implementing, let me use the brainstorming skill to refine this idea.

Questions to explore:
1. What to cache? (responses, queries, computed values?)
2. Cache invalidation? (TTL, event-based, manual?)
3. Storage backend? (Redis, in-memory, database?)
4. Cache key design? (handle params, auth, versions?)

Let me ask some clarifying questions: ..."
```

### 2. Debugging Skills → Training Examples (100 examples)

| Skill | When to Use | Training Focus |
|-------|-------------|----------------|
| `debugging/systematic-debugging/SKILL.md` | Encountering any bug, test failure, unexpected behavior | **RED-YELLOW-GREEN workflow**: Reproduce (RED) → Form hypothesis (YELLOW) → Test & fix (GREEN) |
| `debugging/root-cause-tracing/SKILL.md` | Errors occur deep in execution | Tracing back to find original trigger, not just symptoms |
| `debugging/verification-before-completion/SKILL.md` | About to claim work is complete/fixed/passing | **Never** claim done without verification - run tests, check outputs |
| `debugging/defense-in-depth/SKILL.md` | Invalid data causes failures deep in execution | Validation at multiple system layers (input, processing, output) |

**Training Pattern Example:**
```
User: "Server crashes with 'connection pool exhausted'"
Assistant: "I'll use systematic debugging.

**RED Phase: Reproduce & Document**
[reads logs, greps for errors]

**YELLOW Phase: Form Hypothesis**
Evidence:
- Pool size is 10
- Connections exhausted
- Timeouts waiting

Hypothesis: Connections not returned to pool (leak)

Let me trace where connections are acquired..."
```

### 3. Testing Skills → Training Examples (50 examples)

| Skill | When to Use | Training Focus |
|-------|-------------|----------------|
| `testing/test-driven-development/SKILL.md` | Implementing any feature or bugfix | **RED-GREEN-REFACTOR**: Write test → See fail → Implement → See pass → Refactor |
| `testing/testing-anti-patterns/SKILL.md` | Writing or changing tests | Avoiding common mistakes (over-mocking, test-only methods, etc.) |
| `testing/condition-based-waiting/SKILL.md` | Tests have race conditions, timing dependencies | Using condition-based waits instead of sleep() |

**Training Pattern Example:**
```
User: "Add retry logic to API client"
Assistant: "I'll follow TDD.

**RED Phase: Write failing test**
[writes test that expects retries]

**Run test - should FAIL**
[runs test, shows failure]

**GREEN Phase: Implement minimal code**
[implements retry logic]

**Run test - should PASS**
[runs test, shows success]

**REFACTOR Phase: Improve code quality**
[refactors if needed]"
```

### 4. Problem-Solving Skills → Training Examples (100 examples)

| Skill | When to Use | Training Focus |
|-------|-------------|----------------|
| `problem-solving/when-stuck/SKILL.md` | Stuck and unsure which technique to apply | Try alternatives, break down differently, search patterns, consult docs |
| `problem-solving/scale-game/SKILL.md` | Uncertain about scalability, edge cases unclear | Test at 10x, 100x, 1000x, 10000x to find breaking points |
| `problem-solving/inversion-exercise/SKILL.md` | Stuck on unquestioned assumptions | Challenge "the only way" - what if we DON'T do X? |
| `problem-solving/collision-zone-thinking/SKILL.md` | Conventional approaches feel inadequate | Force unrelated concepts together for breakthrough innovation |
| `problem-solving/simplification-cascades/SKILL.md` | Implementing same concept multiple ways, complexity spiraling | Identify core pattern, eliminate special cases |
| `problem-solving/meta-pattern-recognition/SKILL.md` | Noticing same pattern across 3+ domains | Draw parallels to known solutions, generalize fixes |

**Training Pattern Example:**
```
User: "Sorting algorithm seems slow but I can't figure out why"
Assistant: "Let me use the scale game to identify the issue.

**Scale Game Testing:**
[runs benchmarks at different scales]

Results:
- 10 items: 0.5 μs ✓
- 100 items: 45 μs ✓
- 1,000 items: 4.2 ms ⚠️
- 10,000 items: 420 ms ✗ PROBLEM!

**Diagnosis:** O(n²) complexity - at 10k items, 100M comparisons!

The nested loops are the issue. Need O(n log n) algorithm instead..."
```

### 5. Architecture Skills → Training Examples (50 examples)

| Skill | When to Use | Training Focus |
|-------|-------------|----------------|
| `architecture/preserving-productive-tensions/SKILL.md` | Oscillating between equally valid approaches | Balancing competing priorities, mapping trade-offs clearly |
| `research/tracing-knowledge-lineages/SKILL.md` | Questioning "why do we use X" | Understanding historical context, avoiding revivals of failed ideas |

**Training Pattern Example:**
```
User: "Should we use microservices or monolith?"
Assistant: "This is a productive tension - both are valid. Let me map the trade-offs:

**Monolith advantages:**
- Simpler deployment
- Easier local dev
- Better data consistency
- Lower latency

**Microservices advantages:**
- Independent scaling
- Technology flexibility
- Isolated failures
- Independent deployment

**Rather than choose, let's ask:**
1. Team size? (Monolith works to ~20 devs)
2. Different load patterns per feature?
3. Need independent deployment?

Or consider: **Modular monolith** - simple now, extract services later if needed."
```

## Key Behaviors to Train

### Explicit Skill References

The model should **reference skills by name**:
- ✅ "Let me use the brainstorming skill to refine this idea..."
- ✅ "I'll follow systematic debugging (RED-YELLOW-GREEN)..."
- ✅ "Using the scale game to test edge cases..."
- ❌ (Just doing the behavior without naming it)

### Workflow Adherence

The model should **follow exact workflows**:
- TDD: RED → GREEN → REFACTOR (not just "write test and code")
- Systematic Debugging: RED → YELLOW → GREEN (not just "fix it")
- Brainstorming: Questions before implementation (not just "I'll add caching")
- Scale Game: 10x, 100x, 1000x, 10000x (not just "test with large input")

### When to Use (Triggers)

The model should **recognize when each skill applies**:
- Vague feature request → Brainstorming
- Bug/error → Systematic debugging
- New feature → TDD
- Performance issue → Scale game
- Stuck → When stuck skill
- Assumptions feel forced → Inversion exercise

### Why Behind Each Step

The model should **explain reasoning**:
- "Forming hypothesis because evidence shows X, Y, Z..."
- "Testing at 10x scale to find breaking point..."
- "Writing test first to ensure we understand the requirement..."
- "Using git worktree to isolate this risky change..."

## Integration with Other Categories

### Superpowers Skills Overlap

Some Superpowers skills overlap with general categories:

| General Category | Superpowers Skill | Relationship |
|------------------|-------------------|--------------|
| Agentic Skills (TDD) | `testing/test-driven-development` | **Superpowers version is MORE DETAILED** - use for training |
| Agentic Skills (Debugging) | `debugging/systematic-debugging` | **Superpowers version is MORE DETAILED** - use for training |
| Problem-Solving (When stuck) | `problem-solving/when-stuck` | **Superpowers version is MORE DETAILED** - use for training |
| Problem-Solving (Scale/Edge) | `problem-solving/scale-game` | **Superpowers is SPECIFIC WORKFLOW** - use for training |
| Collaboration (Planning) | `collaboration/writing-plans` | **Superpowers is SPECIFIC FORMAT** - use for training |

**Strategy:** Use Superpowers skills for detailed workflows, general categories for variations/adaptations.

## Dataset Generation Strategy

### Phase 1: Read All Skills

```bash
# Agent should read these files first:
/Users/zachswift/.config/superpowers/skills/skills/collaboration/brainstorming/SKILL.md
/Users/zachswift/.config/superpowers/skills/skills/collaboration/writing-plans/SKILL.md
/Users/zachswift/.config/superpowers/skills/skills/collaboration/executing-plans/SKILL.md
# ... etc for all skills
```

### Phase 2: Extract Key Workflows

For each skill, extract:
1. **When to use** (trigger condition)
2. **Workflow steps** (exact sequence)
3. **Example scenarios** (from skill documentation)
4. **Key principles** (what makes it effective)

### Phase 3: Generate Variations

For each skill:
- Different programming languages (Rust, TypeScript, Python, Go)
- Different domains (web API, CLI tool, database, UI)
- Different complexity levels (simple, medium, complex)
- Success and failure cases
- Edge cases and gotchas

### Phase 4: Quality Check

Every example should:
- ✅ Reference the skill by name
- ✅ Follow the exact workflow
- ✅ Show realistic scenario
- ✅ Include tool calls where appropriate
- ✅ Explain reasoning at each step
- ✅ Natural conversation flow

## Expected Impact on Model Behavior

After training on Superpowers skills, the model should:

1. **Proactively use workflows** - "Before implementing, let me brainstorm..."
2. **Follow disciplines rigorously** - RED-GREEN-REFACTOR, not shortcuts
3. **Explain with skill vocabulary** - "Using systematic debugging...", "Running the scale game..."
4. **Ask before acting** - Brainstorm before coding, questions before assumptions
5. **Verify before claiming done** - Never say "fixed" without running tests
6. **Handle complexity systematically** - Not ad-hoc, but following proven patterns

## Total Training Examples: 450

- Collaboration: 150 (brainstorming, plans, subagents, code review)
- Debugging: 100 (systematic debugging, root cause tracing, verification)
- Testing: 50 (TDD, anti-patterns, condition-based waiting)
- Problem-Solving: 100 (when stuck, scale game, inversion, simplification)
- Architecture: 50 (productive tensions, knowledge lineages)

## Agent Responsible

**Agent:** `.claude/agents/dataset-generator-superpowers.md`

**Usage:**
```
Use the dataset-generator-superpowers agent to create 150 brainstorming and planning examples from Superpowers collaboration skills
```
