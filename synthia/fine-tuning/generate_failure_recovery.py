#!/usr/bin/env python3
"""
Generate tool failure recovery examples to teach the model to:
1. Analyze errors instead of blindly retrying
2. Change strategy when a tool fails
3. Use context clues to choose appropriate tools
"""

import json
import random

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
# PATTERN 1: Wrong package manager - switch to correct one
# ============================================================================

package_manager_failures = [
    {
        "user_request": "Install the dependencies for this Python project",
        "wrong_tool": "bash",
        "wrong_command": "npm install",
        "wrong_desc": "Install npm dependencies",
        "error": "stderr:\nnpm error code ENOENT\nnpm error syscall open\nnpm error enoent ENOENT: no such file or directory, open 'package.json'",
        "analysis": "The error shows npm is looking for package.json, but this is a Python project. I should check for Python dependency files instead.",
        "discovery_tool": "glob",
        "discovery_args": {"pattern": "**/requirements.txt"},
        "discovery_result": "requirements.txt",
        "correct_tool": "bash",
        "correct_command": "pip install -r requirements.txt",
        "correct_desc": "Install Python dependencies",
        "success": "Successfully installed flask-2.0.3 requests-2.28.1 sqlalchemy-1.4.45",
        "final_message": "This is a Python project. I've installed the dependencies from requirements.txt successfully."
    },
    {
        "user_request": "Install project dependencies",
        "wrong_tool": "bash",
        "wrong_command": "pip install -r requirements.txt",
        "wrong_desc": "Install Python dependencies",
        "error": "stderr:\npip: error: No such file or directory: 'requirements.txt'",
        "analysis": "No requirements.txt found. Let me check for other package managers.",
        "discovery_tool": "glob",
        "discovery_args": {"pattern": "**/package.json"},
        "discovery_result": "package.json",
        "correct_tool": "bash",
        "correct_command": "npm install",
        "correct_desc": "Install npm dependencies",
        "success": "added 342 packages in 12s",
        "final_message": "Found package.json - this is a Node.js project. Dependencies installed successfully."
    },
    {
        "user_request": "Install dependencies",
        "wrong_tool": "bash",
        "wrong_command": "npm install",
        "wrong_desc": "Install npm dependencies",
        "error": "stderr:\nnpm error code ENOENT\nnpm error syscall open\nnpm error enoent ENOENT: no such file or directory, open 'package.json'",
        "analysis": "No package.json. Let me check what type of project this is.",
        "discovery_tool": "glob",
        "discovery_args": {"pattern": "**/Cargo.toml"},
        "discovery_result": "Cargo.toml",
        "correct_tool": "bash",
        "correct_command": "cargo build --release",
        "correct_desc": "Build Rust project",
        "success": "Compiling myproject v0.1.0\n    Finished release [optimized] target(s) in 23.45s",
        "final_message": "This is a Rust project. Built successfully with cargo."
    },
    {
        "user_request": "Install dependencies for this project",
        "wrong_tool": "bash",
        "wrong_command": "pip install -r requirements.txt",
        "wrong_desc": "Install Python dependencies",
        "error": "stderr:\npip: error: No such file or directory: 'requirements.txt'",
        "analysis": "No requirements.txt. Checking for Gemfile (Ruby project).",
        "discovery_tool": "glob",
        "discovery_args": {"pattern": "**/Gemfile"},
        "discovery_result": "Gemfile",
        "correct_tool": "bash",
        "correct_command": "bundle install",
        "correct_desc": "Install Ruby gems",
        "success": "Fetching gem metadata from https://rubygems.org/\nBundle complete! 15 Gemfile dependencies, 42 gems now installed.",
        "final_message": "Found Gemfile - this is a Ruby project. Installed gems successfully."
    },
]

def generate_package_manager_examples():
    """Generate examples showing package manager error recovery."""
    examples = []

    for scenario in package_manager_failures:
        example = {
            "messages": [
                {"role": "user", "content": scenario["user_request"]},
                {
                    "role": "assistant",
                    "content": f"I'll {scenario['wrong_desc'].lower()}.",
                    "tool_calls": [create_tool_call("call_1", scenario["wrong_tool"],
                        {"command": scenario["wrong_command"], "description": scenario["wrong_desc"]})]
                },
                {"role": "tool", "tool_call_id": "call_1", "name": scenario["wrong_tool"],
                 "content": f"stdout:\n\nstderr:\n{scenario['error']}"},
                {
                    "role": "assistant",
                    "content": scenario["analysis"],
                    "tool_calls": [create_tool_call("call_2", scenario["discovery_tool"], scenario["discovery_args"])]
                },
                {"role": "tool", "tool_call_id": "call_2", "name": scenario["discovery_tool"],
                 "content": scenario["discovery_result"]},
                {
                    "role": "assistant",
                    "content": f"I'll use {scenario['correct_command']} instead.",
                    "tool_calls": [create_tool_call("call_3", scenario["correct_tool"],
                        {"command": scenario["correct_command"], "description": scenario["correct_desc"]})]
                },
                {"role": "tool", "tool_call_id": "call_3", "name": scenario["correct_tool"],
                 "content": f"stdout:\n{scenario['success']}\n\nstderr:\n"},
                {"role": "assistant", "content": scenario["final_message"]}
            ]
        }
        examples.append(example)

    return examples

