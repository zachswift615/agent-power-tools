use anyhow::Result;
use std::path::PathBuf;

use crate::indexers::ScipQuery;
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
