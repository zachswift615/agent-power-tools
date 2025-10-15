use anyhow::Result;
use std::path::PathBuf;
use crate::core::{output::OutputWriter, location::parse_location, Symbol, SymbolKind, Location, Language};
use crate::indexers::{ScipQuery, SwiftLsp, UnifiedQuery};

/// Find definition and return it (for MCP/API use)
pub async fn find_definition(
    location: String,
    project_root: PathBuf,
) -> Result<Option<Location>> {
    let loc = parse_location(&location)?;

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

    query.find_definition(&loc.file_path, loc.line, loc.column)
}

pub async fn run(
    location: String,
    project_root: PathBuf,
    format: &crate::OutputFormat,
) -> Result<()> {
    let output = OutputWriter::new(format);

    println!("Finding definition for: {}", location);

    match find_definition(location, project_root).await? {
        Some(def_location) => {
            // Convert to Symbol for output
            let symbol = Symbol {
                name: "Symbol".to_string(), // Extract from SCIP if available
                kind: SymbolKind::Variable,  // Extract from SCIP if available
                location: def_location,
                container: None,
                signature: None,
                documentation: None,
            };

            output.write_symbols(&[symbol])?;
        }
        None => {
            output.write_error("No definition found at this location")?;
        }
    }

    Ok(())
}