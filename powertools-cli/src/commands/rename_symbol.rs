use anyhow::Result;
use std::path::PathBuf;

use crate::indexers::ScipQuery;
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
