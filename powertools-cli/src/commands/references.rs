use anyhow::Result;
use std::path::PathBuf;
use crate::core::{output::OutputWriter, Reference};
use crate::indexers::ScipQuery;

/// Find references and return them (for MCP/API use)
pub async fn find_references(
    symbol: String,
    project_root: PathBuf,
    include_declarations: bool,
) -> Result<Vec<Reference>> {
    // Load all SCIP indexes
    let query = ScipQuery::from_project(project_root)?;
    query.find_references(&symbol, include_declarations)
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