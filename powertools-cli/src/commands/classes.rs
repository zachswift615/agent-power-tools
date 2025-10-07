use anyhow::Result;
use std::path::PathBuf;
use ignore::WalkBuilder;
use rayon::prelude::*;
use crate::core::{output::OutputWriter, Language, Symbol};
use crate::analyzers::ClassFinder;

pub async fn run(
    path: Option<PathBuf>,
    include_nested: bool,
    format: &crate::OutputFormat,
) -> Result<()> {
    let search_path = path.unwrap_or_else(|| PathBuf::from("."));
    let output = OutputWriter::new(format);

    let mut all_classes = Vec::new();

    if search_path.is_file() {
        // Find classes in single file
        let mut finder = ClassFinder::new()?;
        let classes = finder.find_in_file(&search_path, include_nested)?;
        all_classes.extend(classes);
    } else {
        // Find classes in directory
        let files = collect_source_files(&search_path)?;

        // Process files in parallel
        let results: Vec<Vec<Symbol>> = files
            .par_iter()
            .filter_map(|file| {
                let mut finder = ClassFinder::new().ok()?;
                finder.find_in_file(file, include_nested).ok()
            })
            .collect();

        for classes in results {
            all_classes.extend(classes);
        }
    }

    if all_classes.is_empty() {
        println!("No classes/structs found");
    } else {
        println!("Found {} classes/structs:", all_classes.len());
        output.write_symbols(&all_classes)?;
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