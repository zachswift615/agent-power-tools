use anyhow::Result;
use std::path::PathBuf;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;
use crate::indexers::ScipIndexer;

pub async fn run(
    path: Option<PathBuf>,
    _force: bool,
    languages: Vec<String>,
    auto_install: bool,
    _format: &crate::OutputFormat,
) -> Result<()> {
    let index_path = path.unwrap_or_else(|| PathBuf::from("."));

    println!("Building SCIP indexes for: {}", index_path.display());

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.set_message("Detecting project languages and running indexers...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let start = Instant::now();

    // Create SCIP indexer and generate indexes for all detected languages
    let mut indexer = ScipIndexer::new(index_path.clone());
    indexer.set_auto_install(auto_install);

    match indexer.generate_indexes(languages) {
        Ok(output_paths) => {
            spinner.finish_with_message("Indexing complete!");
            let elapsed = start.elapsed();
            println!("✓ Indexes built in {:?}", elapsed);
            for path in output_paths {
                println!("✓ Index saved to: {}", path.display());
            }
            Ok(())
        }
        Err(e) => {
            spinner.finish_with_message("Indexing failed!");
            eprintln!("Error: {}", e);
            eprintln!("\nMake sure the appropriate indexer is installed:");
            eprintln!("  TypeScript/JavaScript: npm install -g @sourcegraph/scip-typescript");
            eprintln!("  Python: npm install -g @sourcegraph/scip-python");
            eprintln!("  Rust: rustup component add rust-analyzer");
            Err(e)
        }
    }
}