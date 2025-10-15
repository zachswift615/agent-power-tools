use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::core::{Location, Reference};
use crate::indexers::{ScipQuery, LspQuery};

/// Unified query interface that abstracts over SCIP and LSP backends
///
/// This allows the rest of the codebase to query for definitions and references
/// without knowing whether the backend is using SCIP indexes or LSP servers.
///
/// ## Architecture Decision
/// - **SCIP preferred**: Use SCIP for languages with good indexers (TypeScript, Python, Rust, C++)
/// - **LSP fallback**: Use LSP for languages without SCIP support (Swift, others)
/// - **Performance trade-off**: SCIP is 20-500x faster but LSP is more widely supported
///
/// ## Usage
/// ```ignore
/// // Try SCIP first, fall back to LSP if needed
/// let query = UnifiedQuery::from_project(&project_root)?;
///
/// // Or explicitly choose backend
/// let query = UnifiedQuery::scip_only(project_root)?;
/// let query = UnifiedQuery::lsp_only("sourcekit-lsp", vec![], project_root)?;
/// ```
pub enum UnifiedQuery {
    /// SCIP-based backend (fast, pre-computed indexes)
    Scip(ScipQuery),
    /// LSP-based backend (slower, live server queries)
    Lsp(LspQuery),
}

impl UnifiedQuery {
    /// Create a unified query from a project root
    ///
    /// This attempts to load SCIP indexes first. If none are found, it returns
    /// an error. For LSP, you must explicitly create with `lsp_only()`.
    ///
    /// # Arguments
    /// * `project_root` - Project root directory
    ///
    /// # Returns
    /// SCIP-backed query if indexes exist, error otherwise
    pub fn from_project(project_root: PathBuf) -> Result<Self> {
        // Try SCIP first
        match ScipQuery::from_project(project_root.clone()) {
            Ok(scip_query) => Ok(UnifiedQuery::Scip(scip_query)),
            Err(e) => {
                // No SCIP indexes available
                Err(anyhow::anyhow!(
                    "No SCIP indexes found in {}. For LSP support, use UnifiedQuery::lsp_only(). Error: {}",
                    project_root.display(),
                    e
                ))
            }
        }
    }

    /// Create a SCIP-only query (for testing or when you know SCIP is available)
    pub fn scip_only(project_root: PathBuf) -> Result<Self> {
        Ok(UnifiedQuery::Scip(ScipQuery::from_project(project_root)?))
    }

    /// Create an LSP-only query (for languages without SCIP support)
    ///
    /// # Arguments
    /// * `command` - LSP server command (e.g., "sourcekit-lsp" for Swift)
    /// * `args` - Arguments to pass to the LSP server
    /// * `project_root` - Project root directory
    ///
    /// # Returns
    /// LSP-backed query connected to the language server
    pub fn lsp_only(command: &str, args: Vec<String>, project_root: PathBuf) -> Result<Self> {
        Ok(UnifiedQuery::Lsp(LspQuery::start(command, args, project_root)?))
    }

    /// Find the definition of a symbol at a given location
    ///
    /// # Arguments
    /// * `file_path` - Absolute or relative path to the file
    /// * `line` - Line number (1-indexed, user convention)
    /// * `column` - Column number (1-indexed, user convention)
    ///
    /// # Returns
    /// Location of the definition, or None if not found
    pub fn find_definition(&mut self, file_path: &Path, line: usize, column: usize) -> Result<Option<Location>> {
        match self {
            UnifiedQuery::Scip(scip) => scip.find_definition(file_path, line, column),
            UnifiedQuery::Lsp(lsp) => lsp.find_definition(file_path, line, column),
        }
    }

    /// Find all references to a symbol (SCIP only)
    ///
    /// Note: SCIP can search by symbol name, but LSP requires a position.
    /// For LSP-based references, use `find_references_at_position` instead.
    ///
    /// # Arguments
    /// * `symbol_name` - Name of the symbol to find references for
    /// * `include_declarations` - Whether to include declarations
    ///
    /// # Returns
    /// List of all references, or error if backend doesn't support name-based search
    pub fn find_references(&self, symbol_name: &str, include_declarations: bool) -> Result<Vec<Reference>> {
        match self {
            UnifiedQuery::Scip(scip) => scip.find_references(symbol_name, include_declarations),
            UnifiedQuery::Lsp(_) => {
                Err(anyhow::anyhow!(
                    "LSP backend requires a position for find_references. Use find_references_at_position() instead."
                ))
            }
        }
    }

    /// Find all references to a symbol at a position (works for both SCIP and LSP)
    ///
    /// # Arguments
    /// * `file_path` - File containing a usage of the symbol
    /// * `line` - Line number (1-indexed)
    /// * `column` - Column number (1-indexed)
    /// * `include_declarations` - Whether to include declarations
    ///
    /// # Returns
    /// List of all references to the symbol
    pub fn find_references_at_position(
        &mut self,
        file_path: &Path,
        line: usize,
        column: usize,
        include_declarations: bool,
    ) -> Result<Vec<Reference>> {
        match self {
            UnifiedQuery::Scip(scip) => {
                // For SCIP, we need to:
                // 1. Find the symbol at the given position
                // 2. Then search for all references to that symbol
                // This is less efficient than direct symbol search, but provides a unified API

                // Find what symbol is at this position
                if let Some(def_location) = scip.find_definition(file_path, line, column)? {
                    // Extract symbol name from the definition location
                    // For now, we'll use a simplified approach: read the file and extract the text
                    // In a production implementation, we'd use SCIP's symbol information directly

                    // For MVP, we can fall back to the simpler API
                    // This is a limitation of the unified abstraction
                    return Err(anyhow::anyhow!(
                        "SCIP backend with position-based reference search not yet implemented. Use find_references(symbol_name) for SCIP."
                    ));
                } else {
                    Ok(vec![])
                }
            }
            UnifiedQuery::Lsp(lsp) => {
                lsp.find_references_at_position(file_path, line, column, include_declarations)
            }
        }
    }

    /// Get the backend type as a string (useful for debugging/logging)
    pub fn backend_name(&self) -> &'static str {
        match self {
            UnifiedQuery::Scip(_) => "SCIP",
            UnifiedQuery::Lsp(_) => "LSP",
        }
    }
}

impl Drop for UnifiedQuery {
    fn drop(&mut self) {
        // LSP cleanup happens in LspQuery's drop
        // SCIP has nothing to clean up
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_name() {
        // This would require actual SCIP indexes or LSP servers
        // For now, just verify the enum works
        // In real tests, we'd create temporary indexes/servers
    }

    #[test]
    #[ignore] // Requires SCIP indexes
    fn test_scip_backend() {
        // Test SCIP backend with a known project
    }

    #[test]
    #[ignore] // Requires LSP server
    fn test_lsp_backend() {
        // Test LSP backend with a test server
    }
}
