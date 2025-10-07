use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::core::output::OutputWriter;
use crate::analyzers::PatternMatcher;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn run(
    pattern: String,
    path: Option<PathBuf>,
    extensions: Vec<String>,
    max_results: usize,
    format: &crate::OutputFormat,
) -> Result<()> {
    let search_path = path.unwrap_or_else(|| PathBuf::from("."));
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

    let mut matcher = PatternMatcher::new()?;

    let results = if search_path.is_file() {
        // Search single file
        spinner.set_message(format!("Searching in {}", search_path.display()));
        matcher.search_file(&search_path, &pattern, max_results)?
    } else {
        // Search directory
        spinner.set_message(format!("Searching in directory: {}", search_path.display()));
        matcher.search_directory(&search_path, &pattern, extensions, max_results)?
    };

    spinner.finish_and_clear();

    if results.is_empty() {
        println!("No matches found for pattern: {}", pattern);
    } else {
        println!("Found {} matches:", results.len());
        output.write_search_results(&results)?;
    }

    Ok(())
}