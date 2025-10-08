use anyhow::Result;
use std::path::PathBuf;
use crate::core::output::OutputWriter;
use crate::indexers::ScipQuery;

pub async fn run(
    symbol: String,
    include_declarations: bool,
    project_root: PathBuf,
    format: &crate::OutputFormat,
) -> Result<()> {
    let output = OutputWriter::new(format);

    println!("Finding references for: {}", symbol);

    // Load all SCIP indexes
    let query = ScipQuery::from_project(project_root)?;
    let references = query.find_references(&symbol, include_declarations)?;

    if references.is_empty() {
        output.write_error(&format!("No references found for symbol: {}", symbol))?;
    } else {
        println!("Found {} references", references.len());
        output.write_references(&references)?;
    }

    Ok(())
}