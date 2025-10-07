use anyhow::Result;
use std::path::PathBuf;
use crate::core::output::OutputWriter;
use crate::indexers::{ScipIndexer, ScipQuery};

pub async fn run(
    symbol: String,
    include_declarations: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    let output = OutputWriter::new(format);

    println!("Finding references for: {}", symbol);

    // Get project root (assume current directory for now)
    let project_root = PathBuf::from(".");

    // Read SCIP index
    let indexer = ScipIndexer::new(project_root.clone());
    let index = indexer.read_index()?;

    // Query for references
    let query = ScipQuery::new(index, project_root);
    let references = query.find_references(&symbol, include_declarations)?;

    if references.is_empty() {
        output.write_error(&format!("No references found for symbol: {}", symbol))?;
    } else {
        println!("Found {} references", references.len());
        output.write_references(&references)?;
    }

    Ok(())
}