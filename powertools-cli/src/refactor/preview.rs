use serde::Serialize;
use std::collections::HashMap;
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

/// Type of change being made
#[derive(Debug, Clone, Serialize, PartialEq)]
#[allow(dead_code)] // Used in future refactoring implementations
pub enum ChangeType {
    /// Renaming a symbol
    Rename,
    /// Moving a symbol to a different location
    Move,
    /// Extracting code into a function/method
    Extract,
    /// Inlining a function/variable
    Inline,
    /// Updating an import statement
    ImportUpdate,
    /// Adding a new import
    ImportAdd,
    /// Removing an import
    ImportRemove,
    /// Other/custom change
    Other,
}

/// Risk level for a change
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Safe change - unlikely to break anything
    Low,
    /// Moderate risk - may affect other code
    Medium,
    /// High risk - likely to require additional changes
    High,
}

/// Import change being tracked
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)] // Used in future refactoring implementations
pub struct ImportChange {
    /// Type of import change
    pub change_type: ChangeType,
    /// Import source (e.g., "./utils" or "react")
    pub source: String,
    /// Symbols being imported/modified
    pub symbols: Vec<String>,
    /// Line number where change occurs
    pub line: usize,
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

    /// Import changes in this file
    pub import_changes: Vec<ImportChange>,

    /// Risk level for changes in this file
    pub risk_level: RiskLevel,
}

