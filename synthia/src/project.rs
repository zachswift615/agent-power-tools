// synthia/src/project.rs
use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Detects the project root by looking for .git directory
/// Falls back to current directory if not a git repo
pub fn detect_project_root() -> Result<PathBuf> {
    // Try git root first
    if let Ok(git_root) = detect_git_root() {
        return Ok(git_root);
    }

    // Fall back to current directory
    env::current_dir().context("Failed to get current directory")
}

/// Detects git repository root
fn detect_git_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to run git command")?;

    if !output.status.success() {
        anyhow::bail!("Not a git repository");
    }

    let path_str = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git output")?
        .trim()
        .to_string();

    Ok(PathBuf::from(path_str))
}

/// Extracts project name from path (last component)
pub fn extract_project_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("Failed to extract project name from path")
}

/// Normalizes project name for filesystem safety
/// - Converts to lowercase
/// - Replaces spaces and special chars with underscore
/// - Truncates to 50 characters
/// - Ensures ASCII-safe
pub fn normalize_project_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .chars()
        .take(50)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_project_name() {
        assert_eq!(normalize_project_name("My Project"), "my_project");
        assert_eq!(normalize_project_name("Agent-Powertools"), "agent-powertools");
        assert_eq!(normalize_project_name("Project@#$%123"), "project____123");
        assert_eq!(normalize_project_name("hello world! 2024"), "hello_world__2024");

        // Test truncation
        let long_name = "a".repeat(100);
        assert_eq!(normalize_project_name(&long_name).len(), 50);
    }

    #[test]
    fn test_extract_project_name() {
        let path = PathBuf::from("/home/user/projects/my-project");
        assert_eq!(extract_project_name(&path).unwrap(), "my-project");

        let path = PathBuf::from("/home/user/projects/Agent Powertools");
        assert_eq!(extract_project_name(&path).unwrap(), "Agent Powertools");
    }
}
