use crate::permission_config::PermissionConfig;
use anyhow::Result;
use glob::Pattern;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

pub struct PermissionManager {
    config: PermissionConfig,
    config_path: PathBuf,
    project_root: PathBuf,
}

impl PermissionManager {
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let config_path = project_root.join(".synthia/settings-local.json");
        let config = PermissionConfig::load(&config_path)?;

        Ok(Self {
            config,
            config_path,
            project_root,
        })
    }

    /// Check if an operation is permitted
    pub fn check_permission(&self, tool: &str, params: &Value) -> PermissionDecision {
        let pattern = self.build_pattern(tool, params);

        // Check deny list first (highest priority)
        if self.matches_any(&pattern, &self.config.permissions.deny) {
            return PermissionDecision::Deny;
        }

        // Check allow list
        if self.matches_any(&pattern, &self.config.permissions.allow) {
            return PermissionDecision::Allow;
        }

        // Default to ask
        PermissionDecision::Ask
    }

    /// Add a permission pattern and save config
    pub fn add_permission(&mut self, pattern: String) -> Result<()> {
        self.config.add_permission(pattern)?;
        self.config.save(&self.config_path)?;
        Ok(())
    }

    /// Build a permission pattern from tool and params
    pub fn build_pattern(&self, tool: &str, params: &Value) -> String {
        match tool {
            "bash" => {
                if let Some(command) = params["command"].as_str() {
                    let cmd_name = command.split_whitespace().next().unwrap_or(command);
                    format!("Bash({}:*)", cmd_name)
                } else {
                    "Bash(unknown:*)".to_string()
                }
            }
            "read" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    let abs_path = self.normalize_path(file_path);
                    // Prepend // only if path doesn't already start with /
                    let path_with_prefix = if abs_path.starts_with('/') {
                        format!("/{}", abs_path) // Add one / to make //
                    } else {
                        format!("//{}", abs_path)
                    };
                    format!("Read({})", path_with_prefix)
                } else {
                    "Read(unknown)".to_string()
                }
            }
            "write" | "edit" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    let abs_path = self.normalize_path(file_path);
                    let tool_name = match tool {
                        "write" => "Write",
                        "edit" => "Edit",
                        _ => tool,
                    };
                    // Prepend // only if path doesn't already start with /
                    let path_with_prefix = if abs_path.starts_with('/') {
                        format!("/{}", abs_path) // Add one / to make //
                    } else {
                        format!("//{}", abs_path)
                    };
                    format!("{}({})", tool_name, path_with_prefix)
                } else {
                    format!("{}(unknown)", tool)
                }
            }
            "git" => {
                // Extract git subcommand from params
                if let Some(command) = params["command"].as_str() {
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    if parts.len() > 1 && parts[0] == "git" {
                        format!("Git({}:*)", parts[1])
                    } else {
                        "Git(unknown:*)".to_string()
                    }
                } else {
                    "Git(unknown:*)".to_string()
                }
            }
            "webfetch" => {
                if let Some(url) = params["url"].as_str() {
                    if let Ok(parsed_url) = url::Url::parse(url) {
                        if let Some(domain) = parsed_url.host_str() {
                            return format!("WebFetch(domain:{})", domain);
                        }
                    }
                }
                "WebFetch(unknown)".to_string()
            }
            other => {
                // MCP tools or other tools
                other.to_string()
            }
        }
    }

    /// Check if operation pattern matches any permission pattern
    fn matches_any(&self, operation: &str, patterns: &[String]) -> bool {
        patterns.iter().any(|p| self.matches(operation, p))
    }

    /// Check if operation matches a permission pattern
    fn matches(&self, operation: &str, pattern: &str) -> bool {
        // Exact match
        if operation == pattern {
            return true;
        }

        // Extract tool and pattern from permission string
        if let Some((perm_tool, perm_pattern)) = self.parse_permission(pattern) {
            if let Some((op_tool, op_value)) = self.parse_permission(operation) {
                if perm_tool != op_tool {
                    return false;
                }

                // Check pattern match
                if perm_pattern == "*" {
                    return true;
                }

                // Handle prefix:* patterns (e.g., "cargo:*")
                if perm_pattern.ends_with(":*") {
                    let prefix = &perm_pattern[..perm_pattern.len() - 2]; // Remove ":*"
                    return op_value.starts_with(prefix);
                }

                // Glob pattern matching for paths
                if perm_pattern.contains('*') {
                    if let Ok(glob_pattern) = Pattern::new(&perm_pattern) {
                        return glob_pattern.matches(&op_value);
                    }
                }

                // Prefix match for exact patterns
                return op_value.starts_with(&perm_pattern);
            }
        }

        false
    }

    /// Parse permission string into (tool, pattern)
    fn parse_permission(&self, perm: &str) -> Option<(String, String)> {
        if let Some(idx) = perm.find('(') {
            let tool = perm[..idx].to_string();
            let pattern = perm[idx + 1..]
                .trim_end_matches(')')
                .to_string();
            Some((tool, pattern))
        } else {
            // MCP tools or simple patterns
            Some((perm.to_string(), String::new()))
        }
    }

    /// Normalize path to absolute
    fn normalize_path(&self, path: &str) -> String {
        let path_buf = PathBuf::from(path);

        if path_buf.is_absolute() {
            path.to_string()
        } else {
            self.project_root
                .join(path)
                .to_string_lossy()
                .to_string()
        }
    }

    /// Generate suggested pattern for "don't ask again" based on tool and params
    pub fn suggest_pattern(&self, tool: &str, params: &Value) -> String {
        match tool {
            "bash" => {
                if let Some(command) = params["command"].as_str() {
                    let cmd_name = command.split_whitespace().next().unwrap_or(command);
                    format!("don't ask again for '{}' commands", cmd_name)
                } else {
                    "don't ask again for this command".to_string()
                }
            }
            "read" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    let abs_path = self.normalize_path(file_path);
                    let dir = Path::new(&abs_path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| abs_path);
                    format!("don't ask again for reads in {}/**", dir)
                } else {
                    "don't ask again for reads".to_string()
                }
            }
            "write" | "edit" => {
                if let Some(file_path) = params["file_path"].as_str() {
                    format!("don't ask again for edits to {}", file_path)
                } else {
                    "don't ask again for edits".to_string()
                }
            }
            _ => "don't ask again for this operation".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn create_test_manager() -> PermissionManager {
        let temp_dir = env::temp_dir();
        // Use a unique directory for each test to avoid conflicts
        let project_root = temp_dir.join(format!("test_project_{}", rand::random::<u64>()));
        fs::create_dir_all(&project_root).unwrap();

        PermissionManager::new(project_root).unwrap()
    }

    #[test]
    fn test_new_manager_loads_empty_config() {
        let manager = create_test_manager();
        assert!(manager.config.permissions.allow.is_empty());
    }

    #[test]
    fn test_bash_pattern_matching() {
        let mut manager = create_test_manager();
        manager.add_permission("Bash(cargo:*)".to_string()).unwrap();

        let params = serde_json::json!({
            "command": "cargo test"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_bash_different_command_not_matched() {
        let mut manager = create_test_manager();
        manager.add_permission("Bash(cargo:*)".to_string()).unwrap();

        let params = serde_json::json!({
            "command": "npm install"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn test_read_glob_pattern_matching() {
        let mut manager = create_test_manager();
        manager.add_permission("Read(//Users/test/**)".to_string()).unwrap();

        let params = serde_json::json!({
            "file_path": "/Users/test/foo/bar.txt"
        });

        assert_eq!(
            manager.check_permission("read", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_write_exact_file_matching() {
        let mut manager = create_test_manager();
        manager.add_permission("Write(//Users/test/file.rs)".to_string()).unwrap();

        let params = serde_json::json!({
            "file_path": "/Users/test/file.rs"
        });

        assert_eq!(
            manager.check_permission("write", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_deny_overrides_allow() {
        let mut manager = create_test_manager();
        manager.config.permissions.allow.push("Bash(cargo:*)".to_string());
        manager.config.permissions.deny.push("Bash(cargo:*)".to_string());

        let params = serde_json::json!({
            "command": "cargo test"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Deny
        );
    }

    #[test]
    fn test_default_is_ask() {
        let manager = create_test_manager();

        let params = serde_json::json!({
            "command": "cargo test"
        });

        assert_eq!(
            manager.check_permission("bash", &params),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn test_mcp_tool_exact_match() {
        let mut manager = create_test_manager();
        manager.add_permission("mcp__powertools__index_project".to_string()).unwrap();

        let params = serde_json::json!({});

        assert_eq!(
            manager.check_permission("mcp__powertools__index_project", &params),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_suggest_pattern_bash() {
        let manager = create_test_manager();
        let params = serde_json::json!({
            "command": "cargo test --all"
        });

        let suggestion = manager.suggest_pattern("bash", &params);
        assert!(suggestion.contains("cargo"));
    }

    #[test]
    fn test_add_permission_saves_config() {
        let temp_dir = env::temp_dir();
        let project_root = temp_dir.join("test_add_permission");
        fs::create_dir_all(&project_root).unwrap();

        let mut manager = PermissionManager::new(project_root.clone()).unwrap();
        manager.add_permission("Bash(cargo:*)".to_string()).unwrap();

        // Load fresh manager and verify persistence
        let manager2 = PermissionManager::new(project_root.clone()).unwrap();
        assert_eq!(manager2.config.permissions.allow.len(), 1);

        // Clean up
        fs::remove_dir_all(&project_root).unwrap();
    }
}
