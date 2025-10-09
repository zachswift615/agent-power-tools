use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::preview::{PreviewChange, PreviewDiff};
use super::BatchResult;

/// Mode for performing replacements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ReplacementMode {
    /// Preview changes without applying
    Preview,
    /// Apply changes immediately
    Apply,
}

/// Batch file replacer using regex
pub struct BatchReplacer {
    /// Regex pattern to search for
    pattern: Regex,

    /// Replacement string (supports capture groups)
    replacement: String,

    /// File glob pattern (e.g., "*.rs", "**/*.ts")
    file_pattern: Option<String>,

    /// Root path to search from
    root_path: PathBuf,
}

impl BatchReplacer {
    /// Create a new batch replacer
    pub fn new(
        pattern: &str,
        replacement: String,
        file_pattern: Option<String>,
        root_path: PathBuf,
    ) -> Result<Self> {
        let regex = Regex::new(pattern)
            .with_context(|| format!("Invalid regex pattern: {}", pattern))?;

        Ok(Self {
            pattern: regex,
            replacement,
            file_pattern,
            root_path,
        })
    }

    /// Preview changes without applying them
    pub fn preview(&self) -> Result<Vec<PreviewDiff>> {
        let files = self.collect_files()?;
        let mut previews = Vec::new();

        for file_path in files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let diff = self.preview_file(&file_path, &content)?;
                if diff.num_changes > 0 {
                    previews.push(diff);
                }
            }
        }

        Ok(previews)
    }

    /// Apply replacements to all matching files
    pub fn apply(&self) -> Result<BatchResult> {
        let files = self.collect_files()?;
        let mut result = BatchResult::new();

        for file_path in files {
            result.files_scanned += 1;

            match self.apply_to_file(&file_path) {
                Ok(num_replacements) => {
                    if num_replacements > 0 {
                        result.add_modified_file(file_path, num_replacements);
                    }
                }
                Err(e) => {
                    result.add_error(format!("{}: {}", file_path.display(), e));
                }
            }
        }

        Ok(result)
    }

    /// Preview changes for a single file
    fn preview_file(&self, file_path: &Path, content: &str) -> Result<PreviewDiff> {
        let mut diff = PreviewDiff::new(file_path.to_path_buf());

        for (line_num, line) in content.lines().enumerate() {
            for mat in self.pattern.find_iter(line) {
                let original = mat.as_str().to_string();
                let replacement = self.pattern.replace(line, &self.replacement).to_string();

                diff.add_change(PreviewChange {
                    line: line_num + 1, // 1-indexed
                    column: mat.start() + 1, // 1-indexed
                    original,
                    replacement: replacement.clone(),
                    line_content: line.to_string(),
                });
            }
        }

        Ok(diff)
    }

    /// Apply replacements to a single file
    fn apply_to_file(&self, file_path: &Path) -> Result<usize> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let mut num_replacements = 0;
        let mut modified_content = String::new();

        for line in content.lines() {
            let replaced = self.pattern.replace_all(line, &self.replacement);
            if replaced != line {
                num_replacements += self.pattern.find_iter(line).count();
            }
            modified_content.push_str(&replaced);
            modified_content.push('\n');
        }

        // Only write if content changed
        if num_replacements > 0 {
            fs::write(file_path, modified_content.trim_end_matches('\n'))
                .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        }

        Ok(num_replacements)
    }

    /// Collect all files matching the pattern
    fn collect_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.root_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e.path()))
        {
            let entry = entry?;
            if entry.file_type().is_file() && self.matches_file_pattern(entry.path()) {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        let ignore_patterns = [
            ".git/",
            "target/",
            "node_modules/",
            ".scip",
            "dist/",
            "build/",
            ".next/",
            "__pycache__/",
            ".pytest_cache/",
            ".mypy_cache/",
            "venv/",
            ".venv/",
        ];

        for pattern in &ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Check if a file matches the file pattern
    fn matches_file_pattern(&self, path: &Path) -> bool {
        if let Some(ref pattern) = self.file_pattern {
            // Simple glob matching
            if let Some(file_name) = path.file_name() {
                let name = file_name.to_string_lossy();

                // Handle ** prefix (match any directory)
                if pattern.starts_with("**/") {
                    let suffix = &pattern[3..];
                    return self.simple_glob_match(&name, suffix);
                }

                // Handle * wildcards
                return self.simple_glob_match(&name, pattern);
            }
            false
        } else {
            // No pattern = match all files
            true
        }
    }

    /// Simple glob matching (supports * wildcard)
    fn simple_glob_match(&self, text: &str, pattern: &str) -> bool {
        // Split pattern by * and check each part exists in order
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            // No wildcards - exact match
            return text == pattern;
        }

        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                // First part must be at start
                if !text.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } else if i == parts.len() - 1 {
                // Last part must be at end
                if !text.ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts must exist in order
                if let Some(found_pos) = text[pos..].find(part) {
                    pos += found_pos + part.len();
                } else {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_simple_glob_match() {
        let replacer = BatchReplacer::new("test", "replacement".to_string(), None, PathBuf::from("."))
            .unwrap();

        assert!(replacer.simple_glob_match("test.rs", "*.rs"));
        assert!(replacer.simple_glob_match("test.rs", "test.*"));
        assert!(replacer.simple_glob_match("test.rs", "test.rs"));
        assert!(!replacer.simple_glob_match("test.ts", "*.rs"));
    }

    #[test]
    fn test_preview_file() -> Result<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "hello world\nhello rust\ngoodbye world")?;

        let replacer = BatchReplacer::new(
            "hello",
            "hi".to_string(),
            None,
            temp.path().to_path_buf(),
        )?;

        let content = fs::read_to_string(&file_path)?;
        let diff = replacer.preview_file(&file_path, &content)?;

        assert_eq!(diff.num_changes, 2);
        assert_eq!(diff.changes[0].line, 1);
        assert_eq!(diff.changes[1].line, 2);

        Ok(())
    }

    #[test]
    fn test_apply_to_file() -> Result<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "foo bar foo")?;

        let replacer = BatchReplacer::new(
            "foo",
            "baz".to_string(),
            None,
            temp.path().to_path_buf(),
        )?;

        let num_replacements = replacer.apply_to_file(&file_path)?;
        assert_eq!(num_replacements, 2);

        let content = fs::read_to_string(&file_path)?;
        assert_eq!(content, "baz bar baz");

        Ok(())
    }
}
