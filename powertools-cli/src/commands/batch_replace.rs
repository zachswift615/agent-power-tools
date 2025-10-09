use anyhow::Result;
use std::path::PathBuf;

use crate::refactor::{BatchReplacer, generate_preview};

pub async fn run(
    pattern: String,
    replacement: String,
    file_pattern: Option<String>,
    path: Option<PathBuf>,
    preview: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    let search_path = path.unwrap_or_else(|| PathBuf::from("."));

    let replacer = BatchReplacer::new(
        &pattern,
        replacement.clone(),
        file_pattern.clone(),
        search_path.clone(),
    )?;

    if preview {
        // Preview mode - show what would change
        let diffs = replacer.preview()?;

        match format {
            crate::OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&diffs)?);
            }
            _ => {
                if diffs.is_empty() {
                    println!("No matches found.");
                } else {
                    println!("{}", generate_preview(&diffs));
                    println!("\n💡 Run without --preview to apply changes");
                }
            }
        }
    } else {
        // Apply mode - make the changes
        let result = replacer.apply()?;

        match format {
            crate::OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                println!("✓ Batch replacement complete!");
                println!("  Files scanned: {}", result.files_scanned);
                println!("  Files matched: {}", result.files_matched);
                println!("  Replacements made: {}", result.replacements_made);

                if !result.files_modified.is_empty() {
                    println!("\nModified files:");
                    for file in &result.files_modified {
                        println!("  • {}", file.display());
                    }
                }

                if !result.errors.is_empty() {
                    println!("\nErrors:");
                    for error in &result.errors {
                        println!("  ⚠️  {}", error);
                    }
                }
            }
        }
    }

    Ok(())
}
