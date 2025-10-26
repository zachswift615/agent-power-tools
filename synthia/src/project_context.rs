use std::fs;
use std::path::{Path, PathBuf};
use tracing::{warn, info};

pub struct ProjectContext {
    pub custom_instructions: Option<String>,
    pub synthia_dir: PathBuf,
}

impl ProjectContext {
    /// Load project context from .synthia/ directory
    /// Non-fatal: Returns empty context if any errors occur
    pub fn load() -> Self {
        match Self::load_impl() {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!("Failed to load project context: {}. Continuing without project-specific instructions.", e);
                Self {
                    custom_instructions: None,
                    synthia_dir: PathBuf::from(".synthia"),
                }
            }
        }
    }

    fn load_impl() -> Result<Self, Box<dyn std::error::Error>> {
        let cwd = std::env::current_dir()?;
        let synthia_dir = cwd.join(".synthia");

        // Ensure .synthia/ exists
        if !synthia_dir.exists() {
            fs::create_dir_all(&synthia_dir)?;
            info!("Created .synthia directory at {:?}", synthia_dir);
        }

        // Ensure .SYNTHIA.md exists
        let synthia_md = synthia_dir.join(".SYNTHIA.md");
        if !synthia_md.exists() {
            fs::write(&synthia_md, "")?;
            info!("Created empty .SYNTHIA.md at {:?}", synthia_md);
        }

        // Read custom instructions
        let custom_instructions = Self::load_custom_instructions(&synthia_md)?;

        Ok(Self {
            custom_instructions,
            synthia_dir,
        })
    }

    fn load_custom_instructions(path: &Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let trimmed = content.trim();

        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }
}
