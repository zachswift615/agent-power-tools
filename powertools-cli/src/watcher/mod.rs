mod filters;
mod metadata;

pub use filters::{detect_language_from_path, is_relevant_file};
pub use metadata::IndexMetadata;

use anyhow::{Context, Result};
use notify_debouncer_full::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode, Watcher},
    DebounceEventResult, Debouncer, FileIdMap,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::core::Language;
use crate::indexers::ScipIndexer;

/// File watcher that triggers automatic re-indexing
pub struct FileWatcher {
    project_root: PathBuf,
    debouncer: Option<Debouncer<RecommendedWatcher, FileIdMap>>,
    is_running: Arc<AtomicBool>,
    reindex_tx: mpsc::UnboundedSender<Language>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let (reindex_tx, _reindex_rx) = mpsc::unbounded_channel();

        Ok(Self {
            project_root,
            debouncer: None,
            is_running: Arc::new(AtomicBool::new(false)),
            reindex_tx,
        })
    }

    /// Start watching for file changes
    pub async fn start(
        &mut self,
        debounce_duration: Duration,
        auto_install: bool,
    ) -> Result<()> {
        if self.is_running.load(Ordering::Relaxed) {
            warn!("File watcher already running");
            return Ok(());
        }

        info!(
            "Starting file watcher for {} with {:?} debounce",
            self.project_root.display(),
            debounce_duration
        );

        let (reindex_tx, mut reindex_rx) = mpsc::unbounded_channel();
        self.reindex_tx = reindex_tx.clone();

        let project_root = self.project_root.clone();
        let is_running = self.is_running.clone();

        // Create the debounced file watcher
        let mut debouncer = new_debouncer(
            debounce_duration,
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        for event in events {
                            for path in &event.paths {
                                if is_relevant_file(path) {
                                    if let Some(lang) = detect_language_from_path(path) {
                                        debug!("File change detected: {} ({:?})", path.display(), lang);
                                        if let Err(e) = reindex_tx.send(lang) {
                                            error!("Failed to send reindex request: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        for e in errors {
                            error!("Watch error: {}", e);
                        }
                    }
                }
            },
        )
        .context("Failed to create file watcher")?;

        // Watch the project root recursively
        debouncer
            .watcher()
            .watch(&self.project_root, RecursiveMode::Recursive)
            .context("Failed to start watching project directory")?;

        self.debouncer = Some(debouncer);
        self.is_running.store(true, Ordering::Relaxed);

        // Spawn task to handle reindex requests
        let project_root_clone = project_root.clone();
        tokio::spawn(async move {
            let mut last_reindex: Option<(Language, std::time::Instant)> = None;
            let min_reindex_interval = Duration::from_secs(1);

            while let Some(language) = reindex_rx.recv().await {
                // Deduplicate rapid requests for the same language
                if let Some((last_lang, last_time)) = last_reindex {
                    if last_lang == language && last_time.elapsed() < min_reindex_interval {
                        debug!("Skipping duplicate reindex request for {:?}", language);
                        continue;
                    }
                }

                info!("Re-indexing {:?}...", language);
                let mut indexer = ScipIndexer::new(project_root_clone.clone());
                indexer.set_auto_install(auto_install);

                match indexer.reindex_language(language) {
                    Ok(index_path) => {
                        info!("✓ Re-indexed {:?}: {}", language, index_path.display());

                        // Generate and save metadata
                        if let Ok(metadata) = IndexMetadata::generate(&project_root_clone) {
                            if let Err(e) = metadata.save(&index_path) {
                                warn!("Failed to save metadata: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to re-index {:?}: {}", language, e);
                    }
                }

                last_reindex = Some((language, std::time::Instant::now()));
            }
        });

        Ok(())
    }

    /// Stop watching for file changes
    pub fn stop(&mut self) {
        if !self.is_running.load(Ordering::Relaxed) {
            warn!("File watcher not running");
            return;
        }

        info!("Stopping file watcher");
        self.debouncer = None;
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// Check if the watcher is currently running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Get watcher status information
    pub fn status(&self) -> WatcherStatus {
        WatcherStatus {
            is_running: self.is_running(),
            project_root: self.project_root.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WatcherStatus {
    pub is_running: bool,
    pub project_root: PathBuf,
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        if self.is_running() {
            self.stop();
        }
    }
}
