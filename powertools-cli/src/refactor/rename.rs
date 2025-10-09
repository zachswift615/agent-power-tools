use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::{Location, Reference};
use crate::indexers::ScipQuery;

use super::imports::{get_analyzer_for_file, ImportStatement};
use super::preview::{ChangeType, ImportChange, PreviewChange, PreviewDiff, RefactoringSummary};
use super::transaction::{RefactoringTransaction, TransactionMode, TransactionResult};

/// Options for rename symbol operation
#[derive(Debug, Clone)]
pub struct RenameOptions {
    /// The file where the symbol is located
    pub file_path: PathBuf,

    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,

    /// New name for the symbol
    pub new_name: String,

    /// Whether to update imports/exports
    pub update_imports: bool,

    /// Transaction mode (Execute or DryRun)
    pub mode: TransactionMode,
}

/// Result of a rename operation
#[derive(Debug, Clone, Serialize)]
pub struct RenameResult {
    /// Original symbol name
    pub old_name: String,

    /// New symbol name
    pub new_name: String,

    /// Number of references updated
    pub references_updated: usize,

    /// Number of files modified
    pub files_modified: usize,

    /// Number of imports updated
    pub imports_updated: usize,

    /// Transaction result
    pub transaction_result: TransactionResult,
}

/// Rename a symbol across the codebase
pub struct SymbolRenamer<'a> {
    scip_query: &'a ScipQuery,
    project_root: PathBuf,
}

impl<'a> SymbolRenamer<'a> {
    pub fn new(scip_query: &'a ScipQuery, project_root: PathBuf) -> Self {
        Self {
            scip_query,
            project_root,
        }
    }

    /// Perform the rename operation
    pub fn rename(&self, options: RenameOptions) -> Result<RenameResult> {
        // Step 1: Find the symbol definition
        let definition = self
            .scip_query
            .find_definition(&options.file_path, options.line, options.column)?
            .ok_or_else(|| anyhow::anyhow!("No symbol found at the specified location"))?;

        // Extract the old symbol name from the source
        let old_name = self.extract_symbol_name(&definition)?;

        // Step 2: Find all references to this symbol
        let references = self
            .scip_query
            .find_references(&old_name, true)?; // Include declarations

        if references.is_empty() {
            anyhow::bail!("No references found for symbol '{}'", old_name);
        }

        // Step 3: Group references by file
        let mut references_by_file: HashMap<PathBuf, Vec<Reference>> = HashMap::new();
        for reference in references {
            references_by_file
                .entry(reference.location.file_path.clone())
                .or_insert_with(Vec::new)
                .push(reference);
        }

        // Step 4: Build a transaction with all file changes
        let mut transaction = RefactoringTransaction::new(options.mode);

        for (file_path, file_refs) in &references_by_file {
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

            let new_content = self.replace_symbol_in_file(&content, file_refs, &old_name, &options.new_name)?;

            transaction.add_operation(file_path.clone(), content, new_content)?;
        }

        // Step 5: Update imports/exports if requested
        let mut imports_updated = 0;
        if options.update_imports {
            imports_updated = self.update_imports_for_rename(
                &mut transaction,
                &old_name,
                &options.new_name,
                &references_by_file,
            )?;
        }

        // Step 6: Commit the transaction
        let transaction_result = transaction.commit()?;

        Ok(RenameResult {
            old_name,
            new_name: options.new_name,
            references_updated: references_by_file.values().map(|v| v.len()).sum(),
            files_modified: transaction_result.files_modified.len(),
            imports_updated,
            transaction_result,
        })
    }

    /// Generate a preview of the rename operation
    pub fn preview(&self, options: RenameOptions) -> Result<RefactoringSummary> {
        // Find the symbol definition
        let definition = self
            .scip_query
            .find_definition(&options.file_path, options.line, options.column)?
            .ok_or_else(|| anyhow::anyhow!("No symbol found at the specified location"))?;

        let old_name = self.extract_symbol_name(&definition)?;

        // Find all references
        let references = self.scip_query.find_references(&old_name, true)?;

        if references.is_empty() {
            anyhow::bail!("No references found for symbol '{}'", old_name);
        }

        // Group references by file
        let mut references_by_file: HashMap<PathBuf, Vec<Reference>> = HashMap::new();
        for reference in references {
            references_by_file
                .entry(reference.location.file_path.clone())
                .or_insert_with(Vec::new)
                .push(reference);
        }

        // Build preview diffs
        let mut file_changes = Vec::new();

        for (file_path, file_refs) in &references_by_file {
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

            let mut diff = PreviewDiff::new(file_path.clone());

            // Add changes for each reference
            for reference in file_refs {
                let line_content = content
                    .lines()
                    .nth(reference.location.line - 1)
                    .unwrap_or("")
                    .to_string();

                diff.add_change(PreviewChange {
                    line: reference.location.line,
                    column: reference.location.column,
                    original: old_name.clone(),
                    replacement: options.new_name.clone(),
                    line_content,
                });
            }

            // Check for import changes
            if options.update_imports {
                if let Some(analyzer) = get_analyzer_for_file(file_path) {
                    if let Ok(imports) = analyzer.find_imports(file_path) {
                        for import in imports {
                            if import.symbols.contains(&old_name) {
                                diff.add_import_change(ImportChange {
                                    change_type: ChangeType::ImportUpdate,
                                    source: import.source.clone(),
                                    symbols: vec![old_name.clone(), options.new_name.clone()],
                                    line: import.location.line,
                                });
                            }
                        }
                    }
                }
            }

            file_changes.push(diff);
        }

        Ok(RefactoringSummary::new(file_changes))
    }

