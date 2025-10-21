#!/usr/bin/env python3
"""
Generate Flask + Jinja2 template CRUD examples to teach the model:
1. Recognize Flask/Python projects
2. Create HTML templates with Jinja2 (not React!)
3. Build simple CRUD interfaces with HTML/CSS/vanilla JS
4. Use appropriate technology for the project stack
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
# FLASK TODO APP - FULL CRUD EXAMPLE
# ============================================================================

flask_todo_crud = {
    "messages": [
        {"role": "user", "content": "I have a Python/Flask todo app. Could you build a template/front end for it that lets me do CRUD operations on the TODO backend?"},
        {
            "role": "assistant",
            "content": "I'll help you build HTML templates for your Flask todo app. First, let me check the Flask backend structure.",
            "tool_calls": [create_tool_call("call_1", "glob", {"pattern": "**/*.py"})]
        },
        {"role": "tool", "tool_call_id": "call_1", "name": "glob",
         "content": "app.py\nmodels.py"},
        {
            "role": "assistant",
            "content": "Let me check the Flask app to understand the routes.",
            "tool_calls": [create_tool_call("call_2", "read", {"file_path": "app.py"})]
        },
        {"role": "tool", "tool_call_id": "call_2", "name": "read",
         "content": "from flask import Flask, render_template, request, redirect, url_for\nfrom models import Todo, db\n\napp = Flask(__name__)\napp.config['SQLALCHEMY_DATABASE_URI'] = 'sqlite:///todos.db'\ndb.init_app(app)\n\n@app.route('/')\ndef index():\n    todos = Todo.query.all()\n    return render_template('index.html', todos=todos)\n\n@app.route('/add', methods=['POST'])\ndef add_task():\n    task = request.form.get('task')\n    new_todo = Todo(task=task, completed=False)\n    db.session.add(new_todo)\n    db.session.commit()\n    return redirect(url_for('index'))\n\n@app.route('/complete/<int:id>')\ndef complete_task(id):\n    todo = Todo.query.get(id)\n    todo.completed = True\n    db.session.commit()\n    return redirect(url_for('index'))\n\n@app.route('/delete/<int:id>')\ndef delete_task(id):\n    todo = Todo.query.get(id)\n    db.session.delete(todo)\n    db.session.commit()\n    return redirect(url_for('index'))\n\nif __name__ == '__main__':\n    app.run(debug=True)"},
        {
            "role": "assistant",
            "content": "Perfect! Your Flask app has routes for viewing, adding, completing, and deleting todos. I'll create HTML templates with Jinja2. First, let me create a base template.",
            "tool_calls": [create_tool_call("call_3", "bash", {"command": "mkdir -p templates", "description": "Create templates directory"})]
        },
        {"role": "tool", "tool_call_id": "call_3", "name": "bash", "content": "stdout:\n\nstderr:\n"},
        {
            "role": "assistant",
            "content": "Now I'll create the main template with HTML and CSS for the todo interface.",
            "tool_calls": [create_tool_call("call_4", "write", {
                "file_path": "templates/index.html",
                "content": """<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>Todo App</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 40px 20px;
        }
        .container {
            max-width: 600px;
            margin: 0 auto;
            background: white;
            border-radius: 12px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            overflow: hidden;
        }
        .header {
            background: #667eea;
            color: white;
            padding: 30px;
            text-align: center;
        }
        .header h1 {
            font-size: 32px;
            font-weight: 600;
        }
        .add-form {
            padding: 20px;
            border-bottom: 1px solid #eee;
            display: flex;
            gap: 10px;
        }
        .add-form input {
            flex: 1;
            padding: 12px 16px;
            border: 2px solid #e0e0e0;
            border-radius: 8px;
            font-size: 15px;
            transition: border-color 0.3s;
        }
        .add-form input:focus {
            outline: none;
            border-color: #667eea;
        }
        .add-form button {
            padding: 12px 24px;
            background: #667eea;
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 15px;
            font-weight: 600;
            cursor: pointer;
            transition: background 0.3s;
        }
        .add-form button:hover {
            background: #5568d3;
        }
        .todos {
            padding: 20px;
        }
        .todo-item {
            display: flex;
            align-items: center;
            padding: 15px;
            background: #f8f9fa;
            border-radius: 8px;
            margin-bottom: 10px;
            transition: all 0.3s;
        }
        .todo-item:hover {
            background: #e9ecef;
            transform: translateX(5px);
        }
        .todo-item.completed {
            opacity: 0.6;
        }
        .todo-item.completed .todo-text {
            text-decoration: line-through;
            color: #6c757d;
        }
        .todo-text {
            flex: 1;
            font-size: 16px;
            color: #333;
        }
        .todo-actions {
            display: flex;
            gap: 8px;
        }
        .btn {
            padding: 8px 16px;
            border: none;
            border-radius: 6px;
            font-size: 14px;
            cursor: pointer;
            transition: all 0.3s;
            text-decoration: none;
            display: inline-block;
        }
        .btn-complete {
            background: #28a745;
            color: white;
        }
        .btn-complete:hover {
            background: #218838;
        }
        .btn-delete {
            background: #dc3545;
            color: white;
        }
        .btn-delete:hover {
            background: #c82333;
        }
        .empty-state {
            text-align: center;
            padding: 60px 20px;
            color: #6c757d;
        }
        .empty-state svg {
            width: 80px;
            height: 80px;
            margin-bottom: 20px;
            opacity: 0.3;
        }
        .empty-state p {
            font-size: 18px;
        }
    </style>
