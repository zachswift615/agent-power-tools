use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{self, Write};
use scip::types::Index;

/// SCIP indexer that delegates to language-specific indexers
pub struct ScipIndexer {
    project_root: PathBuf,
    auto_install: bool,
}

impl ScipIndexer {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            auto_install: false,
        }
    }

    pub fn set_auto_install(&mut self, auto_install: bool) {
        self.auto_install = auto_install;
    }

    /// Generate SCIP index by detecting project type and calling appropriate indexer
    pub fn generate_index(&self) -> Result<PathBuf> {
        let project_type = self.detect_project_type()?;

        match project_type {
            ProjectType::TypeScript => self.index_typescript(),
            ProjectType::JavaScript => self.index_javascript(),
            ProjectType::Python => self.index_python(),
            ProjectType::Rust => self.index_rust(),
        }
    }

    /// Read existing SCIP index from disk
    pub fn read_index(&self) -> Result<Index> {
        let index_path = self.get_index_path();

        if !index_path.exists() {
            return Err(anyhow!(
                "SCIP index not found at {}. Run 'powertools index' first",
                index_path.display()
            ));
        }

        // Read the protobuf file
        use protobuf::Message;
        let bytes = std::fs::read(&index_path)
            .context("Failed to read SCIP index file")?;

        let index = Index::parse_from_bytes(&bytes)
            .context("Failed to parse SCIP index")?;

        Ok(index)
    }

    fn detect_project_type(&self) -> Result<ProjectType> {
        // Check for TypeScript/JavaScript
        if self.project_root.join("package.json").exists() {
            if self.project_root.join("tsconfig.json").exists() {
                return Ok(ProjectType::TypeScript);
            }
            return Ok(ProjectType::JavaScript);
        }

        // Check for Python
        if self.project_root.join("requirements.txt").exists()
            || self.project_root.join("setup.py").exists()
            || self.project_root.join("pyproject.toml").exists()
        {
            return Ok(ProjectType::Python);
        }

        // Check for Rust
        if self.project_root.join("Cargo.toml").exists() {
            return Ok(ProjectType::Rust);
        }

        Err(anyhow!(
            "Could not detect project type. Supported: TypeScript, JavaScript, Python, Rust"
        ))
    }

    fn index_typescript(&self) -> Result<PathBuf> {
        println!("Indexing TypeScript project...");

        // Check if scip-typescript is available
        if !self.check_indexer_installed("npx", &["@sourcegraph/scip-typescript", "--help"]) {
            println!("\n⚠️  scip-typescript is not installed.");

            let should_install = if self.auto_install {
                println!("Auto-installing scip-typescript...");
                true
            } else {
                println!("Would you like to install it? (y/N)");
                println!("Command: npm install -g @sourcegraph/scip-typescript");
                print!("> ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_lowercase() == "y"
            };

            if should_install {
                println!("Installing scip-typescript...");
                let status = Command::new("npm")
                    .args(&["install", "-g", "@sourcegraph/scip-typescript"])
                    .status()
                    .context("Failed to install scip-typescript")?;

                if !status.success() {
                    return Err(anyhow!("Installation failed"));
                }
                println!("✓ scip-typescript installed successfully!");
            } else {
                return Err(anyhow!("scip-typescript is required for TypeScript indexing"));
            }
        }

        // Run scip-typescript indexer
        let status = Command::new("npx")
            .args(&["@sourcegraph/scip-typescript", "index"])
            .current_dir(&self.project_root)
            .status()
            .context("Failed to run scip-typescript")?;

        if !status.success() {
            return Err(anyhow!("scip-typescript indexing failed"));
        }

        Ok(self.get_index_path())
    }

    fn index_javascript(&self) -> Result<PathBuf> {
        // JavaScript uses the same indexer as TypeScript
        self.index_typescript()
    }

    fn index_python(&self) -> Result<PathBuf> {
        println!("Indexing Python project...");

        // Check if scip-python is available
        if !self.check_indexer_installed("scip-python", &["--help"]) {
            println!("\n⚠️  scip-python is not installed.");

            let should_install = if self.auto_install {
                println!("Auto-installing scip-python...");
                true
            } else {
                println!("Would you like to install it? (y/N)");
                println!("Command: pip install scip-python");
                print!("> ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_lowercase() == "y"
            };

            if should_install {
                println!("Installing scip-python...");
                let status = Command::new("pip")
                    .args(&["install", "scip-python"])
                    .status()
                    .context("Failed to install scip-python")?;

                if !status.success() {
                    return Err(anyhow!("Installation failed"));
                }
                println!("✓ scip-python installed successfully!");
            } else {
                return Err(anyhow!("scip-python is required for Python indexing"));
            }
        }

        // Run scip-python indexer
        let status = Command::new("scip-python")
            .arg("index")
            .arg(".")
            .current_dir(&self.project_root)
            .status()
            .context("Failed to run scip-python")?;

        if !status.success() {
            return Err(anyhow!("scip-python indexing failed"));
        }

        Ok(self.get_index_path())
    }

    fn index_rust(&self) -> Result<PathBuf> {
        println!("Indexing Rust project...");

        // Check if rust-analyzer is available
        if !self.check_indexer_installed("rust-analyzer", &["--version"]) {
            println!("\n⚠️  rust-analyzer is not installed.");

            let should_install = if self.auto_install {
                println!("Auto-installing rust-analyzer...");
                true
            } else {
                println!("Would you like to install it? (y/N)");
                println!("Command: rustup component add rust-analyzer");
                print!("> ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_lowercase() == "y"
            };

            if should_install {
                println!("Installing rust-analyzer...");
                let status = Command::new("rustup")
                    .args(&["component", "add", "rust-analyzer"])
                    .status()
                    .context("Failed to install rust-analyzer")?;

                if !status.success() {
                    return Err(anyhow!("Installation failed"));
                }
                println!("✓ rust-analyzer installed successfully!");
            } else {
                return Err(anyhow!("rust-analyzer is required for Rust indexing"));
            }
        }

        // Run rust-analyzer SCIP indexer
        let status = Command::new("rust-analyzer")
            .args(&["scip", "."])
            .current_dir(&self.project_root)
            .status()
            .context("Failed to run rust-analyzer scip")?;

        if !status.success() {
            return Err(anyhow!("rust-analyzer SCIP indexing failed"));
        }

        Ok(self.get_index_path())
    }

    fn check_indexer_installed(&self, command: &str, args: &[&str]) -> bool {
        Command::new(command)
            .args(args)
            .output()
            .is_ok()
    }

    fn get_index_path(&self) -> PathBuf {
        self.project_root.join("index.scip")
    }
}

#[derive(Debug)]
enum ProjectType {
    TypeScript,
    JavaScript,
    Python,
    Rust,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_project_type() {
        // Test would require setting up temporary project files
    }
}