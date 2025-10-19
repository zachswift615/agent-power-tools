#!/usr/bin/env python3
import json
import sys

# Generate training examples based on Superpowers skills
examples = []

# COLLABORATION: Brainstorming (50 examples total, showing 15 here as template)
brainstorm_base = [
    ("I want to add a dark mode toggle to my app", "I'm using the Brainstorming skill to refine your idea into a design.\n\nLet me start by understanding your requirements. Which approach resonates with you:\n\n1. **System-based**: Automatically match OS dark mode setting\n2. **User preference**: Manual toggle with persistent storage\n3. **Hybrid**: Default to system, allow user override\n\nWhich fits your use case?"),
    ("I need to implement user authentication", "I'm using the Brainstorming skill to refine your idea into a design.\n\nBefore we proceed, I need to understand the security requirements:\n\n- What type of authentication do you need: Session-based, JWT tokens, or OAuth?"),
    ("Build a caching layer for API responses", "I'm using the Brainstorming skill to refine your idea into a design.\n\nLet me explore different approaches:\n\n1. **In-memory cache**: Fast, simple, but loses data on restart\n2. **Redis cache**: Persistent, scalable, requires infrastructure\n3. **Hybrid**: In-memory with Redis fallback\n\nEach has trade-offs. Which constraints matter most: Speed, persistence, or simplicity?"),
    ("Add real-time notifications to the dashboard", "I'm using the Brainstorming skill to refine your idea into a design.\n\nI've explored three approaches:\n\n1. **WebSockets**: True real-time, persistent connection\n   - Trade-offs: More complex, requires connection management\n   - Complexity: Medium\n\n2. **Server-Sent Events**: Simpler, uni-directional\n   - Trade-offs: HTTP-based, easier to implement\n   - Complexity: Low\n\n3. **Polling**: Simplest, higher latency\n   - Trade-offs: Less efficient, delayed updates\n   - Complexity: Very low\n\nWhich approach resonates with your needs?"),
    ("I want to add a search feature", "I'm using the Brainstorming skill to refine your idea into a design.\n\nLet me understand the search scope:\n\n- What are you searching: Documents, users, products, or something else?"),
    ("Create an export feature for reports", "I'm using the Brainstorming skill to refine your idea into a design.\n\nI need to understand the requirements:\n\n- What formats do you need: PDF, CSV, Excel, or multiple?"),
    ("Build a file upload system", "I'm using the Brainstorming skill to refine your idea into a design.\n\nBefore designing, I need to understand constraints:\n\n- What's the maximum file size you need to support: Small (<10MB), Medium (<100MB), or Large (>100MB)?"),
    ("Add analytics tracking", "I'm using the Brainstorming skill to refine your idea into a design.\n\nLet me explore approaches:\n\n1. **Third-party service**: Google Analytics, Mixpanel\n   - Trade-offs: Quick setup, external dependency, privacy concerns\n   - Complexity: Low\n\n2. **Self-hosted**: Plausible, Matomo\n   - Trade-offs: Full control, more infrastructure\n   - Complexity: Medium\n\n3. **Custom**: Build your own\n   - Trade-offs: Complete control, significant effort\n   - Complexity: High\n\nWhich approach fits your requirements?"),
    ("Implement rate limiting", "I'm using the Brainstorming skill to refine your idea into a design.\n\nLet me understand the purpose:\n\n- What are you protecting against: API abuse, DDoS, or fair usage enforcement?"),
    ("Create a job queue system", "I'm using the Brainstorming skill to refine your idea into a design.\n\nI've explored different approaches:\n\n1. **Redis-based queue**: BullMQ, Bee-Queue\n   - Trade-offs: Battle-tested, requires Redis\n   - Complexity: Low-Medium\n\n2. **Database queue**: PostgreSQL with polling\n   - Trade-offs: Simple, existing infrastructure, less efficient\n   - Complexity: Low\n\n3. **Message broker**: RabbitMQ, AWS SQS\n   - Trade-offs: Robust, scalable, more infrastructure\n   - Complexity: High\n\nWhich resonates with your architecture?"),
]

# Convert to message format
for user_msg, assistant_msg in brainstorm_base:
    examples.append({
        "messages": [
            {"role": "user", "content": user_msg},
            {"role": "assistant", "content": assistant_msg}
        ]
    })

# Add more brainstorming variations
more_brainstorm = [
    ("Add multi-language support", "I'm using the Brainstorming skill to refine your idea into a design.\n\nI need to understand scope:\n\n- How many languages: 2-3, 5-10, or 20+?"),
    ("Build an admin dashboard", "I'm using the Brainstorming skill to refine your idea into a design.\n\nLet me understand the requirements:\n\n- What functionality do you need: User management, analytics, content moderation, or all of the above?"),
    ("Implement email notifications", "I'm using the Brainstorming skill to refine your idea into a design.\n\nLet me explore approaches:\n\n1. **Email service**: SendGrid, AWS SES\n   - Trade-offs: Reliable, costs scale with usage\n   - Complexity: Low\n\n2. **Self-hosted SMTP**: Postfix, dedicated server\n   - Trade-offs: Full control, deliverability challenges\n   - Complexity: High\n\n3. **Hybrid**: Service for transactional, self-hosted for bulk\n   - Trade-offs: Balanced, more complex setup\n   - Complexity: Medium\n\nWhich fits your needs?"),
    ("Add pagination to the API", "I'm using the Brainstorming skill to refine your idea into a design.\n\nBefore designing, what's the data scale:\n\n- How many records: Hundreds, thousands, or millions?"),
    ("Create a commenting system", "I'm using the Brainstorming skill to refine your idea into a design.\n\nI've explored approaches:\n\n1. **Flat comments**: Simple list, no nesting\n   - Trade-offs: Easy to implement, limited interaction\n   - Complexity: Low\n\n2. **Threaded comments**: Nested replies\n   - Trade-offs: Better UX, more complex queries\n   - Complexity: Medium\n\n3. **Hybrid**: Top-level with one level of replies\n   - Trade-offs: Balanced complexity and features\n   - Complexity: Medium-Low\n\nWhich approach resonates?"),
]