# ============================================================================
# PATTERN 2: File not found - explore directory structure
# ============================================================================

file_not_found_scenarios = [
    {
        "user_request": "Show me the main application file",
        "wrong_path": "/app/main.py",
        "error": "File not found: /app/main.py",
        "analysis": "File doesn't exist at that path. Let me check what Python files are in the project.",
        "discovery_tool": "glob",
        "discovery_args": {"pattern": "**/*.py"},
        "discovery_result": "src/app.py\nsrc/routes.py\nsrc/models.py",
        "correct_path": "src/app.py",
        "file_content": "from flask import Flask\n\napp = Flask(__name__)\n\n@app.route('/')\ndef index():\n    return 'Hello World'\n\nif __name__ == '__main__':\n    app.run()",
        "final_message": "Found the main application file at src/app.py. It's a Flask application with a single index route."
    },
    {
        "user_request": "Read the configuration file",
        "wrong_path": "/app/config.json",
        "error": "File not found: /app/config.json",
        "analysis": "config.json not found. Let me search for configuration files.",
        "discovery_tool": "glob",
        "discovery_args": {"pattern": "**/config.*"},
        "discovery_result": "config/settings.yml\nconfig/database.yml",
        "correct_path": "config/settings.yml",
        "file_content": "app_name: MyApp\nport: 3000\ndebug: true",
        "final_message": "Found configuration at config/settings.yml instead of config.json. The app runs on port 3000 with debug enabled."
    },
]

def generate_file_not_found_examples():
    """Generate examples showing file not found error recovery."""
    examples = []

    for scenario in file_not_found_scenarios:
        example = {
            "messages": [
                {"role": "user", "content": scenario["user_request"]},
                {
                    "role": "assistant",
                    "content": f"I'll read {scenario['wrong_path']}.",
                    "tool_calls": [create_tool_call("call_1", "read", {"file_path": scenario["wrong_path"]})]
                },
                {"role": "tool", "tool_call_id": "call_1", "name": "read",
                 "content": scenario["error"]},
                {
                    "role": "assistant",
                    "content": scenario["analysis"],
                    "tool_calls": [create_tool_call("call_2", scenario["discovery_tool"], scenario["discovery_args"])]
                },
                {"role": "tool", "tool_call_id": "call_2", "name": scenario["discovery_tool"],
                 "content": scenario["discovery_result"]},
                {
                    "role": "assistant",
                    "content": f"Let me read {scenario['correct_path']} instead.",
                    "tool_calls": [create_tool_call("call_3", "read", {"file_path": scenario["correct_path"]})]
                },
                {"role": "tool", "tool_call_id": "call_3", "name": "read",
                 "content": scenario["file_content"]},
                {"role": "assistant", "content": scenario["final_message"]}
            ]
        }
        examples.append(example)

    return examples

# ============================================================================
# PATTERN 3: Permission denied - use alternative approach
# ============================================================================

permission_denied_scenarios = [
    {
        "user_request": "Check system logs for errors",
        "wrong_command": "cat /var/log/syslog",
        "error": "cat: /var/log/syslog: Permission denied",
        "analysis": "Permission denied for direct file access. Let me try using journalctl instead.",
        "correct_command": "journalctl -n 50 -p err",
        "correct_desc": "Check system journal for errors",
        "success": "-- Journal begins at Mon 2024-01-15 00:00:00 UTC --\nJan 15 10:23:45 server systemd[1]: Failed to start service.service",
        "final_message": "Found recent error logs using journalctl: systemd failed to start a service at 10:23:45."
    },
    {
        "user_request": "Install a system package",
        "wrong_command": "apt-get install nginx",
        "error": "E: Could not open lock file /var/lib/dpkg/lock - open (13: Permission denied)\nE: Unable to lock the administration directory (/var/lib/dpkg/), are you root?",
        "analysis": "Need root permissions to install packages. I'll use sudo.",
        "correct_command": "sudo apt-get install nginx",
        "correct_desc": "Install nginx with sudo",
        "success": "Reading package lists... Done\nBuilding dependency tree... Done\nnginx is already the newest version (1.18.0-6ubuntu14).",
        "final_message": "nginx is already installed at version 1.18.0."
    },
]

