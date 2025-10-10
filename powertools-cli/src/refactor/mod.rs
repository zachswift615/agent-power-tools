pub mod imports;
mod preview;
mod rename;
mod replacer;
mod transaction;

pub use preview::generate_preview;
pub use rename::{RenameOptions, SymbolRenamer};
pub use replacer::BatchReplacer;
#[allow(unused_imports)]
pub use replacer::ReplacementMode;
pub use transaction::TransactionMode;

use std::path::PathBuf;

/// Result of a batch operation
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchResult {
    /// Total files scanned
    pub files_scanned: usize,

    /// Files with matches
    pub files_matched: usize,

    /// Total replacements made
    pub replacements_made: usize,

    /// Files that were modified
    pub files_modified: Vec<PathBuf>,

    /// Errors encountered
    pub errors: Vec<String>,
}

impl BatchResult {
    pub fn new() -> Self {
        Self {
            files_scanned: 0,
            files_matched: 0,
            replacements_made: 0,
            files_modified: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_modified_file(&mut self, path: PathBuf, num_replacements: usize) {
        self.files_matched += 1;
        self.replacements_made += num_replacements;
        if num_replacements > 0 {
            self.files_modified.push(path);
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

impl Default for BatchResult {
    fn default() -> Self {
        Self::new()
    }
}
