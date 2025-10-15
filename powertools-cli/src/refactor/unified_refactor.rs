///! Unified Refactoring API
///!
///! This module provides a unified interface for refactoring operations
///! that works across both SCIP and LSP backends. It abstracts away the
///! differences between the two approaches and provides a consistent API.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::core::Language;
use crate::indexers::SwiftLsp;
use crate::indexers::lsp_query::apply_workspace_edit;

/// Unified refactoring API that routes to SCIP or LSP based on language
#[allow(dead_code)]
pub struct UnifiedRefactor {
    project_root: PathBuf,
}

/// Result of a refactoring operation
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefactorResult {
    pub success: bool,
    pub files_modified: usize,
    pub edits_applied: usize,
    pub backend: String,
    pub modified_files: Vec<PathBuf>,
    pub error: Option<String>,
}

/// Preview of a refactoring operation (before applying)
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefactorPreview {
    pub files_to_modify: usize,
    pub total_edits: usize,
    pub backend: String,
    pub changes: Vec<FileChange>,
}

/// Description of changes to a single file
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileChange {
    pub file_path: PathBuf,
    pub edits_count: usize,
}

#[allow(dead_code)]
impl UnifiedRefactor {
    /// Create a new unified refactor instance
    ///
    /// # Arguments
    /// * `project_root` - Root directory of the project
    ///
    /// # Returns
    /// A new UnifiedRefactor instance
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Rename a symbol at a given position
    ///
    /// This automatically detects the language and routes to the appropriate backend.
    ///
    /// # Arguments
    /// * `file_path` - File containing the symbol
    /// * `line` - Line number (1-indexed)
    /// * `column` - Column number (1-indexed)
    /// * `new_name` - New name for the symbol
    /// * `preview` - Whether to preview only (don't apply changes)
    ///
    /// # Returns
    /// Result or Preview depending on `preview` parameter
    pub fn rename_symbol(
        &self,
        file_path: &Path,
        line: usize,
        column: usize,
        new_name: String,
        preview: bool,
    ) -> Result<RefactorResult> {
        let language = self.detect_language(file_path);

        match language {
            Language::Swift => {
                self.rename_symbol_lsp(file_path, line, column, new_name, preview)
            }
            _ => {
                self.rename_symbol_scip(file_path, line, column, new_name, preview)
            }
        }
    }

    /// Inline a variable at a given position
    ///
    /// # Arguments
    /// * `file_path` - File containing the variable
    /// * `line` - Line number (1-indexed)
    /// * `column` - Column number (1-indexed)
    /// * `preview` - Whether to preview only (don't apply changes)
    ///
    /// # Returns
    /// Result or Preview depending on `preview` parameter
    pub fn inline_variable(
        &self,
        file_path: &Path,
        line: usize,
        column: usize,
        preview: bool,
    ) -> Result<RefactorResult> {
        let language = self.detect_language(file_path);

        match language {
            Language::Swift => {
                self.inline_variable_lsp(file_path, line, column, preview)
            }
            _ => {
                self.inline_variable_scip(file_path, line, column, preview)
            }
        }
    }

    /// Check if rename is possible at a given position
    ///
    /// # Arguments
    /// * `file_path` - File to check
    /// * `line` - Line number (1-indexed)
    /// * `column` - Column number (1-indexed)
    ///
    /// # Returns
    /// true if rename is possible, false otherwise
    pub fn can_rename(&self, file_path: &Path, line: usize, column: usize) -> Result<bool> {
        let language = self.detect_language(file_path);

        match language {
            Language::Swift => {
                let mut lsp_query = SwiftLsp::start(self.project_root.clone())?;
                lsp_query.can_rename_symbol(file_path, line, column)
            }
            _ => {
                // For SCIP, we always allow rename attempts
                // The actual rename will fail if it's not possible
                Ok(true)
            }
        }
    }

    /// Detect language from file extension
    fn detect_language(&self, file_path: &Path) -> Language {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(Language::from_extension)
            .unwrap_or(Language::Unknown)
    }

    /// LSP-based rename implementation
    fn rename_symbol_lsp(
        &self,
        file_path: &Path,
        line: usize,
        column: usize,
        new_name: String,
        preview: bool,
    ) -> Result<RefactorResult> {
        let mut lsp_query = SwiftLsp::start(self.project_root.clone())?;

        // Validate rename is possible
        let can_rename = lsp_query.can_rename_symbol(file_path, line, column)?;
        if !can_rename {
            return Ok(RefactorResult {
                success: false,
                files_modified: 0,
                edits_applied: 0,
                backend: "LSP".to_string(),
                modified_files: vec![],
                error: Some("Symbol cannot be renamed at this location".to_string()),
            });
        }

        // Get workspace edit
        let workspace_edit = lsp_query.rename_symbol(file_path, line, column, new_name)?;

        if preview {
            // Just count edits for preview
            let files_count = workspace_edit
                .changes
                .as_ref()
                .map(|changes| changes.len())
                .unwrap_or(0);

            let total_edits = workspace_edit
                .changes
                .as_ref()
                .map(|changes| changes.values().map(|edits| edits.len()).sum())
                .unwrap_or(0);

            Ok(RefactorResult {
                success: true,
                files_modified: files_count,
                edits_applied: total_edits,
                backend: "LSP (preview)".to_string(),
                modified_files: vec![],
                error: None,
            })
        } else {
            // Apply changes
            let (modified_files, total_edits) = apply_workspace_edit(&workspace_edit)
                .with_context(|| "Failed to apply workspace edit")?;

            Ok(RefactorResult {
                success: true,
                files_modified: modified_files.len(),
                edits_applied: total_edits,
                backend: "LSP".to_string(),
                modified_files,
                error: None,
            })
        }
    }

