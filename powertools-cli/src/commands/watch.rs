use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

use crate::watcher::FileWatcher;

pub async fn run(
    path: Option<PathBuf>,
    debounce_secs: u64,
    auto_install: bool,
) -> Result<()> {
    let watch_path = path.unwrap_or_else(|| PathBuf::from("."));
    let canonical_path = watch_path.canonicalize()?;

    println!("🔭 Starting file watcher for: {}", canonical_path.display());
    println!("   Debounce delay: {}s", debounce_secs);
    println!("   Press Ctrl+C to stop\n");

    let mut watcher = FileWatcher::new(canonical_path.clone())?;
    watcher
        .start(Duration::from_secs(debounce_secs), auto_install)
        .await?;

    info!("File watcher started, monitoring for changes...");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    println!("\n\nStopping file watcher...");
    watcher.stop();
    println!("✓ File watcher stopped");

    Ok(())
}
