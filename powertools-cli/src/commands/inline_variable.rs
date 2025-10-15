use anyhow::{Result, Context};
use std::path::PathBuf;

use crate::core::Language;
use crate::indexers::{ScipQuery, SwiftLsp};
use crate::indexers::lsp_query::apply_workspace_edit;
use crate::refactor::{InlineOptions, TransactionMode, VariableInliner};

pub async fn run(
    file_path: PathBuf,
    line: usize,
    column: usize,
    project_root: Option<PathBuf>,
    preview: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    let project_root = project_root.unwrap_or_else(|| PathBuf::from("."));

    // Detect language from file extension
    let language = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(Language::from_extension)
        .unwrap_or(Language::Unknown);

    // Route to LSP or SCIP based on language
    match language {
        Language::Swift => {
            // Use LSP-based inline via code actions for Swift
            run_lsp_inline(file_path, line, column, project_root, preview, format).await
        }
        _ => {
            // Use SCIP-based inline for other languages
            run_scip_inline(file_path, line, column, project_root, preview, format).await
        }
    }
}

/// LSP-based inline variable (for Swift and other LSP-backed languages)
async fn run_lsp_inline(
    file_path: PathBuf,
    line: usize,
    column: usize,
    project_root: PathBuf,
    preview: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    // Create LSP query for Swift
    let mut lsp_query = SwiftLsp::start(project_root)?;

    // Get available code actions at this position
    // For inline variable, we need a range - use same line/column for both start and end
    let actions = lsp_query
        .get_code_actions(
            &file_path,
            line,
            column,
            line,
            column,
            Some(vec![lsp_types::CodeActionKind::REFACTOR_INLINE]),
        )
        .with_context(|| "Failed to get code actions from LSP")?;

    // Find an inline variable action
    let inline_action = actions
        .iter()
        .find(|action| {
            match action {
                lsp_types::CodeActionOrCommand::CodeAction(ca) => {
                    ca.title.to_lowercase().contains("inline")
                }
                _ => false,
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No inline variable action available at {}:{}:{}. Make sure the cursor is on a variable declaration.",
                file_path.display(),
                line,
                column
            )
        })?;

    // Extract WorkspaceEdit from the code action
    let workspace_edit = match inline_action {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => {
            ca.edit.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Code action does not contain a WorkspaceEdit")
            })?
        }
        _ => {
            return Err(anyhow::anyhow!("Expected CodeAction, got Command"));
        }
    };

    // Count edits for preview
    let total_edits = workspace_edit
        .changes
        .as_ref()
        .map(|changes| changes.values().map(|edits| edits.len()).sum())
        .unwrap_or(0);

    let files_count = workspace_edit
        .changes
        .as_ref()
        .map(|changes| changes.len())
        .unwrap_or(0);

    if preview {
        // Preview mode - show what would change
        match format {
            crate::OutputFormat::Json => {
                let preview_data = serde_json::json!({
                    "file_path": file_path,
                    "line": line,
                    "column": column,
                    "files_to_modify": files_count,
                    "total_edits": total_edits,
                    "backend": "LSP (sourcekit-lsp)",
                    "action_title": match inline_action {
                        lsp_types::CodeActionOrCommand::CodeAction(ca) => &ca.title,
                        _ => "Unknown",
                    },
                });
                println!("{}", serde_json::to_string_pretty(&preview_data)?);
            }
            _ => {
                println!("🔍 Inline Variable Preview (LSP)");
                println!("Position: {}:{}:{}", file_path.display(), line, column);
                if let lsp_types::CodeActionOrCommand::CodeAction(ca) = inline_action {
                    println!("Action: {}", ca.title);
                }
                println!("Files to modify: {}", files_count);
                println!("Total edits: {}", total_edits);

                if let Some(changes) = &workspace_edit.changes {
                    println!("\nFiles that will be modified:");
                    for (uri, edits) in changes {
                        let file_path = uri
                            .as_str()
                            .strip_prefix("file://")
                            .unwrap_or(uri.as_str());
                        println!("  • {} ({} edits)", file_path, edits.len());
                    }
                }

                println!("\n💡 Run without --preview to apply changes");
            }
        }
    } else {
        // Apply mode - make the changes
        let (modified_files, total_edits_applied) = apply_workspace_edit(workspace_edit)
            .with_context(|| "Failed to apply workspace edit")?;

        match format {
            crate::OutputFormat::Json => {
                let result = serde_json::json!({
                    "success": true,
                    "files_modified": modified_files.len(),
                    "edits_applied": total_edits_applied,
                    "backend": "LSP (sourcekit-lsp)",
                    "modified_files": modified_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                println!("✓ Variable inlined successfully!");
                println!("  Files modified: {}", modified_files.len());
                println!("  Edits applied: {}", total_edits_applied);

                if !modified_files.is_empty() {
                    println!("\nModified files:");
                    for file in &modified_files {
                        println!("  • {}", file.display());
                    }
                }
            }
        }
    }

    Ok(())
}

/// SCIP-based inline variable (for TypeScript, Python, Rust, etc.)
async fn run_scip_inline(
    file_path: PathBuf,
    line: usize,
    column: usize,
    project_root: PathBuf,
    preview: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    // Load the SCIP index
    let scip_query = ScipQuery::from_project(project_root.clone())?;

    // Create the inliner
    let inliner = VariableInliner::new(&scip_query, project_root.clone());

    // Build options
    let options = InlineOptions {
        file_path: file_path.clone(),
        line,
        column,
        mode: if preview {
            TransactionMode::DryRun
        } else {
            TransactionMode::Execute
        },
    };

    if preview {
        // Preview mode - show what would change
        let summary = inliner.preview(options)?;

        match format {
            crate::OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            _ => {
                println!("{}", summary.format_summary());
                println!("\n💡 Run without --preview to apply changes");
            }
        }
    } else {
        // Apply mode - make the changes
        let result = inliner.inline(options)?;

        match format {
            crate::OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                println!("✓ Variable inlined successfully!");
                println!("  Variable: {}", result.variable_name);
                println!("  Replaced with: {}", result.initializer_value);
                println!("  Usages replaced: {}", result.usages_replaced);
                println!("  Files modified: {}", result.files_modified);

                if !result.transaction_result.files_modified.is_empty() {
                    println!("\nModified files:");
                    for file in &result.transaction_result.files_modified {
                        println!("  • {}", file.display());
                    }
                }

                if !result.transaction_result.errors.is_empty() {
                    println!("\n⚠️  Errors:");
                    for error in &result.transaction_result.errors {
                        println!("  {}", error);
                    }
                }
            }
        }
    }

    Ok(())
}
