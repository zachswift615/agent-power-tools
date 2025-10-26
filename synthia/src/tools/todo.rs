use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Todo {
    pub content: String,
    pub status: TodoStatus,
    pub active_form: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

pub struct TodoTool {
    todos: Arc<Mutex<Vec<Todo>>>,
}

impl TodoTool {
    pub fn new() -> Self {
        Self {
            todos: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn format_todos(&self) -> String {
        let todos = self.todos.lock().unwrap();

        if todos.is_empty() {
            return "No todos".to_string();
        }

        let mut output = String::from("Todo List:\n\n");

        for (i, todo) in todos.iter().enumerate() {
            let status_icon = match todo.status {
                TodoStatus::Pending => "☐",
                TodoStatus::InProgress => "⏳",
                TodoStatus::Completed => "✓",
            };

            let display_text = match todo.status {
                TodoStatus::InProgress => &todo.active_form,
                _ => &todo.content,
            };

            output.push_str(&format!("{}. {} {}\n", i + 1, status_icon, display_text));
        }

        output
    }
}

#[async_trait]
impl Tool for TodoTool {
    fn name(&self) -> &str {
        "todo"
    }

    fn description(&self) -> &str {
        "Manage a todo list to track progress through multi-step tasks"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "List of todos to set (replaces existing todos)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The task description (imperative form, e.g., 'Run tests')"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"],
                                "description": "Current status of the task"
                            },
                            "active_form": {
                                "type": "string",
                                "description": "Present continuous form for in-progress display (e.g., 'Running tests')"
                            }
                        },
                        "required": ["content", "status", "active_form"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let todos_json = params["todos"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'todos' parameter"))?;

        let new_todos: Vec<Todo> = serde_json::from_value(Value::Array(todos_json.clone()))?;

        {
            let mut todos = self.todos.lock().unwrap();
            *todos = new_todos;
        }

        let content = self.format_todos();

        Ok(ToolResult {
            content,
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_todo_empty() {
        let tool = TodoTool::new();

        let result = tool
            .execute(serde_json::json!({
                "todos": []
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content, "No todos");
    }

    #[tokio::test]
    async fn test_todo_single_pending() {
        let tool = TodoTool::new();

        let result = tool
            .execute(serde_json::json!({
                "todos": [
                    {
                        "content": "Write tests",
                        "status": "pending",
                        "active_form": "Writing tests"
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("☐"));
        assert!(result.content.contains("Write tests"));
    }

    #[tokio::test]
    async fn test_todo_in_progress_shows_active_form() {
        let tool = TodoTool::new();

        let result = tool
            .execute(serde_json::json!({
                "todos": [
                    {
                        "content": "Implement feature",
                        "status": "in_progress",
                        "active_form": "Implementing feature"
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("⏳"));
        assert!(result.content.contains("Implementing feature"));
        assert!(!result.content.contains("Implement feature"));
    }

    #[tokio::test]
    async fn test_todo_multiple_statuses() {
        let tool = TodoTool::new();

        let result = tool
            .execute(serde_json::json!({
                "todos": [
                    {
                        "content": "Write tests",
                        "status": "completed",
                        "active_form": "Writing tests"
                    },
                    {
                        "content": "Implement code",
                        "status": "in_progress",
                        "active_form": "Implementing code"
                    },
                    {
                        "content": "Write docs",
                        "status": "pending",
                        "active_form": "Writing docs"
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("✓"));
        assert!(result.content.contains("Write tests"));
        assert!(result.content.contains("⏳"));
        assert!(result.content.contains("Implementing code"));
        assert!(result.content.contains("☐"));
        assert!(result.content.contains("Write docs"));
    }

    #[tokio::test]
    async fn test_todo_replaces_existing() {
        let tool = TodoTool::new();

        // First set of todos
        tool.execute(serde_json::json!({
            "todos": [
                {
                    "content": "Old task",
                    "status": "pending",
                    "active_form": "Old task"
                }
            ]
        }))
        .await
        .unwrap();

        // Replace with new todos
        let result = tool
            .execute(serde_json::json!({
                "todos": [
                    {
                        "content": "New task",
                        "status": "pending",
                        "active_form": "New task"
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("New task"));
        assert!(!result.content.contains("Old task"));
    }
}