def generate_permission_denied_examples():
    """Generate examples showing permission denied error recovery."""
    examples = []

    for scenario in permission_denied_scenarios:
        example = {
            "messages": [
                {"role": "user", "content": scenario["user_request"]},
                {
                    "role": "assistant",
                    "content": f"I'll run {scenario['wrong_command']}.",
                    "tool_calls": [create_tool_call("call_1", "bash",
                        {"command": scenario["wrong_command"], "description": scenario["user_request"]})]
                },
                {"role": "tool", "tool_call_id": "call_1", "name": "bash",
                 "content": f"stdout:\n\nstderr:\n{scenario['error']}"},
                {
                    "role": "assistant",
                    "content": scenario["analysis"],
                    "tool_calls": [create_tool_call("call_2", "bash",
                        {"command": scenario["correct_command"], "description": scenario["correct_desc"]})]
                },
                {"role": "tool", "tool_call_id": "call_2", "name": "bash",
                 "content": f"stdout:\n{scenario['success']}\n\nstderr:\n"},
                {"role": "assistant", "content": scenario["final_message"]}
            ]
        }
        examples.append(example)

    return examples

# ============================================================================
# PATTERN 4: Command not found - use alternative tool
# ============================================================================

command_not_found_scenarios = [
    {
        "user_request": "Check what's using port 8080",
        "wrong_command": "lsof -i :8080",
        "error": "bash: lsof: command not found",
        "analysis": "lsof not installed. I'll use netstat instead.",
        "correct_command": "netstat -tulpn | grep :8080",
        "correct_desc": "Check port 8080 with netstat",
        "success": "tcp        0      0 0.0.0.0:8080            0.0.0.0:*               LISTEN      12345/node",
        "final_message": "Port 8080 is being used by a Node.js process (PID 12345)."
    },
    {
        "user_request": "Monitor system resources",
        "wrong_command": "htop",
        "error": "bash: htop: command not found",
        "analysis": "htop not available. I'll use top instead.",
        "correct_command": "top -bn1 | head -20",
        "correct_desc": "Monitor resources with top",
        "success": "top - 10:23:45 up 5 days,  3:21,  2 users,  load average: 0.52, 0.58, 0.59\nTasks: 142 total,   1 running, 141 sleeping",
        "final_message": "System is running normally with load average of 0.52. Using top output since htop isn't installed."
    },
]

def generate_command_not_found_examples():
    """Generate examples showing command not found error recovery."""
    examples = []

    for scenario in command_not_found_scenarios:
        example = {
            "messages": [
                {"role": "user", "content": scenario["user_request"]},
                {
                    "role": "assistant",
                    "content": f"I'll use {scenario['wrong_command']}.",
                    "tool_calls": [create_tool_call("call_1", "bash",
                        {"command": scenario["wrong_command"], "description": scenario["user_request"]})]
                },
                {"role": "tool", "tool_call_id": "call_1", "name": "bash",
                 "content": f"stdout:\n\nstderr:\n{scenario['error']}"},
                {
                    "role": "assistant",
                    "content": scenario["analysis"],
                    "tool_calls": [create_tool_call("call_2", "bash",
                        {"command": scenario["correct_command"], "description": scenario["correct_desc"]})]
                },
                {"role": "tool", "tool_call_id": "call_2", "name": "bash",
                 "content": f"stdout:\n{scenario['success']}\n\nstderr:\n"},
                {"role": "assistant", "content": scenario["final_message"]}
            ]
        }
        examples.append(example)

    return examples

# ============================================================================
# GENERATE ALL EXAMPLES
# ============================================================================

def generate_all_failure_recovery_examples():
    """Generate comprehensive failure recovery dataset."""
    examples = []

    # Generate base examples
    examples.extend(generate_package_manager_examples())
    examples.extend(generate_file_not_found_examples())
    examples.extend(generate_permission_denied_examples())
    examples.extend(generate_command_not_found_examples())

    # Multiply to reach several hundred
    base_count = len(examples)
    print(f"Base examples: {base_count}")

    # Repeat with variations to reach ~300 examples
    target = 300
    multiplier = target // base_count

    all_examples = examples * multiplier
    random.shuffle(all_examples)

    return all_examples[:target]

if __name__ == "__main__":
    examples = generate_all_failure_recovery_examples()

    print(f"Generated {len(examples)} failure recovery examples")

    # Write to JSONL
    output_path = "/Users/zachswift/projects/agent-power-tools/synthia/fine-tuning/data/failure_recovery.jsonl"
    with open(output_path, "w") as f:
        for example in examples:
            f.write(json.dumps(example) + "\n")

    print(f"Written to: {output_path}")
