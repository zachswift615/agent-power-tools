use anyhow::{Result, Context};
use std::path::PathBuf;

use crate::core::Language;
use crate::indexers::{ScipQuery, SwiftLsp};
use crate::indexers::lsp_query::apply_workspace_edit;
use crate::refactor::{RenameOptions, SymbolRenamer, TransactionMode};

pub async fn run(
    file_path: PathBuf,
    line: usize,
    column: usize,
    new_name: String,
    project_root: Option<PathBuf>,
    preview: bool,
    update_imports: bool,
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
            // Use LSP-based rename for Swift
            run_lsp_rename(file_path, line, column, new_name, project_root, preview, format).await
        }
        _ => {
            // Use SCIP-based rename for other languages
            run_scip_rename(
                file_path,
                line,
                column,
                new_name,
                project_root,
                preview,
                update_imports,
                format,
            )
            .await
        }
    }
}

/// LSP-based rename (for Swift and other LSP-backed languages)
async fn run_lsp_rename(
    file_path: PathBuf,
    line: usize,
    column: usize,
    new_name: String,
    project_root: PathBuf,
    preview: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    // Create LSP query for Swift
    let mut lsp_query = SwiftLsp::start(project_root)?;

    // First, validate that rename is possible at this location
    let can_rename = lsp_query
        .can_rename_symbol(&file_path, line, column)
        .with_context(|| "Failed to validate rename")?;

    if !can_rename {
        return Err(anyhow::anyhow!(
            "Cannot rename symbol at {}:{}:{}. The symbol may be read-only or not renameable.",
            file_path.display(),
            line,
            column
        ));
    }

    // Get workspace edit from LSP
    let workspace_edit = lsp_query
        .rename_symbol(&file_path, line, column, new_name.clone())
        .with_context(|| format!("Failed to get rename edits from LSP"))?;

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
                    "new_name": new_name,
                    "files_to_modify": files_count,
                    "total_edits": total_edits,
                    "backend": "LSP (sourcekit-lsp)",
                    "changes": workspace_edit.changes.as_ref().map(|changes| {
                        changes.iter().map(|(uri, edits)| {
                            serde_json::json!({
                                "file": uri.as_str().strip_prefix("file://").unwrap_or(uri.as_str()),
                                "edits": edits.len(),
                            })
                        }).collect::<Vec<_>>()
                    }),
                });
                println!("{}", serde_json::to_string_pretty(&preview_data)?);
            }
            _ => {
                println!("🔍 Rename Preview (LSP)");
                println!("Symbol at: {}:{}:{}", file_path.display(), line, column);
                println!("New name: {}", new_name);
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
        let (modified_files, total_edits_applied) = apply_workspace_edit(&workspace_edit)
            .with_context(|| "Failed to apply workspace edit")?;

        match format {
            crate::OutputFormat::Json => {
                let result = serde_json::json!({
                    "success": true,
                    "new_name": new_name,
                    "files_modified": modified_files.len(),
                    "edits_applied": total_edits_applied,
                    "backend": "LSP (sourcekit-lsp)",
                    "modified_files": modified_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                println!("✓ Symbol renamed successfully!");
                println!("  New name: {}", new_name);
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

/// SCIP-based rename (for TypeScript, Python, Rust, etc.)
async fn run_scip_rename(
    file_path: PathBuf,
    line: usize,
    column: usize,
    new_name: String,
    project_root: PathBuf,
    preview: bool,
    update_imports: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    // Load the SCIP index
    let scip_query = ScipQuery::from_project(project_root.clone())?;

    // Create the renamer
    let renamer = SymbolRenamer::new(&scip_query, project_root.clone());

    // Build options
    let options = RenameOptions {
        file_path: file_path.clone(),
        line,
        column,
        new_name: new_name.clone(),
        update_imports,
        mode: if preview {
            TransactionMode::DryRun
        } else {
            TransactionMode::Execute
        },
    };

    if preview {
        // Preview mode - show what would change
        let summary = renamer.preview(options)?;

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
        let result = renamer.rename(options)?;

        match format {
            crate::OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                println!("✓ Symbol renamed successfully!");
                println!("  {} → {}", result.old_name, result.new_name);
                println!("  References updated: {}", result.references_updated);
                println!("  Files modified: {}", result.files_modified);

                if result.imports_updated > 0 {
                    println!("  Imports updated: {}", result.imports_updated);
                }

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
