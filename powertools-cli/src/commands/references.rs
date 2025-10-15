use anyhow::Result;
use std::path::PathBuf;
use crate::core::{output::OutputWriter, Reference, Language, location::parse_location};
use crate::indexers::{ScipQuery, SwiftLsp, UnifiedQuery};

/// Find references and return them (for MCP/API use)
pub async fn find_references(
    symbol: String,
    project_root: PathBuf,
    include_declarations: bool,
) -> Result<Vec<Reference>> {
    // Check if symbol is a location (file:line:column) or a symbol name
    if symbol.contains(':') && symbol.split(':').count() >= 3 {
        // It's a location - use position-based search (works for SCIP and LSP)
        let loc = parse_location(&symbol)?;

        // Detect language from file extension
        let language = loc.file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(Language::from_extension)
            .unwrap_or(Language::Unknown);

        // Use LSP for Swift, SCIP for everything else
        let mut query = match language {
            Language::Swift => {
                // Use sourcekit-lsp for Swift
                SwiftLsp::create_query(project_root)?
            }
            _ => {
                // Use SCIP for other languages
                UnifiedQuery::scip_only(project_root)?
            }
        };

        query.find_references_at_position(&loc.file_path, loc.line, loc.column, include_declarations)
    } else {
        // It's a symbol name - use name-based search (SCIP only)
        let query = ScipQuery::from_project(project_root)?;
        query.find_references(&symbol, include_declarations)
    }
}

pub async fn run(
    symbol: String,
    include_declarations: bool,
    project_root: PathBuf,
    format: &crate::OutputFormat,
) -> Result<()> {
    let output = OutputWriter::new(format);

    println!("Finding references for: {}", symbol);

    let references = find_references(symbol.clone(), project_root, include_declarations).await?;

    if references.is_empty() {
        output.write_error(&format!("No references found for symbol: {}", symbol))?;
    } else {
        println!("Found {} references", references.len());
        output.write_references(&references)?;
    }

    Ok(())
}