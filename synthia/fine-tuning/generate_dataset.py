#!/usr/bin/env python3
"""
Generate a comprehensive training dataset for fine-tuning Qwen 2.5 Coder 7B
to use Synthia's tools proactively.
"""

import json
import random
import os

def create_tool_call(tool_id, name, arguments_dict):
    """Create a tool call structure."""
    return {
        "id": tool_id,
        "type": "function",
        "function": {
            "name": name,
            "arguments": json.dumps(arguments_dict)
        }
    }

# ============================================================================
# BASH TOOL EXAMPLES
# ============================================================================

bash_examples = [
    {
        "messages": [
            {"role": "user", "content": "Check if the server is running on port 3000"},
            {
                "role": "assistant",
                "content": "I'll check what's running on port 3000.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "lsof -i :3000", "description": "Check port 3000 usage"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\nnode    12345 user   20u  IPv4  0x123  0t0  TCP *:3000 (LISTEN)"},
            {"role": "assistant", "content": "Yes, there's a Node.js server running on port 3000 (PID 12345)."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "What Python version do we have?"},
            {
                "role": "assistant",
                "content": "Let me check the Python version.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "python --version", "description": "Check Python version"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "Python 3.11.5"},
            {"role": "assistant", "content": "You have Python 3.11.5 installed."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Run the tests"},
            {
                "role": "assistant",
                "content": "I'll run the test suite now.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "pytest tests/", "description": "Run pytest test suite"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "============================= test session starts ==============================\nplatform linux -- Python 3.11.5, pytest-7.4.0\ncollected 24 items\n\ntests/test_auth.py ....                                                  [ 16%]\ntests/test_api.py ..........                                             [ 58%]\ntests/test_utils.py ..........                                           [100%]\n\n============================== 24 passed in 2.31s ==============================="},
            {"role": "assistant", "content": "All tests passed! 24 tests completed successfully in 2.31 seconds."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "What's the size of the build directory?"},
            {
                "role": "assistant",
                "content": "I'll check the build directory size.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "du -sh build/", "description": "Get build directory size"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "127M\tbuild/"},
            {"role": "assistant", "content": "The build directory is 127 MB in size."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "List all Docker containers"},
            {
                "role": "assistant",
                "content": "I'll list all Docker containers.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "docker ps -a", "description": "List all Docker containers"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "CONTAINER ID   IMAGE          COMMAND                  CREATED         STATUS         PORTS                    NAMES\n5a3d9e8c1f2b   postgres:15    \"docker-entrypoint.s…\"   2 days ago      Up 2 days      0.0.0.0:5432->5432/tcp   my-postgres\nc7b4f1a2e8d6   redis:7        \"docker-entrypoint.s…\"   3 days ago      Up 3 days      0.0.0.0:6379->6379/tcp   my-redis"},
            {"role": "assistant", "content": "You have 2 Docker containers running:\n1. PostgreSQL 15 (my-postgres) - running for 2 days on port 5432\n2. Redis 7 (my-redis) - running for 3 days on port 6379"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Install the dependencies"},
            {
                "role": "assistant",
                "content": "I'll install the project dependencies using npm.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "npm install", "description": "Install npm dependencies"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "added 342 packages, and audited 343 packages in 12s\n\n54 packages are looking for funding\n  run `npm fund` for details\n\nfound 0 vulnerabilities"},
            {"role": "assistant", "content": "Dependencies installed successfully! Added 342 packages with no vulnerabilities found."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Check disk usage"},
            {
                "role": "assistant",
                "content": "I'll check the disk usage.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "df -h", "description": "Check disk usage"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "Filesystem      Size  Used Avail Use% Mounted on\n/dev/sda1       100G   45G   50G  48% /\n/dev/sdb1       500G  320G  155G  68% /home"},
            {"role": "assistant", "content": "Disk usage:\n- Root partition (/): 45GB used out of 100GB (48%)\n- Home partition (/home): 320GB used out of 500GB (68%)"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Find large files in the project"},
            {
                "role": "assistant",
                "content": "I'll find the largest files in the project.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "find . -type f -size +10M -exec ls -lh {} \\; | sort -k5 -hr | head -10", "description": "Find files larger than 10MB"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "-rw-r--r-- 1 user user 45M Dec 15 10:23 ./dist/bundle.js\n-rw-r--r-- 1 user user 32M Dec 14 09:15 ./data/training.csv\n-rw-r--r-- 1 user user 18M Dec 13 14:30 ./logs/app.log"},
            {"role": "assistant", "content": "Found 3 large files:\n1. dist/bundle.js - 45 MB\n2. data/training.csv - 32 MB\n3. logs/app.log - 18 MB\n\nYou might want to review the bundle size and consider log rotation."}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Build and run the project"},
            {
                "role": "assistant",
                "content": "I'll build the project first.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "npm run build", "description": "Build the project"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "> project@1.0.0 build\n> webpack --mode production\n\nHash: 5f8a3d2b1c9e4a7f\nVersion: webpack 5.88.2\nTime: 3245ms\nBuilt at: 2024-01-15 10:23:45\n  Asset      Size  Chunks             Chunk Names\nmain.js  245 KiB       0  [emitted]  main\n✔ webpack compiled successfully"},
            {
                "role": "assistant",
                "content": "Build completed successfully! Now I'll start the server.",
                "tool_calls": [create_tool_call("call_2", "bash", {"command": "npm start", "description": "Start the server", "run_in_background": True})]
            },
            {"role": "tool", "tool_call_id": "call_2", "name": "bash", "content": "> project@1.0.0 start\n> node dist/main.js\n\nServer listening on port 3000"},
            {"role": "assistant", "content": "Project built and server started successfully on port 3000!"}
        ]
    },
]

# Generate additional bash examples programmatically
bash_commands = [
    ("ls -la", "List directory contents", "total 48\ndrwxr-xr-x  8 user user 4096 Dec 15 10:00 .\ndrwxr-xr-x  3 user user 4096 Dec 14 09:00 ..\n-rw-r--r--  1 user user  245 Dec 15 10:00 README.md\n-rw-r--r--  1 user user 1234 Dec 15 09:30 app.py"),
    ("git log --oneline -5", "Show recent commits", "a1b2c3d feat: add new feature\nb2c3d4e fix: resolve bug\nc3d4e5f docs: update README\nd4e5f6a refactor: clean code\ne5f6a1b chore: update deps"),
    ("npm list --depth=0", "List npm packages", "project@1.0.0\n├── express@4.18.2\n├── lodash@4.17.21\n└── axios@1.6.0"),
    ("ps aux | grep node", "Find Node processes", "user     12345  2.5  1.2 1234567 123456 ?  Sl   10:00   0:15 node server.js"),
    ("tail -n 20 logs/error.log", "View recent error logs", "[2024-01-15 10:23:45] ERROR: Connection timeout\n[2024-01-15 10:24:12] ERROR: Invalid token"),
    ("curl -I https://api.example.com", "Check API status", "HTTP/2 200\ndate: Mon, 15 Jan 2024 10:00:00 GMT\ncontent-type: application/json"),
]

for cmd, desc, output in bash_commands:
    bash_examples.append({
        "messages": [
            {"role": "user", "content": f"Can you {desc.lower()}?"},
            {
                "role": "assistant",
                "content": f"I'll {desc.lower()}.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": cmd, "description": desc})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": output},
            {"role": "assistant", "content": f"Here are the results."}
        ]
    })

# ============================================================================
# READ TOOL EXAMPLES
# ============================================================================

read_examples = [
    {
        "messages": [
            {"role": "user", "content": "What's in the config file?"},
            {
                "role": "assistant",
                "content": "I'll read the config file for you.",
                "tool_calls": [create_tool_call("call_1", "read", {"file_path": "/app/config.json"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "read", "content": '{\n  "port": 3000,\n  "database": {\n    "host": "localhost",\n    "port": 5432,\n    "name": "mydb"\n  },\n  "logging": {\n    "level": "info"\n  }\n}'},
            {"role": "assistant", "content": "The config file shows:\n- Server running on port 3000\n- Database: localhost:5432 (mydb)\n- Logging level: info"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Show me the main app file"},
            {
                "role": "assistant",
                "content": "I'll read the main application file.",
                "tool_calls": [create_tool_call("call_1", "read", {"file_path": "/app/main.py"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "read", "content": 'from flask import Flask, jsonify\nfrom database import db\n\napp = Flask(__name__)\napp.config.from_file("config.json", load=json.load)\n\n@app.route("/health")\ndef health():\n    return jsonify({"status": "ok"})\n\nif __name__ == "__main__":\n    app.run(port=3000)'},
            {"role": "assistant", "content": "The main app file is a Flask application with:\n- Config loaded from config.json\n- A /health endpoint that returns status\n- Server running on port 3000"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "What dependencies are installed?"},
            {
                "role": "assistant",
                "content": "I'll check the package.json file.",
                "tool_calls": [create_tool_call("call_1", "read", {"file_path": "/app/package.json"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "read", "content": '{\n  "name": "my-app",\n  "version": "1.0.0",\n  "dependencies": {\n    "express": "^4.18.2",\n    "mongoose": "^8.0.3",\n    "dotenv": "^16.3.1"\n  },\n  "devDependencies": {\n    "jest": "^29.7.0",\n    "eslint": "^8.55.0"\n  }\n}'},
            {"role": "assistant", "content": "Dependencies:\n- express: ^4.18.2 (web framework)\n- mongoose: ^8.0.3 (MongoDB ODM)\n- dotenv: ^16.3.1 (environment variables)\n\nDev dependencies:\n- jest: ^29.7.0 (testing)\n- eslint: ^8.55.0 (linting)"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Look at the README"},
            {
                "role": "assistant",
                "content": "I'll read the README file.",
                "tool_calls": [create_tool_call("call_1", "read", {"file_path": "/app/README.md"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "read", "content": "# My Project\n\nA web application for managing tasks.\n\n## Installation\n\n```bash\nnpm install\n```\n\n## Usage\n\n```bash\nnpm start\n```\n\n## Testing\n\n```bash\nnpm test\n```"},
            {"role": "assistant", "content": "This is a task management web application. Setup:\n1. Install: `npm install`\n2. Run: `npm start`\n3. Test: `npm test`"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Check the error that occurred"},
            {
                "role": "assistant",
                "content": "Let me read the error log file.",
                "tool_calls": [create_tool_call("call_1", "read", {"file_path": "/var/log/app/error.log", "limit": 50})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "read", "content": "[2024-01-15 10:23:45] ERROR: Database connection failed\n[2024-01-15 10:23:45] ERROR: ECONNREFUSED 127.0.0.1:5432\n[2024-01-15 10:24:12] ERROR: Retry attempt 1 failed\n[2024-01-15 10:24:22] ERROR: Retry attempt 2 failed"},
            {"role": "assistant", "content": "The error log shows database connection failures. The app can't connect to PostgreSQL on localhost:5432. The database service might be down or not started yet."}
        ]
    },
]

# ============================================================================
# GREP/GLOB EXAMPLES
# ============================================================================

grep_glob_examples = [
    {
        "messages": [
            {"role": "user", "content": "Find all TODO comments in the codebase"},
            {
                "role": "assistant",
                "content": "I'll search for TODO comments.",
                "tool_calls": [create_tool_call("call_1", "grep", {"pattern": "TODO", "output_mode": "content", "-n": True})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "grep", "content": "src/main.py:45:    # TODO: optimize this function\nsrc/api.py:123:    # TODO: add error handling\nsrc/utils.py:67:    # TODO: refactor this logic"},
            {"role": "assistant", "content": "Found 3 TODO comments:\n1. src/main.py:45 - optimize function\n2. src/api.py:123 - add error handling\n3. src/utils.py:67 - refactor logic"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Where is the login function defined?"},
            {
                "role": "assistant",
                "content": "I'll search for the login function.",
                "tool_calls": [create_tool_call("call_1", "grep", {"pattern": "def login|function login|const login", "output_mode": "content", "-n": True})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "grep", "content": "src/auth.py:34:def login(username, password):\ntest/auth.test.js:12:  async function login(credentials) {"},
            {"role": "assistant", "content": "Found the login function at:\n- src/auth.py:34 (main implementation)\n- test/auth.test.js:12 (test helper)"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Find all Python files"},
            {
                "role": "assistant",
                "content": "I'll search for Python files.",
                "tool_calls": [create_tool_call("call_1", "glob", {"pattern": "**/*.py"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "glob", "content": "src/main.py\nsrc/auth.py\nsrc/api.py\nsrc/utils.py\ntests/test_main.py\ntests/test_auth.py"},
            {"role": "assistant", "content": "Found 6 Python files:\n- 4 in src/ (main, auth, api, utils)\n- 2 test files in tests/"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "List all TypeScript files"},
            {
                "role": "assistant",
                "content": "I'll find all TypeScript files.",
                "tool_calls": [create_tool_call("call_1", "glob", {"pattern": "**/*.ts"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "glob", "content": "src/index.ts\nsrc/types.ts\nsrc/utils.ts\nsrc/api/client.ts\nsrc/api/types.ts"},
            {"role": "assistant", "content": "Found 5 TypeScript files across src/ and src/api/."}
        ]
    },
]

# ============================================================================
# WRITE/EDIT EXAMPLES
# ============================================================================

write_edit_examples = [
    {
        "messages": [
            {"role": "user", "content": "Create a .gitignore file"},
            {
                "role": "assistant",
                "content": "I'll create a comprehensive .gitignore file.",
                "tool_calls": [create_tool_call("call_1", "write", {
                    "file_path": "/app/.gitignore",
                    "content": "# Dependencies\nnode_modules/\n\n# Environment\n.env\n.env.local\n\n# Logs\nlogs/\n*.log\n\n# Build\ndist/\nbuild/\n\n# IDE\n.vscode/\n.idea/\n\n# OS\n.DS_Store\nThumbs.db"
                })]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "write", "content": "File written successfully"},
            {"role": "assistant", "content": "Created .gitignore with common exclusions:\n- Dependencies (node_modules)\n- Environment files (.env)\n- Logs\n- Build artifacts\n- IDE settings\n- OS files"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Update the port in the config"},
            {
                "role": "assistant",
                "content": "Let me read the config first.",
                "tool_calls": [create_tool_call("call_1", "read", {"file_path": "/app/config.json"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "read", "content": '{\n  "port": 3000,\n  "env": "development"\n}'},
            {
                "role": "assistant",
                "content": "I'll change the port from 3000 to 8080.",
                "tool_calls": [create_tool_call("call_2", "edit", {
                    "file_path": "/app/config.json",
                    "old_string": '  "port": 3000,',
                    "new_string": '  "port": 8080,'
                })]
            },
            {"role": "tool", "tool_call_id": "call_2", "name": "edit", "content": "Edit successful"},
            {"role": "assistant", "content": "Updated the port from 3000 to 8080 in the configuration."}
        ]
    },
]

# ============================================================================
# GIT EXAMPLES
# ============================================================================

git_examples = [
    {
        "messages": [
            {"role": "user", "content": "Check the git status"},
            {
                "role": "assistant",
                "content": "I'll check the git status.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "git status", "description": "Check git status"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "On branch main\nYour branch is up to date with 'origin/main'.\n\nChanges not staged for commit:\n  modified:   src/app.js\n  modified:   src/utils.js\n\nUntracked files:\n  src/new-feature.js"},
            {"role": "assistant", "content": "Git status:\n- Branch: main (up to date)\n- Modified: src/app.js, src/utils.js\n- New file: src/new-feature.js"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Show me the recent commits"},
            {
                "role": "assistant",
                "content": "I'll show the recent commit history.",
                "tool_calls": [create_tool_call("call_1", "bash", {"command": "git log --oneline -10", "description": "Show recent commits"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "a1b2c3d feat: add user authentication\nb2c3d4e fix: resolve database connection issue\nc3d4e5f docs: update API documentation\nd4e5f6a refactor: improve error handling\ne5f6a1b chore: update dependencies"},
            {"role": "assistant", "content": "Recent commits:\n1. feat: add user authentication\n2. fix: resolve database connection issue\n3. docs: update API documentation\n4. refactor: improve error handling\n5. chore: update dependencies"}
        ]
    },
]

# ============================================================================
# POWERTOOLS EXAMPLES
# ============================================================================

powertools_examples = [
    {
        "messages": [
            {"role": "user", "content": "Where is the fetchUser function defined?"},
            {
                "role": "assistant",
                "content": "I'll use goto_definition to find where fetchUser is defined.",
                "tool_calls": [create_tool_call("call_1", "mcp__powertools__goto_definition", {"location": "src/app.ts:42:10"})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "mcp__powertools__goto_definition", "content": '{"file": "src/api/users.ts", "line": 15, "column": 17, "symbol": "fetchUser"}'},
            {"role": "assistant", "content": "The fetchUser function is defined at:\nsrc/api/users.ts:15:17"}
        ]
    },
    {
        "messages": [
            {"role": "user", "content": "Find all places where the User model is used"},
            {
                "role": "assistant",
                "content": "I'll search for all references to the User model.",
                "tool_calls": [create_tool_call("call_1", "mcp__powertools__find_references", {"symbol": "User", "limit": 100})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "mcp__powertools__find_references", "content": '{"count": 12, "has_more": false, "references": [{"file": "src/models/User.ts", "line": 5, "column": 14}, {"file": "src/api/auth.ts", "line": 23, "column": 18}, {"file": "src/api/users.ts", "line": 8, "column": 20}]}'},
            {"role": "assistant", "content": "Found 12 references to User model:\n- src/models/User.ts:5 (definition)\n- src/api/auth.ts:23 (usage)\n- src/api/users.ts:8 (import)\n- ...and 9 more locations"}
        ]
    },
]

# ============================================================================
# WEBFETCH EXAMPLES
# ============================================================================

webfetch_examples = [
    {
        "messages": [
            {"role": "user", "content": "Check the latest TypeScript release"},
            {
                "role": "assistant",
                "content": "I'll fetch the TypeScript releases page.",
                "tool_calls": [create_tool_call("call_1", "webfetch", {
                    "url": "https://github.com/microsoft/TypeScript/releases",
                    "prompt": "What is the latest version and its release date?"
                })]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "webfetch", "content": "The latest version is TypeScript 5.3.3, released on January 10, 2024."},
            {"role": "assistant", "content": "Latest TypeScript version: 5.3.3 (released January 10, 2024)"}
        ]
    },
]

# ============================================================================
# WORKSHOP EXAMPLES
# ============================================================================

workshop_examples = [
    {
        "messages": [
            {"role": "user", "content": "Remember that we decided to use PostgreSQL instead of MongoDB"},
            {
                "role": "assistant",
                "content": "I'll record this architectural decision.",
                "tool_calls": [create_tool_call("call_1", "bash", {
                    "command": "workshop decision \"Use PostgreSQL instead of MongoDB\" -r \"Better support for complex queries and transactions\"",
                    "description": "Record architecture decision"
                })]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "bash", "content": "Decision recorded successfully"},
            {"role": "assistant", "content": "Recorded the decision to use PostgreSQL with reasoning about query and transaction support."}
        ]
    },
]

# ============================================================================
# COMPLEX MULTI-TOOL WORKFLOWS
# ============================================================================

complex_examples = [
    {
        "messages": [
            {"role": "user", "content": "Debug the authentication error"},
            {
                "role": "assistant",
                "content": "I'll help debug this. First, let me check the error logs.",
                "tool_calls": [create_tool_call("call_1", "read", {"file_path": "/var/log/app/error.log", "limit": 30})]
            },
            {"role": "tool", "tool_call_id": "call_1", "name": "read", "content": "[2024-01-15 10:23:45] ERROR: JWT verification failed\n[2024-01-15 10:23:45] ERROR: JsonWebTokenError: invalid signature\n[2024-01-15 10:24:12] ERROR: Retry attempt 1 failed\n[2024-01-15 10:24:22] ERROR: Retry attempt 2 failed"},
            {
                "role": "assistant",
                "content": "The error is JWT signature verification failing. Let me check the auth middleware.",
                "tool_calls": [create_tool_call("call_2", "read", {"file_path": "/app/middleware/auth.js"})]
            },
            {"role": "tool", "tool_call_id": "call_2", "name": "read", "content": "const jwt = require('jsonwebtoken');\n\nfunction authenticate(req, res, next) {\n  const token = req.header('Authorization')?.replace('Bearer ', '');\n  try {\n    const decoded = jwt.verify(token, process.env.JWT_SECRET);\n    req.user = decoded;\n    next();\n  } catch (err) {\n    res.status(401).json({ error: 'Invalid token' });\n  }\n}"},
            {
                "role": "assistant",
                "content": "Now let me check if JWT_SECRET is set in the environment.",
                "tool_calls": [create_tool_call("call_3", "bash", {"command": "grep JWT_SECRET /app/.env", "description": "Check JWT_SECRET in .env"})]
            },
            {"role": "tool", "tool_call_id": "call_3", "name": "bash", "content": "JWT_SECRET=newsecret123"},
            {"role": "assistant", "content": "Found the issue! The JWT_SECRET was recently changed in the .env file. Existing tokens were signed with the old secret and can't be verified with the new one. Users need to log in again to get new tokens signed with the updated secret."}
        ]
    },
]

# ============================================================================
# ASSEMBLE AND WRITE DATASET
# ============================================================================

# Collect all examples
all_examples = []
all_examples.extend(bash_examples * 4)  # Bash is 25%
all_examples.extend(read_examples * 3)  # Read is 20%
all_examples.extend(grep_glob_examples * 3)  # Grep/glob is 15%
all_examples.extend(write_edit_examples * 3)  # Write/edit is 15%
all_examples.extend(git_examples * 2)  # Git is 10%
all_examples.extend(powertools_examples * 2)  # Powertools is 8%
all_examples.extend(webfetch_examples * 2)  # Webfetch is 4%
all_examples.extend(workshop_examples * 2)  # Workshop is 3%
all_examples.extend(complex_examples * 4)  # Complex is 20%

# Shuffle for variety
random.shuffle(all_examples)

# Limit to 250 examples
all_examples = all_examples[:250]

# Print statistics
tool_counts = {}
for example in all_examples:
    for message in example["messages"]:
        if "tool_calls" in message:
            for tool_call in message["tool_calls"]:
                tool_name = tool_call["function"]["name"]
                tool_counts[tool_name] = tool_counts.get(tool_name, 0) + 1

print(f"Generating {len(all_examples)} training examples...")
print("\nTool distribution:")
for tool, count in sorted(tool_counts.items(), key=lambda x: x[1], reverse=True):
    print(f"  {tool}: {count} calls")

# Write JSONL file
output_path = "/Users/zachswift/projects/agent-power-tools/synthia/fine-tuning/dataset.jsonl"
with open(output_path, "w") as f:
    for example in all_examples:
        f.write(json.dumps({"messages": example["messages"]}) + "\n")

file_size = os.path.getsize(output_path)
print(f"\nDataset written to: {output_path}")
print(f"Total examples: {len(all_examples)}")
print(f"File size: {file_size / 1024 / 1024:.2f} MB")
