use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;
use crate::indexers::{UnifiedQuery, LspQuery};

/// Swift language support via sourcekit-lsp
///
/// This module provides semantic navigation for Swift code using Apple's
/// sourcekit-lsp server. It wraps the generic LSP infrastructure with
/// Swift-specific conveniences.
///
/// ## Requirements
/// - sourcekit-lsp must be installed and available in PATH
/// - Typically comes with Xcode or Swift toolchain
/// - macOS: Install Xcode Command Line Tools
/// - Linux: Install Swift from https://swift.org/download/
///
/// ## Features
/// - Go to definition for Swift symbols
/// - Find references to Swift symbols (requires position)
/// - Supports Swift 5.x and later
pub struct SwiftLsp;

impl SwiftLsp {
    /// Check if sourcekit-lsp is available on the system
    ///
    /// # Returns
    /// - Ok(path) if sourcekit-lsp is found in PATH
    /// - Err if sourcekit-lsp is not available
    pub fn check_available() -> Result<PathBuf> {
        // Try to find sourcekit-lsp in common locations
        let possible_paths = vec![
            "sourcekit-lsp",  // In PATH
            "/usr/bin/sourcekit-lsp",
            "/usr/local/bin/sourcekit-lsp",
            "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/sourcekit-lsp",
        ];

        for path_str in &possible_paths {
            let result = Command::new("which")
                .arg(path_str)
                .output();

            if let Ok(output) = result {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .to_string();
                    if !path.is_empty() {
                        return Ok(PathBuf::from(path));
                    }
                }
            }

            // Also try direct execution test
            let result = Command::new(path_str)
                .arg("--version")
                .output();

            if let Ok(output) = result {
                if output.status.success() {
                    return Ok(PathBuf::from(*path_str));
                }
            }
        }

        Err(anyhow::anyhow!(
            "sourcekit-lsp not found. Please install Xcode Command Line Tools (macOS) or Swift toolchain."
        ))
    }

    /// Create a UnifiedQuery instance for Swift using sourcekit-lsp
    ///
    /// # Arguments
    /// * `project_root` - Root directory of the Swift project
    ///
    /// # Returns
    /// A UnifiedQuery instance backed by sourcekit-lsp
    ///
    /// # Examples
    /// ```ignore
    /// let query = SwiftLsp::create_query(PathBuf::from("/path/to/swift/project"))?;
    /// let definition = query.find_definition(&file_path, line, column)?;
    /// ```
    pub fn create_query(project_root: PathBuf) -> Result<UnifiedQuery> {
        // Verify sourcekit-lsp is available
        let lsp_path = Self::check_available()
            .context("sourcekit-lsp is required for Swift navigation")?;

        // Create LSP query with sourcekit-lsp
        // sourcekit-lsp doesn't need special arguments, just the root URI
        let lsp_command = lsp_path.to_string_lossy().to_string();
        let lsp_args = vec![]; // sourcekit-lsp is started without args

        UnifiedQuery::lsp_only(&lsp_command, lsp_args, project_root)
            .context("Failed to start sourcekit-lsp server")
    }

    /// Start an LspQuery instance directly for Swift
    ///
    /// # Arguments
    /// * `project_root` - Root directory of the Swift project
    ///
    /// # Returns
    /// An LspQuery instance connected to sourcekit-lsp
    pub fn start(project_root: PathBuf) -> Result<LspQuery> {
        // Verify sourcekit-lsp is available
        let lsp_path = Self::check_available()
            .context("sourcekit-lsp is required for Swift navigation")?;

        // Create LSP query with sourcekit-lsp
        let lsp_command = lsp_path.to_string_lossy().to_string();
        let lsp_args = vec![]; // sourcekit-lsp is started without args

        LspQuery::start(&lsp_command, lsp_args, project_root)
            .context("Failed to start sourcekit-lsp server")
    }

    /// Get the version of sourcekit-lsp
    ///
    /// # Returns
    /// Version string if available, error otherwise
    #[allow(dead_code)]
    pub fn version() -> Result<String> {
        let lsp_path = Self::check_available()?;

        let output = Command::new(lsp_path)
            .arg("--version")
            .output()
            .context("Failed to execute sourcekit-lsp --version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("sourcekit-lsp --version failed"));
        }

        let version = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in version output")?;

        Ok(version.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Only runs if sourcekit-lsp is installed
    fn test_sourcekit_lsp_available() {
        let result = SwiftLsp::check_available();
        assert!(result.is_ok(), "sourcekit-lsp should be available");
    }

    #[test]
    #[ignore] // Only runs if sourcekit-lsp is installed
    fn test_get_version() {
        let version = SwiftLsp::version();
        assert!(version.is_ok(), "Should get version");
        println!("sourcekit-lsp version: {}", version.unwrap());
    }

    #[test]
    #[ignore] // Requires sourcekit-lsp and a Swift project
    fn test_create_query() {
        // This would test creating a query for a real Swift project
        // Requires a test Swift project with .swift files
    }
}
