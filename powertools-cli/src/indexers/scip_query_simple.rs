use anyhow::{anyhow, Result};
use scip::types::Index;
use std::path::{Path, PathBuf};
use crate::core::{Location, Reference, ReferenceKind};

/// SCIP query implementation
pub struct ScipQuery {
    index: Index,
    project_root: PathBuf,
}

impl ScipQuery {
    pub fn new(index: Index, project_root: PathBuf) -> Self {
        Self { index, project_root }
    }

    /// Find the definition of a symbol at a given location
    pub fn find_definition(&self, file_path: &Path, line: usize, column: usize) -> Result<Option<Location>> {
        // Make file_path relative to project_root
        let relative_path = if file_path.is_absolute() {
            file_path.strip_prefix(&self.project_root)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string()
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Find the document
        let document = self.index.documents.iter()
            .find(|doc| doc.relative_path == relative_path);

        let doc = match document {
            Some(d) => d,
            None => return Ok(None),
        };

        // Find occurrence at the given position
        // SCIP uses 0-based indexing
        let target_line = line.saturating_sub(1) as i32;
        let target_col = column as i32;

        let occurrence = doc.occurrences.iter().find(|occ| {
            if occ.range.len() >= 3 {
                let start_line = occ.range[0];
                let start_col = occ.range[1];
                let end_col = occ.range[2];

                start_line == target_line && target_col >= start_col && target_col < end_col
            } else {
                false
            }
        });

        let occ = match occurrence {
            Some(o) => o,
            None => return Ok(None),
        };

        // Get the symbol from this occurrence
        let symbol = &occ.symbol;

        // Symbol roles: 1 = Definition, 2 = Import, 4 = Write, 8 = Read
        const DEFINITION_ROLE: i32 = 1;

        // If this occurrence is already a definition, return it
        if occ.symbol_roles & DEFINITION_ROLE != 0 {
            return Ok(Some(Location {
                file_path: self.project_root.join(&relative_path),
                line: (occ.range[0] as usize) + 1,
                column: occ.range[1] as usize,
                end_line: Some((occ.range.get(3).unwrap_or(&occ.range[0]) + 1) as usize),
                end_column: Some(*occ.range.get(4).unwrap_or(&occ.range[2]) as usize),
            }));
        }

        // Otherwise, search for the definition of this symbol
        for document in &self.index.documents {
            for occurrence in &document.occurrences {
                if occurrence.symbol == *symbol && (occurrence.symbol_roles & DEFINITION_ROLE != 0) {
                    return Ok(Some(Location {
                        file_path: self.project_root.join(&document.relative_path),
                        line: (occurrence.range[0] as usize) + 1,
                        column: occurrence.range[1] as usize,
                        end_line: Some((occurrence.range.get(3).unwrap_or(&occurrence.range[0]) + 1) as usize),
                        end_column: Some(*occurrence.range.get(4).unwrap_or(&occurrence.range[2]) as usize),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol_name: &str, include_declarations: bool) -> Result<Vec<Reference>> {
        let mut references = Vec::new();

        const DEFINITION_ROLE: i32 = 1;

        for document in &self.index.documents {
            for occurrence in &document.occurrences {
                // Simple substring match for now - could be enhanced to parse SCIP symbols
                if occurrence.symbol.contains(symbol_name) {
                    let is_definition = occurrence.symbol_roles & DEFINITION_ROLE != 0;

                    if !is_definition || include_declarations {
                        if occurrence.range.len() >= 3 {
                            references.push(Reference {
                                location: Location {
                                    file_path: self.project_root.join(&document.relative_path),
                                    line: (occurrence.range[0] as usize) + 1,
                                    column: occurrence.range[1] as usize,
                                    end_line: Some((occurrence.range.get(3).unwrap_or(&occurrence.range[0]) + 1) as usize),
                                    end_column: Some(*occurrence.range.get(4).unwrap_or(&occurrence.range[2]) as usize),
                                },
                                kind: if is_definition { ReferenceKind::Definition } else { ReferenceKind::Reference },
                                context: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(references)
    }

    pub fn _get_index(&self) -> &Index {
        &self.index
    }
}
