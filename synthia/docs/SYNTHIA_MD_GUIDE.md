# Using .SYNTHIA.md for Project-Specific Instructions

## Overview

Synthia supports project-level custom instructions via `.synthia/.SYNTHIA.md`, similar to Claude Code's `.claude/CLAUDE.md` pattern.

## How It Works

1. **Auto-creation:** When you start Synthia in a directory, it automatically creates:
   - `.synthia/` directory
   - `.synthia/.SYNTHIA.md` (empty file)

2. **Reading:** Synthia reads `.SYNTHIA.md` at startup and injects the content as a system message

3. **Updates:** Changes to `.SYNTHIA.md` require restarting Synthia to take effect

## Usage

### Basic Example

Add custom instructions to `.synthia/.SYNTHIA.md`:

```markdown
You are a helpful coding assistant working on a Python web application.

Project context:
- Using FastAPI framework
- PostgreSQL database
- Following PEP 8 style guide
- All API responses should include error handling
```

### What to Include

- **Project conventions:** Coding style, naming patterns, architecture rules
- **Context:** What the project does, key technologies used
- **Constraints:** Requirements, limitations, gotchas
- **Preferences:** Response format, level of detail, examples vs explanations

### Example: API Project

```markdown
This is a REST API for a task management system.

Stack:
- FastAPI (Python 3.11)
- PostgreSQL with SQLAlchemy ORM
- Pydantic for validation
- pytest for testing

Guidelines:
- All endpoints must have OpenAPI docs
- Use dependency injection for database sessions
- Write tests for every new endpoint
- Follow RESTful conventions (GET/POST/PUT/DELETE)
```

### Example: Code Review Focus

```markdown
When reviewing code, prioritize:
1. Security vulnerabilities
2. Performance issues
3. Code duplication
4. Missing error handling
5. Unclear variable names

Be direct and specific. Suggest fixes, don't just point out problems.
```

## Best Practices

- **Be specific:** Vague instructions get vague results
- **Keep it concise:** Focus on what's unique to your project
- **Update regularly:** Add new conventions as the project evolves
- **Commit it:** Share instructions with your team via git

## .gitignore

The `.synthia/` directory can be:
- **Committed:** Share instructions with team
- **Ignored:** Keep instructions personal

Add to `.gitignore` to ignore:
```
.synthia/
```

## Troubleshooting

**Instructions not working?**
- Restart Synthia (changes only loaded at startup)
- Check `/tmp/synthia.log` for "Loaded project-specific instructions" message
- Verify `.SYNTHIA.md` is not empty (empty files are ignored)

**Directory not created?**
- Verify you have write permissions in current directory
- Check `/tmp/synthia.log` for errors
