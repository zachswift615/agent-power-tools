use anyhow::Result;
use std::path::PathBuf;
use crate::core::{output::OutputWriter, location::parse_location, Symbol, SymbolKind};
use crate::indexers::{ScipIndexer, ScipQuery};

pub async fn run(
    location: String,
    format: &crate::OutputFormat,
) -> Result<()> {
    let output = OutputWriter::new(format);
    let loc = parse_location(&location)?;

    println!("Finding definition for: {}", location);

    // Get project root (assume current directory for now)
    let project_root = PathBuf::from(".");

    // Read SCIP index
    let indexer = ScipIndexer::new(project_root.clone());
    let index = indexer.read_index()?;

    // Query for definition
    let query = ScipQuery::new(index, project_root);

    match query.find_definition(&loc.file_path, loc.line, loc.column)? {
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