    /// SCIP-based rename implementation
    ///
    /// TODO: Wire this to the existing SymbolRenamer implementation
    /// For now, users should continue using the commands directly which
    /// will route to SCIP correctly. This unified API is primarily for
    /// LSP-based languages currently.
    fn rename_symbol_scip(
        &self,
        _file_path: &Path,
        _line: usize,
        _column: usize,
        _new_name: String,
        _preview: bool,
    ) -> Result<RefactorResult> {
        // TODO: Call existing SCIP renamer from refactor/rename.rs
        // let scip_query = ScipQuery::from_project(self.project_root.clone())?;
        // let renamer = SymbolRenamer::new(&scip_query, self.project_root.clone());
        // let options = RenameOptions { ... };
        // let result = renamer.rename(options)?;

        Ok(RefactorResult {
            success: false,
            files_modified: 0,
            edits_applied: 0,
            backend: "SCIP".to_string(),
            modified_files: vec![],
            error: Some("SCIP integration pending - use rename_symbol command directly".to_string()),
        })
    }

    /// LSP-based inline variable implementation
    fn inline_variable_lsp(
        &self,
        file_path: &Path,
        line: usize,
        column: usize,
        preview: bool,
    ) -> Result<RefactorResult> {
        let mut lsp_query = SwiftLsp::start(self.project_root.clone())?;

        // Get code actions
        let actions = lsp_query.get_code_actions(
            file_path,
            line,
            column,
            line,
            column,
            Some(vec![lsp_types::CodeActionKind::REFACTOR_INLINE]),
        )?;

        // Find inline action
        let inline_action = actions
            .iter()
            .find(|action| match action {
                lsp_types::CodeActionOrCommand::CodeAction(ca) => {
                    ca.title.to_lowercase().contains("inline")
                }
                _ => false,
            })
            .ok_or_else(|| anyhow::anyhow!("No inline variable action available"))?;

        // Extract workspace edit
        let workspace_edit = match inline_action {
            lsp_types::CodeActionOrCommand::CodeAction(ca) => ca
                .edit
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Code action missing WorkspaceEdit"))?,
            _ => return Err(anyhow::anyhow!("Expected CodeAction, got Command")),
        };

        if preview {
            let files_count = workspace_edit
                .changes
                .as_ref()
                .map(|changes| changes.len())
                .unwrap_or(0);

            let total_edits = workspace_edit
                .changes
                .as_ref()
                .map(|changes| changes.values().map(|edits| edits.len()).sum())
                .unwrap_or(0);

            Ok(RefactorResult {
                success: true,
                files_modified: files_count,
                edits_applied: total_edits,
                backend: "LSP (preview)".to_string(),
                modified_files: vec![],
                error: None,
            })
        } else {
            let (modified_files, total_edits) = apply_workspace_edit(workspace_edit)?;

            Ok(RefactorResult {
                success: true,
                files_modified: modified_files.len(),
                edits_applied: total_edits,
                backend: "LSP".to_string(),
                modified_files,
                error: None,
            })
        }
    }

    /// SCIP-based inline variable implementation
    ///
    /// TODO: Wire this to the existing VariableInliner implementation
    /// For now, users should continue using the commands directly which
    /// will route to SCIP correctly. This unified API is primarily for
    /// LSP-based languages currently.
    fn inline_variable_scip(
        &self,
        _file_path: &Path,
        _line: usize,
        _column: usize,
        _preview: bool,
    ) -> Result<RefactorResult> {
        // TODO: Call existing SCIP inliner from refactor/inline.rs
        // let scip_query = ScipQuery::from_project(self.project_root.clone())?;
        // let inliner = VariableInliner::new(&scip_query, self.project_root.clone());
        // let options = InlineOptions { ... };
        // let result = inliner.inline(options)?;

        Ok(RefactorResult {
            success: false,
            files_modified: 0,
            edits_applied: 0,
            backend: "SCIP".to_string(),
            modified_files: vec![],
            error: Some("SCIP integration pending - use inline_variable command directly".to_string()),
        })
    }
}
