use anyhow::Result;
use std::path::PathBuf;
use crate::core::{output::OutputWriter, SearchResult};
use crate::analyzers::PatternMatcher;
use indicatif::{ProgressBar, ProgressStyle};

/// Search for patterns and return results (for MCP/API use)
pub async fn search_patterns(
    pattern: String,
    path: Option<PathBuf>,
    extensions: Vec<String>,
    max_results: usize,
) -> Result<Vec<SearchResult>> {
    let search_path = path.unwrap_or_else(|| PathBuf::from("."));
    let mut matcher = PatternMatcher::new()?;

    let results = if search_path.is_file() {
        matcher.search_file(&search_path, &pattern, max_results)?
    } else {
        matcher.search_directory(&search_path, &pattern, extensions, max_results)?
    };

    Ok(results)
}

pub async fn run(
    pattern: String,
    path: Option<PathBuf>,
    extensions: Vec<String>,
    max_results: usize,
    format: &crate::OutputFormat,
) -> Result<()> {
    let search_path = path.clone().unwrap_or_else(|| PathBuf::from("."));
    let output = OutputWriter::new(format);

    // Create progress bar for better UX
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.set_message("Searching for pattern...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    if search_path.is_file() {
        spinner.set_message(format!("Searching in {}", search_path.display()));
    } else {
        spinner.set_message(format!("Searching in directory: {}", search_path.display()));
    }

    let results = search_patterns(pattern.clone(), path, extensions, max_results).await?;

    spinner.finish_and_clear();

    if results.is_empty() {
        println!("No matches found for pattern: {}", pattern);
    } else {
        println!("Found {} matches:", results.len());
        output.write_search_results(&results)?;
    }

    Ok(())
}