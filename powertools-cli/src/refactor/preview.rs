use serde::Serialize;
use std::path::PathBuf;

/// A single change in a file (one line with replacement)
#[derive(Debug, Clone, Serialize)]
pub struct PreviewChange {
    /// Line number (1-indexed)
    pub line: usize,

    /// Column where match starts (1-indexed)
    pub column: usize,

    /// Original text
    pub original: String,

    /// Replacement text
    pub replacement: String,

    /// Full line content (for context)
    pub line_content: String,
}

/// Preview of all changes in a single file
#[derive(Debug, Clone, Serialize)]
pub struct PreviewDiff {
    /// File path
    pub file_path: PathBuf,

    /// Number of changes in this file
    pub num_changes: usize,

    /// Individual changes
    pub changes: Vec<PreviewChange>,
}

impl PreviewDiff {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            num_changes: 0,
            changes: Vec::new(),
        }
    }

    pub fn add_change(&mut self, change: PreviewChange) {
        self.num_changes += 1;
        self.changes.push(change);
    }

    /// Generate a human-readable diff output
    pub fn format_diff(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("📝 {}\n", self.file_path.display()));
        output.push_str(&format!("   {} change{}\n\n",
            self.num_changes,
            if self.num_changes == 1 { "" } else { "s" }
        ));

        for (i, change) in self.changes.iter().enumerate() {
            output.push_str(&format!("  {}:{}\n", change.line, change.column));
            output.push_str(&format!("  - {}\n", change.original));
            output.push_str(&format!("  + {}\n", change.replacement));
            if i < self.changes.len() - 1 {
                output.push('\n');
            }
        }

        output
    }
}

/// Generate preview for all files
pub fn generate_preview(diffs: &[PreviewDiff]) -> String {
    let mut output = String::new();

    let total_files = diffs.len();
    let total_changes: usize = diffs.iter().map(|d| d.num_changes).sum();

    output.push_str("========================================\n");
    output.push_str("           PREVIEW CHANGES\n");
    output.push_str("========================================\n\n");
    output.push_str(&format!("📊 {} file{}, {} change{}\n\n",
        total_files,
        if total_files == 1 { "" } else { "s" },
        total_changes,
        if total_changes == 1 { "" } else { "s" }
    ));

    for (i, diff) in diffs.iter().enumerate() {
        output.push_str(&diff.format_diff());
        if i < diffs.len() - 1 {
            output.push_str("\n----------------------------------------\n\n");
        }
    }

    output.push_str("\n========================================\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_diff_format() {
        let mut diff = PreviewDiff::new(PathBuf::from("src/test.rs"));
        diff.add_change(PreviewChange {
            line: 10,
            column: 5,
            original: "foo".to_string(),
            replacement: "bar".to_string(),
            line_content: "    let x = foo;".to_string(),
        });

        let formatted = diff.format_diff();
        assert!(formatted.contains("src/test.rs"));
        assert!(formatted.contains("10:5"));
        assert!(formatted.contains("- foo"));
        assert!(formatted.contains("+ bar"));
    }
}
