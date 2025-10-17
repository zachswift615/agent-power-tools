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
}

// Default value functions
fn default_api_base() -> String {
    "http://localhost:1234/v1".to_string()
}

fn default_model() -> String {
    "qwen2.5-coder-7b-instruct".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> Option<u32> {
    Some(4096)
}

fn default_bash_timeout() -> u64 {
    120
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

fn default_streaming() -> bool {
    true
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
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LLMConfig::default(),
            timeouts: TimeoutConfig::default(),
            ui: UIConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from file or use defaults
    ///
    /// Checks the following locations in order:
    /// 1. ~/.config/synthia/config.toml
    /// 2. ./synthia.toml (current directory)
    ///
    /// If no config file is found, returns default configuration
    pub fn load() -> Result<Self> {
        let config_paths = Self::get_config_paths();

        for path in &config_paths {
            if path.exists() {
                tracing::info!("Loading config from: {}", path.display());
                let contents = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

                let config: Config = toml::from_str(&contents)
                    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

                return Ok(config);
            }
        }

        tracing::info!("No config file found, using defaults");
        Ok(Config::default())
    }

    /// Get the list of config file paths to check (in order)
    fn get_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. ~/.config/synthia/config.toml
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".config").join("synthia").join("config.toml"));
        }

        // 2. ./synthia.toml (current directory)
        paths.push(PathBuf::from("synthia.toml"));

        paths
    }

    /// Create a default config file at the specified path
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
    pub fn user_config_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".config").join("synthia"))
    }

    /// Get the user config file path (~/.config/synthia/config.toml)
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
        assert_eq!(config.llm.model, "qwen2.5-coder-7b-instruct");
        assert_eq!(config.llm.temperature, 0.7);
        assert_eq!(config.llm.max_tokens, Some(4096));
        assert_eq!(config.timeouts.bash_timeout, 120);
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

        assert_eq!(config.llm.model, "qwen2.5-coder-7b-instruct");
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
        assert_eq!(config.llm.model, "qwen2.5-coder-7b-instruct");
    }
}

// Need to add dirs crate for home directory detection