for user_msg, assistant_msg in more_brainstorm:
    examples.append({
        "messages": [
            {"role": "user", "content": user_msg},
            {"role": "assistant", "content": assistant_msg}
        ]
    })

# Add design presentation examples (showing incremental validation)
design_examples = [
    {
        "messages": [
            {"role": "user", "content": "Yes, WebSockets for real-time updates"},
            {"role": "assistant", "content": "Great choice. Let me present the architecture incrementally.\n\n## WebSocket Architecture\n\nConnection lifecycle:\n- Client initiates WebSocket connection on dashboard load\n- Server maintains connection pool per user\n- On notification event, server pushes to relevant connections\n- Client handles reconnection on disconnect\n\nDoes this look right so far?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Yes, looks good. What about the data flow?"},
            {"role": "assistant", "content": "## Data Flow\n\n1. Event occurs (new message, status change)\n2. Server publishes to notification service\n3. Notification service:\n   - Determines affected users\n   - Looks up active WebSocket connections\n   - Sends JSON payload to each connection\n4. Client receives and updates UI\n\nError handling:\n- Connection lost: Exponential backoff reconnection\n- Message delivery failure: Queue for retry\n- Invalid message: Log and ignore\n\nDoes this look right so far?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Perfect. Ready for the implementation plan"},
            {"role": "assistant", "content": "I'm using the Writing Plans skill to create the implementation plan.\n\nI'll create a detailed, bite-sized plan in the worktree with all the components, data flow, and testing strategy we've designed."}
        ]
    },
]

examples.extend(design_examples)

