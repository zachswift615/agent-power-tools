use anyhow::Result;
use std::path::PathBuf;
use crate::core::{output::OutputWriter, location::parse_location, Symbol, SymbolKind};
use crate::indexers::ScipQuery;

pub async fn run(
    location: String,
    project_root: PathBuf,
    format: &crate::OutputFormat,
) -> Result<()> {
    let output = OutputWriter::new(format);
    let loc = parse_location(&location)?;

    println!("Finding definition for: {}", location);

    // Load all SCIP indexes
    let query = ScipQuery::from_project(project_root)?;

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