impl PreviewDiff {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            num_changes: 0,
            changes: Vec::new(),
            import_changes: Vec::new(),
            risk_level: RiskLevel::Low,
        }
    }

    pub fn add_change(&mut self, change: PreviewChange) {
        self.num_changes += 1;
        self.changes.push(change);
    }

    pub fn add_import_change(&mut self, import_change: ImportChange) {
        self.import_changes.push(import_change);
    }

    #[allow(dead_code)] // Used in future refactoring implementations
    pub fn set_risk_level(&mut self, risk: RiskLevel) {
        self.risk_level = risk;
    }

    /// Calculate risk level based on changes
    pub fn calculate_risk(&mut self) {
        // High risk if:
        // - Many changes (>10)
        // - Import removals
        // - Changes to critical files (main, index, etc.)
        let has_import_removals = self.import_changes.iter()
            .any(|ic| ic.change_type == ChangeType::ImportRemove);

        let is_critical_file = self.file_path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.contains("main") || n.contains("index") || n.contains("app"))
            .unwrap_or(false);

        self.risk_level = if has_import_removals || is_critical_file {
            RiskLevel::High
        } else if self.num_changes > 10 || !self.import_changes.is_empty() {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
    }

    /// Generate a human-readable diff output
    pub fn format_diff(&self) -> String {
        let mut output = String::new();

        // Risk indicator
        let risk_indicator = match self.risk_level {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡",
            RiskLevel::High => "🔴",
        };

        output.push_str(&format!("{} 📝 {}\n", risk_indicator, self.file_path.display()));
        output.push_str(&format!("   {} change{}\n",
            self.num_changes,
            if self.num_changes == 1 { "" } else { "s" }
        ));

        // Show import changes if any
        if !self.import_changes.is_empty() {
            output.push_str(&format!("   {} import change{}\n",
                self.import_changes.len(),
                if self.import_changes.len() == 1 { "" } else { "s" }
            ));
        }
        output.push('\n');

        // Show import changes first
        for import_change in &self.import_changes {
            let action = match import_change.change_type {
                ChangeType::ImportAdd => "➕ Add import",
                ChangeType::ImportRemove => "➖ Remove import",
                ChangeType::ImportUpdate => "🔄 Update import",
                _ => "📦 Import change",
            };
            output.push_str(&format!("  {} from '{}' (line {})\n",
                action, import_change.source, import_change.line));
            if !import_change.symbols.is_empty() {
                output.push_str(&format!("     Symbols: {}\n", import_change.symbols.join(", ")));
            }
        }

        if !self.import_changes.is_empty() && !self.changes.is_empty() {
            output.push('\n');
        }

        // Show code changes
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

/// Multi-file refactoring summary with cross-file analysis
#[derive(Debug, Clone, Serialize)]
pub struct RefactoringSummary {
    /// All file changes
    pub file_changes: Vec<PreviewDiff>,

    /// Overall risk level (highest risk among all files)
    pub overall_risk: RiskLevel,

    /// Total number of files affected
    pub total_files: usize,

    /// Total number of changes across all files
    pub total_changes: usize,

    /// Total import changes across all files
    pub total_import_changes: usize,

    /// Files grouped by risk level
    pub risk_breakdown: HashMap<String, usize>,

    /// Warnings/recommendations for the user
    pub warnings: Vec<String>,
}

impl RefactoringSummary {
    pub fn new(mut file_changes: Vec<PreviewDiff>) -> Self {
        // Calculate risk for each file
        for diff in &mut file_changes {
            diff.calculate_risk();
        }

        let total_files = file_changes.len();
        let total_changes: usize = file_changes.iter().map(|d| d.num_changes).sum();
        let total_import_changes: usize = file_changes.iter().map(|d| d.import_changes.len()).sum();

        // Find highest risk
        let overall_risk = file_changes.iter()
            .map(|d| &d.risk_level)
            .max()
            .cloned()
            .unwrap_or(RiskLevel::Low);

        // Risk breakdown
        let mut risk_breakdown = HashMap::new();
        for diff in &file_changes {
            let key = match diff.risk_level {
                RiskLevel::Low => "low",
                RiskLevel::Medium => "medium",
                RiskLevel::High => "high",
            };
            *risk_breakdown.entry(key.to_string()).or_insert(0) += 1;
        }

        // Generate warnings
        let mut warnings = Vec::new();
        if overall_risk == RiskLevel::High {
            warnings.push("⚠️  High-risk changes detected. Review carefully before applying.".to_string());
        }
        if total_import_changes > 0 {
            warnings.push(format!("📦 {} import changes will be made. Verify all imports resolve correctly.", total_import_changes));
        }
        let high_risk_count = risk_breakdown.get("high").copied().unwrap_or(0);
        if high_risk_count > 0 {
            warnings.push(format!("🔴 {} file(s) have high-risk changes", high_risk_count));
        }

        Self {
            file_changes,
            overall_risk,
            total_files,
            total_changes,
            total_import_changes,
            risk_breakdown,
            warnings,
        }
    }

    /// Format the summary for display
    pub fn format_summary(&self) -> String {
        let mut output = String::new();

        // Header
        let risk_indicator = match self.overall_risk {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡",
            RiskLevel::High => "🔴",
        };

        output.push_str("========================================\n");
        output.push_str(&format!("    {} REFACTORING PREVIEW\n", risk_indicator));
        output.push_str("========================================\n\n");

        // Summary stats
        output.push_str(&format!("📊 {} file{}, {} change{}\n",
            self.total_files,
            if self.total_files == 1 { "" } else { "s" },
            self.total_changes,
            if self.total_changes == 1 { "" } else { "s" }
        ));

        if self.total_import_changes > 0 {
            output.push_str(&format!("📦 {} import change{}\n",
                self.total_import_changes,
                if self.total_import_changes == 1 { "" } else { "s" }
            ));
        }

        // Risk breakdown
        if !self.risk_breakdown.is_empty() {
            output.push_str("\n🎯 Risk Assessment:\n");
            if let Some(high) = self.risk_breakdown.get("high") {
                output.push_str(&format!("   🔴 High:   {} file{}\n", high, if *high == 1 { "" } else { "s" }));
            }
            if let Some(medium) = self.risk_breakdown.get("medium") {
                output.push_str(&format!("   🟡 Medium: {} file{}\n", medium, if *medium == 1 { "" } else { "s" }));
            }
            if let Some(low) = self.risk_breakdown.get("low") {
                output.push_str(&format!("   🟢 Low:    {} file{}\n", low, if *low == 1 { "" } else { "s" }));
            }
        }

        // Warnings
        if !self.warnings.is_empty() {
            output.push_str("\n⚠️  Warnings:\n");
            for warning in &self.warnings {
                output.push_str(&format!("   {}\n", warning));
            }
        }

        output.push_str("\n========================================\n\n");

        // Individual file diffs
        for (i, diff) in self.file_changes.iter().enumerate() {
            output.push_str(&diff.format_diff());
            if i < self.file_changes.len() - 1 {
                output.push_str("\n----------------------------------------\n\n");
            }
        }

        output.push_str("\n========================================\n");
        output
    }
}

/// Generate preview for all files (legacy function - now wraps RefactoringSummary)
pub fn generate_preview(diffs: &[PreviewDiff]) -> String {
    let summary = RefactoringSummary::new(diffs.to_vec());
    summary.format_summary()
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
