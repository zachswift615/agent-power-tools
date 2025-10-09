use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::preview::{RefactoringSummary, RiskLevel};

/// A single file operation in a transaction
#[derive(Debug, Clone, Serialize)]
pub struct FileOperation {
    /// The file path
    pub path: PathBuf,

    /// The original content (for rollback)
    pub original_content: String,

    /// The new content to write
    pub new_content: String,

    /// Whether this operation has been applied
    pub applied: bool,
}

/// Transaction execution mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionMode {
    /// Actually write files
    Execute,

    /// Dry-run - don't write files
    DryRun,
}

/// Refactoring transaction with atomic all-or-nothing semantics
#[derive(Debug)]
pub struct RefactoringTransaction {
    /// All file operations in this transaction
    operations: Vec<FileOperation>,

    /// Original file contents for rollback (path -> content)
    backup: HashMap<PathBuf, String>,

    /// Whether the transaction has been committed
    committed: bool,

    /// Transaction mode
    mode: TransactionMode,
}

impl RefactoringTransaction {
    /// Create a new transaction
    pub fn new(mode: TransactionMode) -> Self {
        Self {
            operations: Vec::new(),
            backup: HashMap::new(),
            committed: false,
            mode,
        }
    }

    /// Add a file operation to the transaction
    pub fn add_operation(
        &mut self,
        path: PathBuf,
        original_content: String,
        new_content: String,
    ) -> Result<()> {
        if self.committed {
            anyhow::bail!("Cannot add operations to a committed transaction");
        }

        // Store backup if we haven't already
        self.backup
            .entry(path.clone())
            .or_insert_with(|| original_content.clone());

        self.operations.push(FileOperation {
            path,
            original_content,
            new_content,
            applied: false,
        });

        Ok(())
    }

    /// Add a file operation by reading the current file content
    pub fn add_file_change(&mut self, path: PathBuf, new_content: String) -> Result<()> {
        let original_content = if path.exists() {
            fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?
        } else {
            String::new()
        };

        self.add_operation(path, original_content, new_content)
    }

    /// Execute all operations in the transaction
    pub fn commit(&mut self) -> Result<TransactionResult> {
        if self.committed {
            anyhow::bail!("Transaction has already been committed");
        }

        let mut result = TransactionResult {
            mode: self.mode,
            total_operations: self.operations.len(),
            successful_operations: 0,
            failed_operations: 0,
            files_modified: Vec::new(),
            errors: Vec::new(),
        };

        // In dry-run mode, just simulate
        if self.mode == TransactionMode::DryRun {
            result.successful_operations = self.operations.len();
            result.files_modified = self.operations.iter().map(|op| op.path.clone()).collect();
            self.committed = true;
            return Ok(result);
        }

        // Execute each operation
        for operation in &mut self.operations {
            match Self::apply_operation_static(operation) {
                Ok(()) => {
                    operation.applied = true;
                    result.successful_operations += 1;
                    result.files_modified.push(operation.path.clone());
                }
                Err(e) => {
                    result.failed_operations += 1;
                    result.errors.push(format!("{}: {}", operation.path.display(), e));

                    // Rollback on first error
                    if let Err(rollback_err) = self.rollback() {
                        result.errors.push(format!(
                            "CRITICAL: Rollback failed: {}. Manual recovery may be required.",
                            rollback_err
                        ));
                    } else {
                        result
                            .errors
                            .push("Transaction rolled back successfully".to_string());
                    }

                    return Err(anyhow::anyhow!(
                        "Transaction failed: {}. All changes have been rolled back.",
                        e
                    ));
                }
            }
        }

        self.committed = true;
        Ok(result)
    }

    /// Apply a single operation (write file) - static method to avoid borrow issues
    fn apply_operation_static(operation: &FileOperation) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = operation.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create directory: {}", parent.display())
            })?;
        }

        fs::write(&operation.path, &operation.new_content)
            .with_context(|| format!("Failed to write file: {}", operation.path.display()))?;

        Ok(())
    }

    /// Rollback all applied operations
    pub fn rollback(&mut self) -> Result<()> {
        let mut errors = Vec::new();

        // Rollback in reverse order
        for operation in self.operations.iter().rev() {
            if !operation.applied {
                continue;
            }

            // Restore original content
            if let Err(e) = fs::write(&operation.path, &operation.original_content) {
                errors.push(format!("{}: {}", operation.path.display(), e));
            }
        }

        if !errors.is_empty() {
            anyhow::bail!("Rollback encountered errors: {}", errors.join("; "));
        }

        Ok(())
    }

    /// Get a preview summary of this transaction
    pub fn preview(&self) -> Result<RefactoringSummary> {
        use super::preview::{ChangeType, ImportChange, PreviewChange, PreviewDiff};

        let mut file_changes = Vec::new();

        for operation in &self.operations {
            let mut diff = PreviewDiff::new(operation.path.clone());

            // Simple line-by-line diff
            let old_lines: Vec<&str> = operation.original_content.lines().collect();
            let new_lines: Vec<&str> = operation.new_content.lines().collect();

            // Find differences
            for (line_num, (old, new)) in old_lines.iter().zip(new_lines.iter()).enumerate() {
                if old != new {
                    diff.add_change(PreviewChange {
                        line: line_num + 1,
                        column: 1,
                        original: old.to_string(),
                        replacement: new.to_string(),
                        line_content: new.to_string(),
                    });
                }
            }

            // Handle added lines
            if new_lines.len() > old_lines.len() {
                for (i, new) in new_lines.iter().enumerate().skip(old_lines.len()) {
                    diff.add_change(PreviewChange {
                        line: i + 1,
                        column: 1,
                        original: String::new(),
                        replacement: new.to_string(),
                        line_content: new.to_string(),
                    });
                }
            }

            // TODO: Detect import changes using import analyzers
            // For now, we'll do simple heuristic detection
            let has_import_changes = operation.new_content.contains("import ")
                != operation.original_content.contains("import ");

            if has_import_changes {
                diff.add_import_change(ImportChange {
                    change_type: ChangeType::Other,
                    source: "detected".to_string(),
                    symbols: Vec::new(),
                    line: 1,
                });
            }

            file_changes.push(diff);
        }

        Ok(RefactoringSummary::new(file_changes))
    }

    /// Get the list of operations
    pub fn operations(&self) -> &[FileOperation] {
        &self.operations
    }

    /// Check if transaction is committed
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

