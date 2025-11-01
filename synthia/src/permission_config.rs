use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub permissions: Permissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub ask: Vec<String>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            permissions: Permissions {
                allow: Vec::new(),
                deny: Vec::new(),
                ask: Vec::new(),
            },
        }
    }
}

impl PermissionConfig {
    /// Load permission config from file, or return default if file doesn't exist
    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(config_path)?;
        let config: PermissionConfig = serde_json::from_str(&contents)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse permission config: {}, using defaults", e);
                Self::default()
            });

        Ok(config)
    }

    /// Save permission config to file atomically
    pub fn save(&self, config_path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;

        // Write atomically: temp file + rename
        let temp_path = config_path.with_extension("tmp");
        fs::write(&temp_path, json)?;
        fs::rename(temp_path, config_path)?;

        Ok(())
    }

    /// Add a permission pattern to the allow list
    pub fn add_permission(&mut self, pattern: String) -> Result<()> {
        // Avoid duplicates
        if !self.permissions.allow.contains(&pattern) {
            self.permissions.allow.push(pattern);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = PermissionConfig::default();
        assert!(config.permissions.allow.is_empty());
        assert!(config.permissions.deny.is_empty());
        assert!(config.permissions.ask.is_empty());
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = PathBuf::from("/nonexistent/path/settings.json");
        let config = PermissionConfig::load(&path).unwrap();
        assert!(config.permissions.allow.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = env::temp_dir();
        let config_path = temp_dir.join("test_permissions.json");

        // Clean up from previous runs
        let _ = fs::remove_file(&config_path);

        let mut config = PermissionConfig::default();
        config.add_permission("Bash(cargo:*)".to_string()).unwrap();
        config.add_permission("Read(//Users/test/**)".to_string()).unwrap();

        config.save(&config_path).unwrap();

        let loaded = PermissionConfig::load(&config_path).unwrap();
        assert_eq!(loaded.permissions.allow.len(), 2);
        assert!(loaded.permissions.allow.contains(&"Bash(cargo:*)".to_string()));

        // Clean up
        fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_add_permission_avoids_duplicates() {
        let mut config = PermissionConfig::default();
        config.add_permission("Bash(cargo:*)".to_string()).unwrap();
        config.add_permission("Bash(cargo:*)".to_string()).unwrap();
        assert_eq!(config.permissions.allow.len(), 1);
    }

    #[test]
    fn test_load_corrupted_json_returns_default() {
        let temp_dir = env::temp_dir();
        let config_path = temp_dir.join("corrupted_permissions.json");

        // Write invalid JSON
        fs::write(&config_path, "{ invalid json }").unwrap();

        let config = PermissionConfig::load(&config_path).unwrap();
        assert!(config.permissions.allow.is_empty());

        // Clean up
        fs::remove_file(&config_path).unwrap();
    }
}