    /// Extract symbol name from location
    fn extract_symbol_name(&self, location: &Location) -> Result<String> {
        let content = fs::read_to_string(&location.file_path)
            .with_context(|| format!("Failed to read file: {}", location.file_path.display()))?;

        let line = content
            .lines()
            .nth(location.line - 1)
            .ok_or_else(|| anyhow::anyhow!("Line {} not found in file", location.line))?;

        // Extract the identifier at the column position
        let start_col = location.column - 1;
        let chars: Vec<char> = line.chars().collect();

        if start_col >= chars.len() {
            anyhow::bail!("Column {} out of bounds in line {}", location.column, location.line);
        }

        // Find the start of the identifier (go backwards)
        let mut id_start = start_col;
        while id_start > 0 && (chars[id_start - 1].is_alphanumeric() || chars[id_start - 1] == '_') {
            id_start -= 1;
        }

        // Find the end of the identifier (go forwards)
        let mut id_end = start_col;
        while id_end < chars.len() && (chars[id_end].is_alphanumeric() || chars[id_end] == '_') {
            id_end += 1;
        }

        let symbol_name: String = chars[id_start..id_end].iter().collect();

        if symbol_name.is_empty() {
            anyhow::bail!("No identifier found at location");
        }

        Ok(symbol_name)
    }

    /// Replace all occurrences of the symbol in a file
    fn replace_symbol_in_file(
        &self,
        content: &str,
        references: &[Reference],
        old_name: &str,
        new_name: &str,
    ) -> Result<String> {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // Sort references by line and column in reverse order
        // This ensures we don't mess up positions when replacing
        let mut sorted_refs = references.to_vec();
        sorted_refs.sort_by(|a, b| {
            b.location.line.cmp(&a.location.line)
                .then(b.location.column.cmp(&a.location.column))
        });

        for reference in sorted_refs {
            let line_idx = reference.location.line - 1;
            if line_idx >= lines.len() {
                continue;
            }

            let line = &lines[line_idx];
            let col_idx = reference.location.column - 1;

            // Ensure the symbol actually exists at this location
            if !self.symbol_at_position(line, col_idx, old_name) {
                continue;
            }

            // Replace the symbol
            let mut chars: Vec<char> = line.chars().collect();

            // Find the actual bounds of the identifier
            let mut start = col_idx;
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
                start -= 1;
            }

            let mut end = col_idx;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }

            // Replace
            let new_line = format!(
                "{}{}{}",
                chars[..start].iter().collect::<String>(),
                new_name,
                chars[end..].iter().collect::<String>()
            );

            lines[line_idx] = new_line;
        }

        Ok(lines.join("\n"))
    }

    /// Check if a symbol exists at a specific position in a line
    fn symbol_at_position(&self, line: &str, col_idx: usize, symbol: &str) -> bool {
        let chars: Vec<char> = line.chars().collect();

        if col_idx >= chars.len() {
            return false;
        }

        // Find identifier bounds
        let mut start = col_idx;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        let mut end = col_idx;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        let found_symbol: String = chars[start..end].iter().collect();
        found_symbol == symbol
    }

    /// Update imports/exports after renaming
    fn update_imports_for_rename(
        &self,
        transaction: &mut RefactoringTransaction,
        old_name: &str,
        new_name: &str,
        references_by_file: &HashMap<PathBuf, Vec<Reference>>,
    ) -> Result<usize> {
        let mut imports_updated = 0;

        for file_path in references_by_file.keys() {
            if let Some(analyzer) = get_analyzer_for_file(file_path) {
                let imports = analyzer.find_imports(file_path)?;
                let mut needs_update = false;

                for import in &imports {
                    if import.symbols.contains(&old_name.to_string()) {
                        needs_update = true;
                        break;
                    }
                }

                if needs_update {
                    let content = fs::read_to_string(file_path)?;
                    let mut new_content = content.clone();

                    // Simple string replacement in import statements
                    // TODO: Use AST-based replacement for more precision
                    for import in imports {
                        if import.symbols.contains(&old_name.to_string()) {
                            // Replace old_name with new_name in the import line
                            let lines: Vec<&str> = new_content.lines().collect();
                            if import.location.line > 0 && import.location.line <= lines.len() {
                                let old_line = lines[import.location.line - 1];
                                let new_line = old_line.replace(
                                    &format!(" {} ", old_name),
                                    &format!(" {} ", new_name)
                                ).replace(
                                    &format!("{{{}}}", old_name),
                                    &format!("{{{}}}", new_name)
                                ).replace(
                                    &format!("{{ {} }}", old_name),
                                    &format!("{{ {} }}", new_name)
                                );

                                new_content = new_content.replace(old_line, &new_line);
                                imports_updated += 1;
                            }
                        }
                    }

                    // Update the transaction with the new content
                    transaction.add_operation(file_path.clone(), content, new_content)?;
                }
            }
        }

        Ok(imports_updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_at_position() {
        let renamer = SymbolRenamer {
            scip_query: unsafe { &*(std::ptr::null() as *const ScipQuery) }, // Dummy for testing
            project_root: PathBuf::new(),
        };

        assert!(renamer.symbol_at_position("let foo = 42;", 4, "foo"));
        assert!(renamer.symbol_at_position("  myVar = 10", 2, "myVar"));
        assert!(!renamer.symbol_at_position("let foo = 42;", 4, "bar"));
    }

    #[test]
    fn test_extract_symbol_name() {
        // This would need a real file and location to test properly
        // Skipping for now as it requires file I/O
    }
}
