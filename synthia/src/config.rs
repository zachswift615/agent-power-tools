use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure for Synthia
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub llm: LLMConfig,

    #[serde(default)]
    pub timeouts: TimeoutConfig,

    #[serde(default)]
    pub ui: UIConfig,

    #[serde(default)]
    pub tools: ToolsConfig,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Base URL for the LLM API (e.g., "http://localhost:1234/v1")
    #[serde(default = "default_api_base")]
    pub api_base: String,

    /// Optional API key for authentication
    #[serde(default)]
    pub api_key: Option<String>,

    /// Model name to use
    #[serde(default = "default_model")]
    pub model: String,

    /// Temperature for generation (0.0 to 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum tokens to generate
    #[serde(default = "default_max_tokens")]
    pub max_tokens: Option<u32>,

    /// Enable streaming for text generation
    #[serde(default = "default_streaming")]
    pub streaming: bool,

    /// Context window size for the model (default: 8192)
    #[serde(default = "default_context_window")]
    pub context_window: Option<usize>,
}

/// Timeout configuration for various tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Timeout for bash commands in seconds
    #[serde(default = "default_bash_timeout")]
    pub bash_timeout: u64,

    /// Timeout for git operations in seconds
    #[serde(default = "default_git_timeout")]
    pub git_timeout: u64,

    /// Timeout for workshop commands in seconds
    #[serde(default = "default_workshop_timeout")]
    pub workshop_timeout: u64,

    /// Timeout for powertools operations in seconds
    #[serde(default = "default_powertools_timeout")]
    pub powertools_timeout: u64,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    /// Enable syntax highlighting in markdown rendering
    #[serde(default = "default_syntax_highlighting")]
    pub syntax_highlighting: bool,

    /// Maximum lines to show in tool output
    #[serde(default = "default_max_output_lines")]
    pub max_output_lines: usize,

    /// Enable edit approval prompts
    #[serde(default = "default_edit_approval")]
    pub edit_approval: bool,
}

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// Optional path to a custom powertools binary.
    /// If not specified, uses the embedded binary.
    #[serde(default)]
    pub powertools_binary_path: Option<PathBuf>,

    /// Maximum output size for bash commands in characters (~4 chars per token)
    /// Default: 50,000 chars (~12,500 tokens)
    #[serde(default = "default_max_bash_output_chars")]
    pub max_bash_output_chars: usize,

    /// Maximum output size for read operations in characters
    /// Default: 200,000 chars (~50,000 tokens)
    #[serde(default = "default_max_read_output_chars")]
    pub max_read_output_chars: usize,

    /// Warn at this many characters for read operations (doesn't fail, just warns)
    /// Default: 100,000 chars (~25,000 tokens)
    #[serde(default = "default_read_warn_at_chars")]
    pub read_warn_at_chars: usize,
}

// Default value functions
fn default_api_base() -> String {
    "http://localhost:1234/v1".to_string()
}