/// Result of a transaction execution
#[derive(Debug, Clone, Serialize)]
pub struct TransactionResult {
    /// Transaction mode used
    pub mode: TransactionMode,

    /// Total number of operations
    pub total_operations: usize,

    /// Number of successful operations
    pub successful_operations: usize,

    /// Number of failed operations
    pub failed_operations: usize,

    /// Files that were modified
    pub files_modified: Vec<PathBuf>,

    /// Errors encountered
    pub errors: Vec<String>,
}

impl TransactionResult {
    /// Check if transaction was successful
    pub fn is_success(&self) -> bool {
        self.failed_operations == 0
    }

    /// Format result for display
    pub fn format_summary(&self) -> String {
        let mut output = String::new();

        output.push_str("========================================\n");
        output.push_str(if self.mode == TransactionMode::DryRun {
            "       DRY-RUN TRANSACTION RESULT\n"
        } else {
            "         TRANSACTION RESULT\n"
        });
        output.push_str("========================================\n\n");

        if self.is_success() {
            output.push_str("✅ Transaction completed successfully\n\n");
        } else {
            output.push_str("❌ Transaction failed and was rolled back\n\n");
        }

        output.push_str(&format!(
            "📊 {} total operation{}\n",
            self.total_operations,
            if self.total_operations == 1 { "" } else { "s" }
        ));
        output.push_str(&format!(
            "✅ {} successful\n",
            self.successful_operations
        ));

        if self.failed_operations > 0 {
            output.push_str(&format!("❌ {} failed\n", self.failed_operations));
        }

        if !self.files_modified.is_empty() {
            output.push_str(&format!(
                "\n📝 {} file{} modified:\n",
                self.files_modified.len(),
                if self.files_modified.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
            for file in &self.files_modified {
                output.push_str(&format!("   {}\n", file.display()));
            }
        }

        if !self.errors.is_empty() {
            output.push_str("\n⚠️  Errors:\n");
            for error in &self.errors {
                output.push_str(&format!("   {}\n", error));
            }
        }

        output.push_str("\n========================================\n");
        output
    }
}

impl Serialize for TransactionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TransactionMode::Execute => serializer.serialize_str("execute"),
            TransactionMode::DryRun => serializer.serialize_str("dry_run"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_transaction_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "original").unwrap();

        let mut tx = RefactoringTransaction::new(TransactionMode::DryRun);
        tx.add_operation(
            file_path.clone(),
            "original".to_string(),
            "modified".to_string(),
        )
        .unwrap();

        let result = tx.commit().unwrap();
        assert!(result.is_success());
        assert_eq!(result.successful_operations, 1);

        // File should NOT be modified in dry-run
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "original");
    }

    #[test]
    fn test_transaction_execute() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "original").unwrap();

        let mut tx = RefactoringTransaction::new(TransactionMode::Execute);
        tx.add_operation(
            file_path.clone(),
            "original".to_string(),
            "modified".to_string(),
        )
        .unwrap();

        let result = tx.commit().unwrap();
        assert!(result.is_success());
        assert_eq!(result.successful_operations, 1);

        // File SHOULD be modified in execute mode
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "modified");
    }

    #[test]
    fn test_transaction_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, "original1").unwrap();
        fs::write(&file2, "original2").unwrap();

        let mut tx = RefactoringTransaction::new(TransactionMode::Execute);
        tx.add_operation(file1.clone(), "original1".to_string(), "modified1".to_string())
            .unwrap();

        // This will fail because we're trying to write to a non-existent directory
        tx.add_operation(
            PathBuf::from("/nonexistent/path/file2.txt"),
            "original2".to_string(),
            "modified2".to_string(),
        )
        .unwrap();

        // Commit should fail and rollback
        let result = tx.commit();
        assert!(result.is_err());

        // file1 should be restored to original
        let content1 = fs::read_to_string(&file1).unwrap();
        assert_eq!(content1, "original1");
    }

    #[test]
    fn test_transaction_preview() {
        let mut tx = RefactoringTransaction::new(TransactionMode::DryRun);
        tx.add_operation(
            PathBuf::from("test.rs"),
            "let x = 1;".to_string(),
            "let x = 2;".to_string(),
        )
        .unwrap();

        let preview = tx.preview().unwrap();
        assert_eq!(preview.total_files, 1);
        assert_eq!(preview.total_changes, 1);
    }
}