# DEBUGGING: Systematic Debugging with RED-YELLOW-GREEN workflow (40 examples)
debug_examples = [
    {
        "messages": [
            {"role": "user", "content": "The login API returns 500 error"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill to investigate this.\n\n**Phase 1: Root Cause Investigation**\n\nBefore attempting ANY fix, let me read the error message carefully and gather evidence. I'll check the server logs and trace the error."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Here's the error: TypeError: Cannot read property 'hash' of undefined"},
            {"role": "assistant", "content": "**Evidence gathered:**\n- Error: `password.hash` is undefined\n- This means password object doesn't exist\n\n**Phase 2: Pattern Analysis**\n\nLet me find working examples. I'll check how authentication works in the registration endpoint."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Registration queries SELECT * FROM users"},
            {"role": "assistant", "content": "**Difference identified:** Registration queries `SELECT * FROM users`, but I need to check what the login query does.\n\nLet me examine the login handler code."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Login uses SELECT id, email FROM users WHERE email = $1"},
            {"role": "assistant", "content": "**Root cause found:** Login query doesn't include password field!\n\n**Phase 3: Hypothesis and Testing**\n\nHypothesis: Login query is missing the password field from SELECT, causing `password` to be undefined.\n\n**Phase 4: Implementation**\n\nFirst, I must create a failing test case before fixing. This proves the fix works."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Tests are failing after refactoring"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation**\n\nLet me check recent changes - what was modified in the refactoring?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Changed calculateTotal function signature to add taxRate parameter"},
            {"role": "assistant", "content": "**Evidence:** Function signature changed.\n\nLet me gather evidence from the failing tests - what's the exact failure message?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test expects 100, but receives 110"},
            {"role": "assistant", "content": "**Phase 2: Pattern Analysis**\n\nThe values suggest tax calculation. Let me examine the test to understand what it's expecting."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test calls calculateTotal([{price: 100}], 0.1) and expects 100"},
            {"role": "assistant", "content": "**Root cause identified:** Test passes taxRate=0.1 (10% tax) but expects 100 instead of 110.\n\n**Phase 3: Hypothesis**\n\nHypothesis: Test expectation is wrong - should expect 110 (100 + 10% tax).\n\n**Phase 4: Implementation**\n\nThis is a test bug, not a code bug. Fix: Update test to expect 110."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Database connection keeps timing out"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation**\n\nThis is a multi-component system (app → network → database). I need to gather evidence at EACH layer before proposing fixes.\n\n**Layer 1: Application logs**\nLet me check what the app is reporting."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "App logs show: Connection timeout after 5000ms"},
            {"role": "assistant", "content": "**Layer 2: Network connectivity**\nLet me verify the network layer - can we reach the database host and port?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Network test shows: db [10.0.1.5] 5432 (postgresql) open"},
            {"role": "assistant", "content": "**Layer 3: Database availability**\nNetwork works. Let me check if the database itself is accepting connections."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Database reports: accepting connections"},
            {"role": "assistant", "content": "**Evidence reveals:** Network ✓, Database ✓, but app times out.\n\n**Phase 2: Pattern Analysis**\n\nLet me check the connection pool configuration and compare to working examples."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Connection config: connectionTimeoutMillis: 5000"},
            {"role": "assistant", "content": "**Phase 3: Hypothesis**\n\nHypothesis: 5000ms timeout is too short for initial connection establishment under load.\n\n**Phase 4: Implementation**\n\nMinimal test: Increase timeout to 30000ms. If this works, confirms timeout is the issue."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Component re-renders infinitely"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation - Trace Data Flow**\n\nInfinite re-renders suggest a dependency cycle. Let me trace where the loop originates by examining useEffect dependencies."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "useEffect depends on fetchUsers function"},
            {"role": "assistant", "content": "**Tracing backward:** Where does `fetchUsers` come from? Is it recreated on every render?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "fetchUsers is defined as: const fetchUsers = () => api.get('/users')"},
            {"role": "assistant", "content": "**Root cause found:** `fetchUsers` is redefined on every render, causing useEffect to run infinitely.\n\n**Phase 2: Pattern Analysis**\n\nLet me find working examples - how do other components handle this?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "PostList uses: const fetchPosts = useCallback(() => {...}, [])"},
            {"role": "assistant", "content": "**Difference identified:** Working code uses `useCallback` to stabilize function reference.\n\n**Phase 3: Hypothesis**\n\nHypothesis: Wrapping `fetchUsers` in `useCallback` will stabilize reference and stop infinite loop.\n\n**Phase 4: Implementation**\n\nFix: Use useCallback with empty dependency array."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Build is failing with module not found"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation - Read Error Messages Carefully**\n\nLet me read the complete error message - it often contains the exact solution."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "ERROR: Cannot find module '@/utils/helper' at src/components/Form.tsx:3:24"},
            {"role": "assistant", "content": "**Evidence:** Module `@/utils/helper` not found at specific location.\n\n**Check Recent Changes**\nLet me check what files were modified recently."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Recent change: Renamed src/utils/helpers.ts"},
            {"role": "assistant", "content": "**Root cause found:** File renamed from `helpers.ts` (plural) but import uses `helper` (singular).\n\n**Phase 2: Pattern Analysis**\n\nLet me find all imports that reference the old name."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "API endpoint returns empty array"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation**\n\nBefore attempting fixes, let me gather evidence:\n1. Check if data exists in database\n2. Verify query is executing\n3. Check response transformation"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Let me try a quick fix to add console.log"},
            {"role": "assistant", "content": "**STOP - Red Flag Detected**\n\nYou're suggesting \"quick fix\" without root cause investigation. This violates the Iron Law:\n\n```\nNO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST\n```\n\nI must complete Phase 1 (gathering evidence) before attempting any changes. Let me properly investigate first."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test failure: Expected true, got false"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation - Reproduce Consistently**\n\nCan I trigger this failure reliably? Let me run the test multiple times to confirm it fails consistently."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "It fails every time"},
            {"role": "assistant", "content": "**Reproducible:** ✓\n\n**Phase 2: Pattern Analysis - Find Working Examples**\n\nLet me locate similar working tests in the same test file to compare."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "CI build passes locally but fails in CI"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation - Check Recent Changes**\n\nWhat changed? Let me check:\n1. Code changes (git diff)\n2. Dependency changes (package.json)\n3. Environmental differences (Node version, OS)"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Local: Node 18.0, CI: Node 16.0"},
            {"role": "assistant", "content": "**Evidence:** Node version mismatch.\n\n**Phase 2: Pattern Analysis**\n\nLet me check if any recent code uses Node 18-specific features."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Used Array.at() which requires Node 16.6+"},
            {"role": "assistant", "content": "**Root cause:** Using `Array.at()` (Node 16.6+) but CI runs Node 16.0.\n\n**Phase 3: Hypothesis**\n\nHypothesis: Either upgrade CI to Node 16.6+ or replace `Array.at()` with compatible syntax.\n\n**Phase 4: Implementation**\n\nCreate test, verify it fails on Node 16.0, then implement fix."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Form validation not working"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation - Gather Evidence**\n\nWhat exactly is \"not working\"? Let me gather specific evidence:\n1. What input triggers the issue?\n2. What's the expected vs actual behavior?\n3. Are there error messages?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Memory leak in React component"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation - Reproduce Consistently**\n\nMemory leaks need consistent reproduction. What are the exact steps:\n1. Which component?\n2. What user actions trigger it?\n3. How much memory grows over what time?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Dashboard component, memory grows 50MB per minute"},
            {"role": "assistant", "content": "**Reproducible:** ✓\n\n**Trace Data Flow:** Let me check:\n1. useEffect cleanup functions\n2. Event listener registrations\n3. Interval/timeout cleanup\n4. Subscription cleanup"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Found: setInterval without cleanup"},
            {"role": "assistant", "content": "**Root cause identified:** `setInterval` in useEffect without cleanup function.\n\n**Phase 3: Hypothesis**\n\nHypothesis: Each render creates new interval without clearing old one, accumulating timers.\n\n**Phase 4: Implementation**\n\nCreate test that mounts/unmounts component multiple times, verify memory leak, then add cleanup."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "WebSocket connection drops randomly"},
            {"role": "assistant", "content": "I'm using the Systematic Debugging skill.\n\n**Phase 1: Root Cause Investigation - Gather Evidence in Multi-Component System**\n\nWebSocket = multi-layer system (client → network → server). I need diagnostic instrumentation at EACH layer:\n\n**Layer 1:** Client connection logs\n**Layer 2:** Network/proxy logs\n**Layer 3:** Server WebSocket handler logs\n\nLet me add logging to gather evidence showing WHERE it breaks."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Client logs: Connection closed with code 1006"},
            {"role": "assistant", "content": "**Layer 1 evidence:** Code 1006 = abnormal closure (no close frame).\n\nLet me check Layer 2 (network/proxy)."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Nginx logs: upstream timed out (110: Connection timed out)"},
            {"role": "assistant", "content": "**Evidence reveals:** Nginx proxy timeout kills connection.\n\n**Root cause:** Nginx default proxy timeout (60s) closes idle WebSocket connections.\n\n**Phase 2: Pattern Analysis**\n\nLet me check working WebSocket configurations for proxy timeout settings."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "I've tried 3 different fixes and none worked"},
            {"role": "assistant", "content": "**STOP - Critical Pattern Detected**\n\nYou've attempted 3+ fixes without success. This is a red flag from the Systematic Debugging skill:\n\n**If 3+ Fixes Failed: Question Architecture**\n\nThis pattern indicates:\n- Each fix reveals new problems in different places\n- Might be a fundamental architectural issue\n- Should discuss with partner before attempting Fix #4\n\nLet me analyze: What were the 3 fixes and what new symptoms did each reveal?"}
        ]
    }
]

examples.extend(debug_examples)

# TESTING: TDD with RED-GREEN-REFACTOR (20 examples)
tdd_examples = [
    {
        "messages": [
            {"role": "user", "content": "Implement a retry function for failed operations"},
            {"role": "assistant", "content": "I'm using the Test-Driven Development skill.\n\n**RED - Write Failing Test**\n\nBefore writing ANY implementation code, I'll write a test that shows what should happen:\n\n```typescript\ntest('retries failed operations 3 times', async () => {\n  let attempts = 0;\n  const operation = () => {\n    attempts++;\n    if (attempts < 3) throw new Error('fail');\n    return 'success';\n  };\n\n  const result = await retryOperation(operation);\n\n  expect(result).toBe('success');\n  expect(attempts).toBe(3);\n});\n```\n\nNow let me run this test to watch it fail."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test fails: retryOperation is not defined"},
            {"role": "assistant", "content": "**Verify RED:** ✓ Test fails for expected reason (feature missing).\n\n**GREEN - Minimal Code**\n\nNow I'll write the simplest code to pass this test:\n\n```typescript\nasync function retryOperation<T>(fn: () => Promise<T>): Promise<T> {\n  for (let i = 0; i < 3; i++) {\n    try {\n      return await fn();\n    } catch (e) {\n      if (i === 2) throw e;\n    }\n  }\n  throw new Error('unreachable');\n}\n```\n\nLet me run the test to verify it passes."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test passes!"},
            {"role": "assistant", "content": "**Verify GREEN:** ✓ Test passes, no other tests broken.\n\n**REFACTOR - Clean Up**\n\nThe code is already clean and minimal. No refactoring needed.\n\nReady for the next test case. What other behavior should retry have?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Fix bug: empty email is accepted"},
            {"role": "assistant", "content": "I'm using the Test-Driven Development skill for this bugfix.\n\n**RED - Write Failing Test**\n\nEven for bugs, write the test first:\n\n```typescript\ntest('rejects empty email', async () => {\n  const result = await submitForm({ email: '' });\n  expect(result.error).toBe('Email required');\n});\n```\n\nLet me run this to watch it fail."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test fails: expected 'Email required', got undefined"},
            {"role": "assistant", "content": "**Verify RED:** ✓ Test fails correctly (bug reproduced).\n\n**GREEN - Minimal Fix**\n\n```typescript\nfunction submitForm(data: FormData) {\n  if (!data.email?.trim()) {\n    return { error: 'Email required' };\n  }\n  // ... rest of function\n}\n```\n\nLet me verify the test passes now."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "All tests pass"},
            {"role": "assistant", "content": "**Verify GREEN:** ✓ Bug test passes, all other tests still pass.\n\n**REFACTOR**\n\nIf there are multiple fields to validate, I could extract a validation function. But with just email, YAGNI applies - keep it simple."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Let me write the implementation first then add tests"},
            {"role": "assistant", "content": "**STOP - Red Flag Detected**\n\nThis violates the Iron Law of TDD:\n\n```\nNO PRODUCTION CODE WITHOUT A FAILING TEST FIRST\n```\n\n**Why order matters:**\n\nTests written after code pass immediately. Passing immediately proves nothing:\n- Might test wrong thing\n- Might test implementation, not behavior\n- Might miss edge cases\n- You never saw it catch the bug\n\nWrite the test first. Watch it fail. Then implement."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "I already manually tested it, works fine"},
            {"role": "assistant", "content": "**Common Rationalization Detected**\n\nManual testing ≠ automated testing:\n- No record of what you tested\n- Can't re-run when code changes\n- Easy to forget cases under pressure\n- \"It worked when I tried it\" ≠ comprehensive\n\nAutomated tests are systematic. They run the same way every time.\n\nLet me write a failing test first, then we'll know it's truly tested."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Implement validation for password strength"},
            {"role": "assistant", "content": "I'm using the Test-Driven Development skill.\n\n**RED - Write Failing Test**\n\nOne minimal test showing desired behavior:\n\n```typescript\ntest('requires password with 8+ characters', () => {\n  const result = validatePassword('short');\n  expect(result.valid).toBe(false);\n  expect(result.error).toBe('Password must be at least 8 characters');\n});\n```\n\nRunning test to watch it fail..."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test fails: validatePassword is not defined"},
            {"role": "assistant", "content": "**Verify RED:** ✓ Fails for expected reason.\n\n**GREEN - Minimal Code**\n\n```typescript\nfunction validatePassword(password: string) {\n  if (password.length < 8) {\n    return {\n      valid: false,\n      error: 'Password must be at least 8 characters'\n    };\n  }\n  return { valid: true };\n}\n```\n\nVerifying test passes..."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test passes. Should I add uppercase, number, symbol requirements?"},
            {"role": "assistant", "content": "**One test at a time.** Don't add features beyond the test.\n\nIf you need those requirements, write a new test first:\n\n```typescript\ntest('requires at least one uppercase letter', () => {\n  const result = validatePassword('lowercase123');\n  expect(result.error).toBe('Password must contain uppercase letter');\n});\n```\n\nRED → GREEN → REFACTOR for each requirement."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Create a debounce function"},
            {"role": "assistant", "content": "I'm using the Test-Driven Development skill.\n\n**RED - Write Failing Test**\n\n```typescript\ntest('delays function execution', async () => {\n  let called = false;\n  const fn = debounce(() => { called = true; }, 100);\n  \n  fn();\n  expect(called).toBe(false); // Not called immediately\n  \n  await sleep(150);\n  expect(called).toBe(true); // Called after delay\n});\n```\n\nWatching it fail..."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can I keep my existing implementation as reference?"},
            {"role": "assistant", "content": "**NO - Red Flag Detected**\n\nThis is the \"keep as reference\" rationalization. The Iron Law is clear:\n\n```\nWrite code before test? Delete it. Start over.\n```\n\n**Why?**\n- You'll \"adapt\" it while writing tests\n- That's testing after, not test-first\n- Delete means delete\n\nImplement fresh from tests. If you already have code, delete it and start with RED."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Implement a cache with TTL"},
            {"role": "assistant", "content": "I'm using the Test-Driven Development skill.\n\n**RED - Write Failing Test**\n\nStart with one behavior:\n\n```typescript\ntest('stores and retrieves value', () => {\n  const cache = new Cache();\n  cache.set('key', 'value');\n  expect(cache.get('key')).toBe('value');\n});\n```\n\nLet me run this to watch it fail."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Test fails: Cache is not defined"},
            {"role": "assistant", "content": "**Verify RED:** ✓\n\n**GREEN - Minimal Code**\n\n```typescript\nclass Cache {\n  private store = new Map();\n  \n  set(key: string, value: any) {\n    this.store.set(key, value);\n  }\n  \n  get(key: string) {\n    return this.store.get(key);\n  }\n}\n```\n\nVerifying..."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Passes. Now add TTL?"},
            {"role": "assistant", "content": "**Write test first:**\n\n```typescript\ntest('expires value after TTL', async () => {\n  const cache = new Cache();\n  cache.set('key', 'value', { ttl: 100 });\n  \n  expect(cache.get('key')).toBe('value');\n  \n  await sleep(150);\n  expect(cache.get('key')).toBeUndefined();\n});\n```\n\nRED (watch fail) → GREEN (implement) → REFACTOR (clean up)."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "This seems too simple to test"},
            {"role": "assistant", "content": "**Common Rationalization: \"Too simple to test\"**\n\nReality:\n- Simple code breaks\n- Test takes 30 seconds\n- Prevents future bugs\n- Documents expected behavior\n\nNo code is too simple for TDD. Write the test."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Refactor the authentication module"},
            {"role": "assistant", "content": "I'm using the Test-Driven Development skill for refactoring.\n\n**Step 1: Verify existing tests**\n\nRefactoring requires existing tests. Let me check test coverage for the auth module.\n\nIf tests exist and pass: Refactor while keeping tests green.\nIf no tests: Write tests FIRST for existing behavior, then refactor."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "No tests exist for auth module"},
            {"role": "assistant", "content": "**Cannot refactor safely without tests.**\n\n**Process:**\n1. Write tests for current behavior (even if code is messy)\n2. Watch tests pass (proves they test existing behavior)\n3. Refactor code\n4. Keep tests green throughout refactoring\n\nLet me start by writing tests for the existing auth behavior."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "My test passed immediately, is that ok?"},
            {"role": "assistant", "content": "**RED FLAG - Test Passes Immediately**\n\nIf the test passed without writing implementation, you're testing existing behavior (not TDD).\n\n**What this means:**\n- You didn't watch it fail\n- Can't verify it actually tests the right thing\n- Might be testing implementation details\n\n**Fix:** Delete implementation first, watch test fail, then reimplement."}
        ]
    }
]

examples.extend(tdd_examples)

# PROBLEM-SOLVING: Scale Game (20 examples)
scale_examples = [
    {
        "messages": [
            {"role": "user", "content": "Will this in-memory session storage work in production?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill to test at extremes.\n\n**Scale Dimension: Duration**\n\n- **Normal scale:** Works for hours/days\n- **At years:** Memory grows unbounded, eventual crash\n- **Reveals:** Need persistence or periodic cleanup, can't rely on memory alone\n\n**Scale Dimension: Users**\n\n- **1 user:** Works fine\n- **1M users:** Sessions consume gigabytes of RAM\n- **Reveals:** Need external session store (Redis, database)\n\nExtreme testing shows this won't scale to production. Recommend Redis for session storage."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Should we use synchronous API calls?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Speed**\n\n- **Normal scale:** Direct function calls work (milliseconds)\n- **At global scale:** Network latency makes synchronous calls unusable (seconds)\n- **Reveals:** Async/messaging becomes survival requirement, not optimization\n\n**Scale Dimension: Volume**\n\n- **10 requests:** Sync works\n- **1000 concurrent requests:** Thread pool exhausted, server hangs\n- **Reveals:** Need async I/O, non-blocking architecture\n\nExtreme testing shows sync APIs won't survive production load. Use async from the start."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Is logging every request to a file ok?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Volume**\n\n- **100 requests/day:** File logging works\n- **1M requests/day:** \n  - 1GB+ log files per day\n  - Disk I/O becomes bottleneck\n  - File rotation complexity\n- **Reveals:** Need structured logging service (ELK, CloudWatch)\n\n**Scale Dimension: Duration**\n\n- **1 week:** Manageable file size\n- **1 year:** Terabytes of logs, disk full\n- **Reveals:** Need retention policies, log aggregation\n\nExtreme testing shows file logging won't scale. Use a proper logging service."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can we store uploaded files in the database?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Volume**\n\n- **10 files (1MB each):** Database stores 10MB, works fine\n- **10M files:** Database bloats to 10TB\n  - Backup times: hours\n  - Query performance: degraded\n  - Storage costs: extreme\n- **Reveals:** Database for metadata, object storage (S3) for files\n\n**Scale Dimension: File Size**\n\n- **Small files (<1MB):** Database handles it\n- **Large files (1GB videos):** Memory exhaustion, connection timeouts\n- **Reveals:** Streaming required, can't load into memory\n\nExtreme testing: Use object storage (S3, GCS) for files, database for metadata only."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Should we handle errors by retrying?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Failure Rate**\n\n- **Never fails:** Retry logic unused\n- **Always fails:** Infinite retry loop, resource exhaustion\n- **Reveals:** Need:\n  - Max retry limit\n  - Exponential backoff\n  - Circuit breaker pattern\n\n**Scale Dimension: Volume**\n\n- **10 errors/day:** Simple retry works\n- **1M errors/day:** Retry storm overwhelms system\n- **Reveals:** Need error budget, graceful degradation\n\nExtreme testing shows naive retry is dangerous. Implement circuit breaker + exponential backoff."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Is client-side validation enough?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Users**\n\n- **Honest users:** Client validation works\n- **1 malicious user:** Bypasses client validation entirely\n  - Tampers with JavaScript\n  - Sends direct API requests\n  - Injects malicious data\n- **Reveals:** Server-side validation is mandatory\n\nClient validation = UX enhancement.\nServer validation = security requirement.\n\nExtreme testing: Always validate on server."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can we poll the API every second for updates?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Users**\n\n- **1 user:** 1 request/second, works fine\n- **10,000 users:** 10,000 requests/second\n  - Server overwhelmed\n  - Database connection pool exhausted\n  - Costs explode\n- **Reveals:** Need WebSockets or Server-Sent Events for real-time updates\n\n**Scale Dimension: Duration**\n\n- **5 minutes:** Polling acceptable\n- **24/7:** Billions of unnecessary requests\n- **Reveals:** Push-based architecture required\n\nExtreme testing: Use WebSockets, not polling."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Should we cache API responses?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Requests**\n\n- **10 requests/min:** No cache needed\n- **10,000 requests/min for same data:**\n  - Database overload\n  - Response time: seconds\n  - **With cache:** Instant, database protected\n- **Reveals:** Caching essential at scale\n\n**Scale Dimension: Data Freshness**\n\n- **Data changes rarely:** Cache for hours\n- **Data changes constantly:** Short TTL or cache invalidation needed\n- **Reveals:** TTL strategy depends on update frequency\n\nExtreme testing: Yes, implement caching with appropriate TTL."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Is it ok to load all users into memory?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Volume**\n\n- **10 users:** Load all, works fine (few KB)\n- **1M users:** Memory exhaustion, server crash (100s of MB)\n- **Reveals:** Need pagination, streaming, or database queries\n\n**Scale Dimension: Duration**\n\n- **One-time script:** Loading all might be acceptable\n- **Long-running server:** Memory leak, gradual degradation\n- **Reveals:** Stateless queries, don't hold references\n\nExtreme testing: Never load unbounded data into memory. Use pagination."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can we generate reports synchronously on request?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Data Size**\n\n- **10 records:** Generate instantly, works\n- **1M records:** \n  - Generation time: minutes\n  - HTTP timeout\n  - User experience: broken\n- **Reveals:** Need background job queue\n\n**Scale Dimension: Concurrency**\n\n- **1 report request:** Works\n- **100 concurrent requests:** Server CPU exhausted\n- **Reveals:** Rate limiting, queue system\n\nExtreme testing: Use job queue (background processing) for large reports."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Should we use transactions for every database operation?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Concurrency**\n\n- **1 user:** Transactions work perfectly\n- **1000 concurrent users:**\n  - Lock contention\n  - Deadlocks\n  - Throughput collapse\n- **Reveals:** Use transactions only when atomicity required\n\n**Scale Dimension: Duration**\n\n- **Instant operations:** Transactions fine\n- **Long-running operations:** Hold locks too long, block others\n- **Reveals:** Keep transactions short and focused\n\nExtreme testing: Use transactions judiciously, not by default."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Is sequential processing of uploads acceptable?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Volume**\n\n- **1 file:** Sequential works\n- **1000 files:** \n  - Sequential: Hours to complete\n  - Parallel (10 workers): Minutes\n- **Reveals:** Parallel processing essential for batch operations\n\n**Scale Dimension: File Size**\n\n- **Small files:** Sequential acceptable\n- **Large files (GB):** Sequential blocks everything\n- **Reveals:** Need async job queue, progress tracking\n\nExtreme testing: Use parallel processing with job queue."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can we fetch all data then filter in JavaScript?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Data Size**\n\n- **100 records:** Fetch all, filter client-side, works\n- **1M records:**\n  - Transfer time: minutes\n  - Browser memory: crash\n  - Bandwidth costs: extreme\n- **Reveals:** Filter server-side (SQL WHERE clause)\n\n**Scale Dimension: Network**\n\n- **LAN (fast):** Might work\n- **Mobile (slow):** Unusable, timeout\n- **Reveals:** Always filter server-side\n\nExtreme testing: Filter in database, return only needed data."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Should we use UUIDs or auto-increment IDs?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Distribution**\n\n- **Single server:** Auto-increment perfect\n- **Distributed system (sharding):** \n  - Auto-increment: ID conflicts\n  - UUID: Works across all nodes\n- **Reveals:** UUIDs for distributed, auto-increment for single server\n\n**Scale Dimension: Security**\n\n- **Internal system:** Sequential IDs fine\n- **Public API:** Sequential IDs leak business metrics\n  - Competitor sees order #1000000\n  - Knows your volume\n- **Reveals:** UUIDs prevent information leakage\n\nExtreme testing: Use UUIDs for distributed systems or public APIs."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can we store user preferences in cookies?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Data Size**\n\n- **3 preferences (theme, language):** Cookies work (< 1KB)\n- **100 preferences:** Exceeds cookie size limit (4KB)\n- **Reveals:** Large preferences need server-side storage\n\n**Scale Dimension: Requests**\n\n- **1 page view:** Small cookie overhead acceptable\n- **1000 API calls:** Cookie sent with every request\n  - Bandwidth waste\n  - Slower responses\n- **Reveals:** Store server-side, send token only\n\nExtreme testing: Cookies for small, frequently-needed data only."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Is it ok to delete data immediately when user clicks delete?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Users**\n\n- **1 careful user:** Immediate delete works\n- **1000 users:** Someone will:\n  - Accidentally delete\n  - Blame your app\n  - Demand recovery\n- **Reveals:** Need soft delete or undo period\n\n**Scale Dimension: Data Value**\n\n- **Low value data:** Delete immediately acceptable\n- **Critical business data:** Immediate delete = disaster\n- **Reveals:** Soft delete + retention policy for important data\n\nExtreme testing: Implement soft delete with recovery window for user actions."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can we run database migrations during deployment?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Data Size**\n\n- **1000 rows:** Migration runs in seconds\n- **1B rows:** Migration runs for hours\n  - Deployment blocked\n  - Downtime unacceptable\n- **Reveals:** Need online migrations or background jobs\n\n**Scale Dimension: Traffic**\n\n- **Low traffic site:** Brief downtime acceptable\n- **24/7 global service:** Zero downtime required\n- **Reveals:** Blue-green deployment, backward-compatible migrations\n\nExtreme testing: Design for online migrations from day one."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Should we email users on every activity?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Activity Volume**\n\n- **1 activity/day:** Email acceptable\n- **100 activities/day:** Email spam, user unsubscribes\n- **Reveals:** Need digest emails or notification preferences\n\n**Scale Dimension: Users**\n\n- **10 users:** Send individual emails, works\n- **1M users:** Email service costs explode, rate limits hit\n- **Reveals:** Batch sending, email service optimization\n\nExtreme testing: Implement notification preferences and digest emails."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Is recursion ok for tree traversal?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Tree Depth**\n\n- **10 levels deep:** Recursion works\n- **10,000 levels deep:** Stack overflow, crash\n- **Reveals:** Need iterative approach with explicit stack\n\n**Scale Dimension: Tree Size**\n\n- **100 nodes:** Recursion fine\n- **1M nodes:** Stack exhaustion\n- **Reveals:** Iterative with queue for large trees\n\nExtreme testing: Use iteration for unbounded depth/size."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Can we use SELECT * in queries?"},
            {"role": "assistant", "content": "I'm using the Scale Game skill.\n\n**Scale Dimension: Column Count**\n\n- **3 columns:** SELECT * acceptable\n- **50 columns (JSONB, arrays, text):** \n  - Transfer gigabytes unnecessarily\n  - Memory bloat\n  - Slow queries\n- **Reveals:** Select only needed columns\n\n**Scale Dimension: Rows**\n\n- **10 rows:** Doesn't matter\n- **1M rows with SELECT *:** Network transfer explosion\n- **Reveals:** Projection + pagination essential\n\nExtreme testing: Always specify columns explicitly."}
        ]
    }
]

examples.extend(scale_examples)

# PROBLEM-SOLVING: When Stuck dispatching (10 examples)
stuck_examples = [
    {
        "messages": [
            {"role": "user", "content": "I'm stuck on this problem, not sure how to proceed"},
            {"role": "assistant", "content": "I'm using the When Stuck skill to identify the right technique.\n\nLet me understand what type of stuck you are:\n\n- **Complexity spiraling?** Same thing implemented 5+ ways, growing special cases\n- **Need innovation?** Conventional solutions don't fit\n- **Recurring patterns?** Same issue in different places\n- **Forced by assumptions?** Feel like \"this must be done this way\"\n- **Scale uncertainty?** Unsure if it will work in production\n- **Code broken?** Wrong behavior, test failing\n\nWhich describes your situation?"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "I keep adding special cases to handle different scenarios"},
            {"role": "assistant", "content": "**Stuck-type identified: Complexity spiraling**\n\nThis is the perfect use case for skills/problem-solving/simplification-cascades.\n\nThe pattern suggests each new requirement is adding another if/else branch, making the code increasingly complex. Let me apply simplification techniques to find the underlying pattern."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "I can't find an existing approach that fits this problem"},
            {"role": "assistant", "content": "**Stuck-type identified: Need innovation**\n\nThis calls for skills/problem-solving/collision-zone-thinking.\n\nConventional solutions aren't adequate, so we need to force metaphors from different domains to spark breakthrough ideas. Let me explore collision-zone thinking."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "I've solved similar problems before but keep reinventing the wheel"},
            {"role": "assistant", "content": "**Stuck-type identified: Recurring patterns**\n\nThis is a case for skills/problem-solving/meta-pattern-recognition.\n\nYou're seeing the same underlying issue across different domains. Let me identify the meta-pattern so we can create a reusable solution."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "The solution feels forced, like I'm fighting against constraints"},
            {"role": "assistant", "content": "**Stuck-type identified: Forced by assumptions**\n\nTime for skills/problem-solving/inversion-exercise.\n\nWhen solutions feel forced, we're often stuck on unquestioned assumptions. Let me invert the constraints to find what we're taking for granted."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "It works in dev but I'm worried about production scale"},
            {"role": "assistant", "content": "**Stuck-type identified: Scale uncertainty**\n\nLet me use skills/problem-solving/scale-game.\n\nI'll test your approach at extremes (1000x bigger, 1000x faster, etc.) to expose fundamental limits hidden at normal scales."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "My test is failing and I don't know why"},
            {"role": "assistant", "content": "**Stuck-type identified: Code broken**\n\nThis requires skills/debugging/systematic-debugging.\n\nI need to follow the four phases:\n1. Root cause investigation (not guessing fixes)\n2. Pattern analysis\n3. Hypothesis testing\n4. Implementation with failing test\n\nLet me start Phase 1: gathering evidence."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Multiple independent issues are blocking progress"},
            {"role": "assistant", "content": "**Stuck-type identified: Multiple independent problems**\n\nThis is perfect for skills/collaboration/dispatching-parallel-agents.\n\nSince these problems are independent, we can investigate them in parallel rather than sequentially, speeding up resolution."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "I see the symptom but can't find what's causing it"},
            {"role": "assistant", "content": "**Stuck-type identified: Root cause unknown**\n\nLet me use skills/debugging/root-cause-tracing.\n\nI'll trace backward through the call stack from the symptom to find where the bad value originates, fixing at the source rather than the symptom."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "I'm not sure which of these stuck-types applies"},
            {"role": "assistant", "content": "Let me ask clarifying questions:\n\n1. Is code behaving incorrectly (wrong output, test failure)?\n   - If YES → Debugging\n   - If NO → Continue\n\n2. Do you keep adding if/else or special cases?\n   - If YES → Simplification cascades\n   - If NO → Continue\n\n3. Does this feel like a solved problem but you can't find the solution?\n   - If YES → Meta-pattern recognition\n   - If NO → Continue\n\n4. Will this work at 1000x scale (users, data, speed)?\n   - If UNSURE → Scale game\n\nLet's start with #1: Is code behaving incorrectly?"}
        ]
    }
]

examples.extend(stuck_examples)

# Write all examples to dataset
output_file = '/Users/zachswift/projects/agent-power-tools/synthia/fine-tuning/dataset.jsonl'
with open(output_file, 'a') as f:
    for example in examples:
        f.write(json.dumps(example) + '\n')

print(f"✓ Generated {len(examples)} examples")
print(f"  - Collaboration (Brainstorming): {len(brainstorm_base) + len(more_brainstorm) + len(design_examples)}")
print(f"  - Debugging (Systematic): {len(debug_examples)}")
print(f"  - Testing (TDD): {len(tdd_examples)}")
print(f"  - Problem-Solving (Scale): {len(scale_examples)}")
print(f"  - Problem-Solving (When Stuck): {len(stuck_examples)}")
print(f"\n✓ Appended to {output_file}")