</head>
<body>
    <div class=\"container\">
        <div class=\"header\">
            <h1>📝 My Todo List</h1>
        </div>

        <form action=\"/add\" method=\"POST\" class=\"add-form\">
            <input type=\"text\" name=\"task\" placeholder=\"Add a new task...\" required>
            <button type=\"submit\">Add Task</button>
        </form>

        <div class=\"todos\">
            {% if todos %}
                {% for todo in todos %}
                <div class=\"todo-item {% if todo.completed %}completed{% endif %}\">
                    <div class=\"todo-text\">{{ todo.task }}</div>
                    <div class=\"todo-actions\">
                        {% if not todo.completed %}
                        <a href=\"/complete/{{ todo.id }}\" class=\"btn btn-complete\">✓ Complete</a>
                        {% endif %}
                        <a href=\"/delete/{{ todo.id }}\" class=\"btn btn-delete\" onclick=\"return confirm('Delete this task?')\">🗑️ Delete</a>
                    </div>
                </div>
                {% endfor %}
            {% else %}
                <div class=\"empty-state\">
                    <svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">
                        <path d=\"M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2\"></path>
                    </svg>
                    <p>No tasks yet. Add one above to get started!</p>
                </div>
            {% endif %}
        </div>
    </div>
</body>
</html>"""
            })]
        },
        {"role": "tool", "tool_call_id": "call_4", "name": "write", "content": "File written successfully"},
        {
            "role": "assistant",
            "content": "Perfect! I've created a complete CRUD interface for your Flask todo app with:\n\n**Features:**\n- ✅ Clean, modern design with gradient background\n- ✅ Add new todos with a simple form\n- ✅ Mark todos as complete (they get strikethrough styling)\n- ✅ Delete todos with confirmation\n- ✅ Responsive design that works on mobile\n- ✅ Empty state when no todos exist\n- ✅ Hover effects and smooth transitions\n\n**Technology used:**\n- HTML with Jinja2 templating (perfect for Flask)\n- Embedded CSS for styling\n- Vanilla JavaScript for the delete confirmation\n- No React or build tools needed - works out of the box!\n\nYour Flask app's routes (`/`, `/add`, `/complete/<id>`, `/delete/<id>`) are all integrated. Just run `python app.py` and visit http://localhost:5000 to see it in action!"
        }
    ]
}

# ============================================================================
# FLASK BLOG APP - CREATE ARTICLE FORM
# ============================================================================

flask_blog_create = {
    "messages": [
        {"role": "user", "content": "Add a create article form to my Flask blog"},
        {
            "role": "assistant",
            "content": "I'll create an HTML template for creating articles. Let me check your Flask routes first.",
            "tool_calls": [create_tool_call("call_1", "read", {"file_path": "app.py"})]
        },
        {"role": "tool", "tool_call_id": "call_1", "name": "read",
         "content": "@app.route('/create', methods=['GET', 'POST'])\ndef create_article():\n    if request.method == 'POST':\n        title = request.form.get('title')\n        content = request.form.get('content')\n        article = Article(title=title, content=content)\n        db.session.add(article)\n        db.session.commit()\n        return redirect(url_for('index'))\n    return render_template('create.html')"},
        {
            "role": "assistant",
            "content": "Perfect! I'll create a form template for creating articles.",
            "tool_calls": [create_tool_call("call_2", "write", {
                "file_path": "templates/create.html",
                "content": """<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>Create Article</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .form-container {
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            margin-bottom: 30px;
        }
        .form-group {
            margin-bottom: 20px;
        }
        label {
            display: block;
            margin-bottom: 8px;
            color: #555;
            font-weight: 600;
        }
        input[type=\"text\"],
        textarea {
            width: 100%;
            padding: 12px;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 16px;
            font-family: inherit;
        }
        textarea {
            min-height: 300px;
            resize: vertical;
        }
        button {
            background: #007bff;
            color: white;
            padding: 12px 30px;
            border: none;
            border-radius: 4px;
            font-size: 16px;
            cursor: pointer;
        }
        button:hover {
            background: #0056b3;
        }
        .cancel {
            margin-left: 10px;
            background: #6c757d;
        }
        .cancel:hover {
            background: #545b62;
        }
    </style>
</head>
<body>
    <div class=\"form-container\">
        <h1>Create New Article</h1>
        <form action=\"/create\" method=\"POST\">
            <div class=\"form-group\">
                <label for=\"title\">Title</label>
                <input type=\"text\" id=\"title\" name=\"title\" required>
            </div>
            <div class=\"form-group\">
                <label for=\"content\">Content</label>
                <textarea id=\"content\" name=\"content\" required></textarea>
            </div>
            <button type=\"submit\">Publish Article</button>
            <a href=\"/\"><button type=\"button\" class=\"cancel\">Cancel</button></a>
        </form>
    </div>
</body>
</html>"""
            })]
        },
        {"role": "tool", "tool_call_id": "call_2", "name": "write", "content": "File written successfully"},
        {"role": "assistant", "content": "Created templates/create.html with a clean form for creating articles. The form includes:\n- Title input field\n- Large text area for content\n- Publish and Cancel buttons\n- Responsive design with clean styling"}
    ]
}

# ============================================================================
# PATTERN: User tries React/create-react-app on Flask project
# ============================================================================

wrong_tech_stack_example = {
    "messages": [
        {"role": "user", "content": "Build a frontend for my Flask API"},
        {
            "role": "assistant",
            "content": "I'll create a frontend for your Flask app. Let me first check what type of project this is.",
            "tool_calls": [create_tool_call("call_1", "glob", {"pattern": "**/*.py"})]
        },
        {"role": "tool", "tool_call_id": "call_1", "name": "glob",
         "content": "app.py\nmodels.py\nroutes.py"},
        {
            "role": "assistant",
            "content": "This is a Flask/Python project. Let me check for any existing templates directory.",
            "tool_calls": [create_tool_call("call_2", "bash", {"command": "ls -la", "description": "List directory contents"})]
        },
        {"role": "tool", "tool_call_id": "call_2", "name": "bash",
         "content": "stdout:\ntotal 32\ndrwxr-xr-x  5 user user 4096 Jan 15 10:00 .\ndrwxr-xr-x  3 user user 4096 Jan 14 09:00 ..\n-rw-r--r--  1 user user  456 Jan 15 10:00 app.py\n-rw-r--r--  1 user user  234 Jan 15 09:30 models.py\n-rw-r--r--  1 user user  123 Jan 15 09:15 routes.py\n\nstderr:\n"},
        {
            "role": "assistant",
            "content": "For a Flask project, I'll create HTML templates with Jinja2 instead of a separate React app. This is the standard Flask approach and requires no build tools. Let me create the templates directory and a base template.",
            "tool_calls": [create_tool_call("call_3", "bash", {"command": "mkdir -p templates static/css", "description": "Create templates and static directories"})]
        },
        {"role": "tool", "tool_call_id": "call_3", "name": "bash", "content": "stdout:\n\nstderr:\n"},
        {
            "role": "assistant",
            "content": "Now I'll create a base HTML template with Jinja2.",
            "tool_calls": [create_tool_call("call_4", "write", {
                "file_path": "templates/base.html",
                "content": """<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>{% block title %}Flask App{% endblock %}</title>
    <link rel=\"stylesheet\" href=\"{{ url_for('static', filename='css/style.css') }}\">
</head>
<body>
    <nav>
        <div class=\"container\">
            <h1>My Flask App</h1>
            <ul>
                <li><a href=\"/\">Home</a></li>
                <li><a href=\"/about\">About</a></li>
            </ul>
        </div>
    </nav>
    <main class=\"container\">
        {% block content %}{% endblock %}
    </main>
</body>
</html>"""
            })]
        },
        {"role": "tool", "tool_call_id": "call_4", "name": "write", "content": "File written successfully"},
        {"role": "assistant", "content": "Created a Jinja2 template structure for your Flask app. This is the recommended approach for Flask projects - simple HTML templates that Flask can render directly, no React or build process needed!"}
    ]
}

# ============================================================================
# GENERATE ALL EXAMPLES
# ============================================================================

def generate_all_flask_examples():
    """Generate Flask + Jinja2 template examples."""
    examples = []

    # Add main examples
    examples.append(flask_todo_crud)
    examples.append(flask_blog_create)
    examples.append(wrong_tech_stack_example)

    # Create variations with different CRUD operations
    variations = []

    # TODO list variations
    for i in range(5):
        variations.append({
            "messages": flask_todo_crud["messages"]
        })

    # Blog variations
    for i in range(3):
        variations.append({
            "messages": flask_blog_create["messages"]
        })

    # Wrong tech stack variations
    for i in range(3):
        variations.append({
            "messages": wrong_tech_stack_example["messages"]
        })

    examples.extend(variations)

    random.shuffle(examples)
    return examples

if __name__ == "__main__":
    examples = generate_all_flask_examples()

    print(f"Generated {len(examples)} Flask/Jinja2 examples")

    # Write to JSONL
    output_path = "/Users/zachswift/projects/agent-power-tools/synthia/fine-tuning/data/flask_templates.jsonl"
    with open(output_path, "w") as f:
        for example in examples:
            f.write(json.dumps(example) + "\n")

    print(f"Written to: {output_path}")
