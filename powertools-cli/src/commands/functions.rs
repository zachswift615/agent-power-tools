use anyhow::Result;
use std::path::PathBuf;
use ignore::WalkBuilder;
use rayon::prelude::*;
use crate::core::{output::OutputWriter, Language, Symbol};
use crate::analyzers::FunctionFinder;

/// Find functions and return them (for MCP/API use)
pub async fn find_functions(
    path: Option<PathBuf>,
    include_private: bool,
) -> Result<Vec<Symbol>> {
    let search_path = path.unwrap_or_else(|| PathBuf::from("."));
    let mut all_functions = Vec::new();

    if search_path.is_file() {
        // Find functions in single file
        let mut finder = FunctionFinder::new()?;
        let functions = finder.find_in_file(&search_path, include_private)?;
        all_functions.extend(functions);
    } else {
        // Find functions in directory
        let files = collect_source_files(&search_path)?;

        // Process files in parallel
        let results: Vec<Vec<Symbol>> = files
            .par_iter()
            .filter_map(|file| {
                let mut finder = FunctionFinder::new().ok()?;
                finder.find_in_file(file, include_private).ok()
            })
            .collect();

        for functions in results {
            all_functions.extend(functions);
        }
    }

    Ok(all_functions)
}

pub async fn run(
    path: Option<PathBuf>,
    include_private: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    let all_functions = find_functions(path, include_private).await?;
    let output = OutputWriter::new(format);

    if all_functions.is_empty() {
        println!("No functions found");
    } else {
        println!("Found {} functions:", all_functions.len());
        output.write_symbols(&all_functions)?;
    }

    Ok(())
}

fn collect_source_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let walker = WalkBuilder::new(dir)
        .standard_filters(true)
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if Language::from_extension(ext).tree_sitter_language().is_some() {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}