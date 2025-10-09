use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

use super::filters::is_relevant_file;

/// Metadata stored alongside SCIP index files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// When the index was created
    pub created_at: SystemTime,

    /// Hash of all source file paths and their modification times
    pub files_hash: u64,

    /// Number of source files indexed
    pub file_count: usize,

    /// SCIP indexer version (if available)
    pub indexer_version: Option<String>,
}

impl IndexMetadata {
    /// Generate metadata for a project
    pub fn generate(project_root: &Path) -> Result<Self> {
        let mut hasher = DefaultHasher::new();
        let mut file_count = 0;

        // Walk directory and hash file paths + mtimes
        for entry in WalkDir::new(project_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !super::filters::should_ignore(e.path()))
        {
            let entry = entry?;
            if entry.file_type().is_file() && is_relevant_file(entry.path()) {
                // Hash the file path
                entry.path().hash(&mut hasher);

                // Hash the modification time
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(mtime) = metadata.modified() {
                        format!("{:?}", mtime).hash(&mut hasher);
                    }
                }

                file_count += 1;
            }
        }

        Ok(Self {
            created_at: SystemTime::now(),
            files_hash: hasher.finish(),
            file_count,
            indexer_version: None,
        })
    }

    /// Save metadata to a file
    pub fn save(&self, index_path: &Path) -> Result<()> {
        let meta_path = Self::meta_path(index_path);
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize metadata")?;
        std::fs::write(&meta_path, json)
            .context("Failed to write metadata file")?;
        Ok(())
    }

    /// Load metadata from a file
    pub fn load(index_path: &Path) -> Result<Self> {
        let meta_path = Self::meta_path(index_path);
        let json = std::fs::read_to_string(&meta_path)
            .context("Failed to read metadata file")?;
        let metadata: IndexMetadata = serde_json::from_str(&json)
            .context("Failed to parse metadata")?;
        Ok(metadata)
    }

    /// Check if the index is stale compared to current project state
    pub fn is_stale(&self, project_root: &Path) -> Result<bool> {
        let current = Self::generate(project_root)?;
        Ok(self.files_hash != current.files_hash)
    }

    /// Get the metadata file path for an index
    fn meta_path(index_path: &Path) -> PathBuf {
        index_path.with_extension("scip.meta")
    }

    /// Check if metadata exists for an index
    pub fn exists(index_path: &Path) -> bool {
        Self::meta_path(index_path).exists()
    }
}

/// Check if any index exists and is stale
pub fn check_staleness(project_root: &Path) -> Result<Option<String>> {
    let index_files = [
        "index.typescript.scip",
        "index.javascript.scip",
        "index.python.scip",
        "index.rust.scip",
        "index.cpp.scip",
    ];

    for index_file in &index_files {
        let index_path = project_root.join(index_file);
        if index_path.exists() {
            if let Ok(metadata) = IndexMetadata::load(&index_path) {
                if metadata.is_stale(project_root)? {
                    return Ok(Some(index_file.to_string()));
                }
            } else {
                // No metadata or failed to load - consider stale
                return Ok(Some(index_file.to_string()));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_generate_metadata() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();

        // Create some test files
        fs::write(project_root.join("test.rs"), "fn main() {}").unwrap();
        fs::write(project_root.join("test.py"), "print('hello')").unwrap();

        let metadata = IndexMetadata::generate(project_root).unwrap();
        assert_eq!(metadata.file_count, 2);
        assert!(metadata.files_hash != 0);
    }

    #[test]
    fn test_save_load_metadata() {
        let temp = TempDir::new().unwrap();
        let index_path = temp.path().join("index.scip");

        let metadata = IndexMetadata {
            created_at: SystemTime::now(),
            files_hash: 12345,
            file_count: 10,
            indexer_version: Some("1.0.0".to_string()),
        };

        metadata.save(&index_path).unwrap();
        let loaded = IndexMetadata::load(&index_path).unwrap();

        assert_eq!(loaded.files_hash, 12345);
        assert_eq!(loaded.file_count, 10);
    }
}
