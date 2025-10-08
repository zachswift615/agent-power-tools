use anyhow::{anyhow, Result};
use scip::types::{Index, Document, Occurrence, SymbolInformation};
use std::path::{Path, PathBuf};
use crate::core::{Location, Symbol, SymbolKind, Reference, ReferenceKind};

/// Query SCIP index for semantic information
pub struct ScipQuery {
    index: Index,
    project_root: PathBuf,
}

impl ScipQuery {
    #[allow(dead_code)]
    pub fn new(index: Index, project_root: PathBuf) -> Self {
        Self { index, project_root }
    }

    /// Find the definition of a symbol at a given location
    pub fn find_definition(&self, file_path: &Path, line: usize, column: usize) -> Result<Option<Location>> {
        let relative_path = self.make_relative(file_path)?;

        // Find the document in the index
        let document = self.find_document(&relative_path)?;

        // Find the occurrence at the given position
        let occurrence = self.find_occurrence_at_position(&document, line, column)?;

        // Get the symbol from the occurrence
        let symbol = occurrence.symbol.as_ref()
            .ok_or_else(|| anyhow!("Occurrence has no symbol"))?;

        // Find the definition of this symbol
        self.find_symbol_definition(symbol)
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol_name: &str, include_declarations: bool) -> Result<Vec<Reference>> {
        let mut references = Vec::new();

        // Search through all documents
        for document in &self.index.documents {
            for occurrence in &document.occurrences {
                if let Some(occ_symbol) = &occurrence.symbol {
                    // Check if this symbol matches what we're looking for
                    if self.symbol_matches(occ_symbol, symbol_name) {
                        // Check if we should include this occurrence
                        if !include_declarations && self.is_definition(occurrence) {
                            continue;
                        }

                        if let Some(range) = occurrence.range.as_ref() {
                            let location = self.range_to_location(&document.relative_path, range)?;
                            let kind = self.occurrence_to_reference_kind(occurrence);

                            references.push(Reference {
                                location,
                                kind,
                                context: None, // Could extract from source
                            });
                        }
                    }
                }
            }
        }

        Ok(references)
    }

    /// Find all symbols matching a query
    pub fn find_symbols(&self, query: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();

        for document in &self.index.documents {
            for symbol_info in &document.symbols {
                let symbol_name = self.extract_symbol_name(&symbol_info.symbol);

                if symbol_name.contains(query) {
                    if let Some(location) = self.symbol_info_to_location(&document.relative_path, symbol_info)? {
                        symbols.push(Symbol {
                            name: symbol_name,
                            kind: self.scip_kind_to_symbol_kind(symbol_info),
                            location,
                            container: None,
                            signature: None,
                            documentation: symbol_info.documentation.iter().next().cloned(),
                        });
                    }
                }
            }
        }

        Ok(symbols)
    }

    fn find_document(&self, relative_path: &str) -> Result<&Document> {
        self.index.documents.iter()
            .find(|doc| doc.relative_path == relative_path)
            .ok_or_else(|| anyhow!("Document not found in index: {}", relative_path))
    }

    fn find_occurrence_at_position(&self, document: &Document, line: usize, column: usize) -> Result<&Occurrence> {
        // SCIP uses 0-indexed lines and columns
        let target_line = (line - 1) as i32;
        let target_col = (column - 1) as i32;

        for occurrence in &document.occurrences {
            if let Some(range) = &occurrence.range {
                // Range format: [start_line, start_col, end_line, end_col]
                if range.len() >= 4 {
                    let start_line = range[0];
                    let start_col = range[1];
                    let end_line = range[2];
                    let end_col = range[3];

                    if (target_line > start_line || (target_line == start_line && target_col >= start_col))
                        && (target_line < end_line || (target_line == end_line && target_col <= end_col))
                    {
                        return Ok(occurrence);
                    }
                }
            }
        }

        Err(anyhow!("No symbol found at position {}:{}", line, column))
    }

    fn find_symbol_definition(&self, symbol: &str) -> Result<Option<Location>> {
        // Search for the definition occurrence of this symbol
        for document in &self.index.documents {
            for occurrence in &document.occurrences {
                if occurrence.symbol.as_ref() == Some(&symbol.to_string()) {
                    // Check if this is a definition
                    if self.is_definition(occurrence) {
                        if let Some(range) = &occurrence.range {
                            return Ok(Some(self.range_to_location(&document.relative_path, range)?));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn is_definition(&self, occurrence: &Occurrence) -> bool {
        // SCIP uses symbol_roles to indicate if occurrence is a definition
        // Role 1 = Definition
        occurrence.symbol_roles & 1 == 1
    }

    fn symbol_matches(&self, symbol: &str, query: &str) -> bool {
        // Extract the simple name from the SCIP symbol
        let simple_name = self.extract_symbol_name(symbol);
        simple_name.contains(query) || symbol.contains(query)
    }

    fn extract_symbol_name(&self, scip_symbol: &str) -> String {
        // SCIP symbols have format like: "scip-typescript npm <package> <version> <path> <name>"
        // We want to extract just the name
        scip_symbol.split_whitespace()
            .last()
            .unwrap_or(scip_symbol)
            .trim_matches('`')
            .to_string()
    }

    fn range_to_location(&self, relative_path: &str, range: &[i32]) -> Result<Location> {
        if range.len() < 4 {
            return Err(anyhow!("Invalid range format"));
        }

        let file_path = self.project_root.join(relative_path);

        Ok(Location {
            file_path,
            line: (range[0] + 1) as usize, // Convert to 1-indexed
            column: (range[1] + 1) as usize,
            end_line: Some((range[2] + 1) as usize),
            end_column: Some((range[3] + 1) as usize),
        })
    }

    fn symbol_info_to_location(&self, relative_path: &str, symbol_info: &SymbolInformation) -> Result<Option<Location>> {
        // SymbolInformation doesn't always have range, might need to find first occurrence
        Ok(None) // Placeholder for now
    }

    fn occurrence_to_reference_kind(&self, occurrence: &Occurrence) -> ReferenceKind {
        if self.is_definition(occurrence) {
            ReferenceKind::Definition
        } else {
            // Could check other roles for more specific kinds
            ReferenceKind::Reference
        }
    }

    fn scip_kind_to_symbol_kind(&self, _symbol_info: &SymbolInformation) -> SymbolKind {
        // SCIP has its own kind system, map to our SymbolKind
        // For now, default to Variable
        SymbolKind::Variable
    }

    fn make_relative(&self, path: &Path) -> Result<String> {
        path.strip_prefix(&self.project_root)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|_| anyhow!("Path is not within project root"))
    }
}