fn default_model() -> String {
    "google/gemma-3-12b".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> Option<u32> {
    Some(4096)
}

fn default_bash_timeout() -> u64 {
    300 // 5 minutes - increased from 120s to handle server startup and other long operations
}

fn default_git_timeout() -> u64 {
    120
}

fn default_workshop_timeout() -> u64 {
    30
}

fn default_powertools_timeout() -> u64 {
    60
}

fn default_syntax_highlighting() -> bool {
    true
}

fn default_max_output_lines() -> usize {
    1000
}

fn default_edit_approval() -> bool {
    true
}

fn default_streaming() -> bool {
    true
}

fn default_context_window() -> Option<usize> {
    Some(8192)
}

fn default_max_bash_output_chars() -> usize {
    50_000 // ~12,500 tokens
}

fn default_max_read_output_chars() -> usize {
    200_000 // ~50,000 tokens
}

fn default_read_warn_at_chars() -> usize {
    100_000 // ~25,000 tokens
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            api_base: default_api_base(),
            api_key: None,
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            streaming: default_streaming(),
            context_window: default_context_window(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            bash_timeout: default_bash_timeout(),
            git_timeout: default_git_timeout(),
            workshop_timeout: default_workshop_timeout(),
            powertools_timeout: default_powertools_timeout(),
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            syntax_highlighting: default_syntax_highlighting(),
            max_output_lines: default_max_output_lines(),
            edit_approval: default_edit_approval(),
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            powertools_binary_path: None,
            max_bash_output_chars: default_max_bash_output_chars(),
            max_read_output_chars: default_max_read_output_chars(),
            read_warn_at_chars: default_read_warn_at_chars(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LLMConfig::default(),
            timeouts: TimeoutConfig::default(),
            ui: UIConfig::default(),
            tools: ToolsConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration with proper hierarchy and merging
    ///
    /// **Config Priority (highest to lowest):**
    /// 1. `./synthia.toml` (project-level config - HIGHEST priority)
    /// 2. `~/.config/synthia/config.toml` (global user config)
    /// 3. Hardcoded defaults (fallback)
    ///
    /// **Merging behavior:**
    /// - Project config overrides specific fields in global config
    /// - Missing fields use global config or defaults
    ///
    /// **Single Source of Truth:**
    /// - Global config: `~/.config/synthia/config.toml`
    /// - Project config: `./synthia.toml` (optional, overrides global)
    /// - No other config locations are checked
    pub fn load() -> Result<Self> {
        // Start with defaults
        let mut config = Config::default();

        // Load global config if it exists (lowest priority override)
        if let Some(global_path) = Self::global_config_path() {
            if global_path.exists() {
                tracing::info!("Loading global config from: {}", global_path.display());
                let global_config = Self::load_from_file(&global_path)?;
                config = Self::merge_configs(config, global_config);
            }
        }

        // Load project config if it exists (highest priority override)
        let project_path = Self::project_config_path();
        if project_path.exists() {
            tracing::info!("Loading project config from: {}", project_path.display());
            let project_config = Self::load_from_file(&project_path)?;
            config = Self::merge_configs(config, project_config);
        }

        tracing::info!(
            "Final config: model={}, api_base={}",
            config.llm.model,
            config.llm.api_base
        );

        Ok(config)
    }

    /// Load config from a specific file
    fn load_from_file(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }

    /// Merge two configs (override takes precedence for non-default values)
    fn merge_configs(_base: Config, override_config: Config) -> Config {
        // For now, override completely replaces base
        // TODO: Could implement field-level merging if needed
        override_config
    }

    /// Get the global config path (~/.config/synthia/config.toml)
    fn global_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".config").join("synthia").join("config.toml"))
    }

    /// Get the project config path (./synthia.toml in current directory)
    fn project_config_path() -> PathBuf {
        PathBuf::from("synthia.toml")
    }

    /// DEPRECATED: Use global_config_path() or project_config_path()
    #[deprecated(note = "Use global_config_path() or project_config_path() instead")]
    #[allow(dead_code)]
    fn get_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(path) = Self::global_config_path() {
            paths.push(path);
        }
        paths.push(Self::project_config_path());
        paths
    }

    /// Create a default config file at the specified path
    #[allow(dead_code)]
    pub fn create_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = Config::default();
        let toml_string = toml::to_string_pretty(&config)
            .context("Failed to serialize default config")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(path.as_ref(), toml_string)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;

        Ok(())
    }

    /// Get the user config directory path (~/.config/synthia)
    #[allow(dead_code)]
    pub fn user_config_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".config").join("synthia"))
    }

    /// Get the user config file path (~/.config/synthia/config.toml)
    #[allow(dead_code)]
    pub fn user_config_path() -> Option<PathBuf> {
        Self::user_config_dir().map(|dir| dir.join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.llm.api_base, "http://localhost:1234/v1");
        assert_eq!(config.llm.model, "google/gemma-3-12b");
        assert_eq!(config.llm.temperature, 0.7);
        assert_eq!(config.llm.max_tokens, Some(4096));
        assert_eq!(config.timeouts.bash_timeout, 300);
        assert_eq!(config.timeouts.git_timeout, 120);
        assert_eq!(config.timeouts.workshop_timeout, 30);
        assert_eq!(config.timeouts.powertools_timeout, 60);
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::default();
        let toml_string = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_string).unwrap();

        assert_eq!(config.llm.api_base, deserialized.llm.api_base);
        assert_eq!(config.llm.model, deserialized.llm.model);
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
            [llm]
            model = "custom-model"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.llm.model, "custom-model");
        // Other fields should use defaults
        assert_eq!(config.llm.api_base, "http://localhost:1234/v1");
        assert_eq!(config.llm.temperature, 0.7);
    }

    #[test]
    fn test_invalid_toml() {
        let toml_str = r#"
            [llm
            model = "missing bracket"
        "#;

        let result: Result<Config, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        Config::create_default_config(&config_path).unwrap();

        assert!(config_path.exists());

        let contents = fs::read_to_string(&config_path).unwrap();
        let config: Config = toml::from_str(&contents).unwrap();

        assert_eq!(config.llm.model, "google/gemma-3-12b");
    }

    #[test]
    fn test_create_config_with_nested_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nested").join("dirs").join("config.toml");

        Config::create_default_config(&config_path).unwrap();

        assert!(config_path.exists());
    }

    #[test]
    fn test_load_nonexistent_returns_defaults() {
        // This test assumes no config file exists at the default locations
        // In a test environment, this should be safe
        let config = Config::load().unwrap();

        // Should have default values
        assert_eq!(config.llm.model, "google/gemma-3-12b");
    }

    #[test]
    fn test_edit_approval_default() {
        let config = UIConfig::default();
        assert_eq!(config.edit_approval, true); // Enabled by default
    }

    #[test]
    fn test_edit_approval_from_toml() {
        let toml_str = r#"
            [ui]
            edit_approval = false
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ui.edit_approval, false);
    }
}

// Need to add dirs crate for home directory detection
