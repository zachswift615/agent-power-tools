use anyhow::{anyhow, Result};
use scip::types::Index;
use std::path::{Path, PathBuf};
use crate::core::{Location, Reference, ReferenceKind};

/// SCIP query implementation supporting multiple language indexes
pub struct ScipQuery {
    indexes: Vec<Index>,
    project_root: PathBuf,
}

impl ScipQuery {
    /// Create from a single index (for backward compatibility)
    #[allow(dead_code)]
    pub fn new(index: Index, project_root: PathBuf) -> Self {
        Self {
            indexes: vec![index],
            project_root
        }
    }

    /// Create by loading all available language indexes from project root
    pub fn from_project(project_root: PathBuf) -> Result<Self> {
        use protobuf::Message;
        let mut indexes = Vec::new();

        // Try to load each language-specific index
        for filename in &[
            "index.typescript.scip",
            "index.javascript.scip",
            "index.python.scip",
            "index.rust.scip",
            "index.cpp.scip",
            "index.scip", // Legacy fallback
        ] {
            let path = project_root.join(filename);
            if path.exists() {
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        match Index::parse_from_bytes(&bytes) {
                            Ok(index) => {
                                if !index.documents.is_empty() {
                                    indexes.push(index);
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to parse {}: {}", filename, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read {}: {}", filename, e);
                    }
                }
            }
        }

        if indexes.is_empty() {
            return Err(anyhow!(
                "No SCIP indexes found in {}. Run 'powertools index' first",
                project_root.display()
            ));
        }

        Ok(Self { indexes, project_root })
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

        // Symbol roles: 1 = Definition, 2 = Import, 4 = Write, 8 = Read
        const DEFINITION_ROLE: i32 = 1;

        // Search across all indexes
        for index in &self.indexes {
            // Find the document
            let document = index.documents.iter()
                .find(|doc| doc.relative_path == relative_path);

            let doc = match document {
                Some(d) => d,
                None => continue, // Try next index
            };

            // Find occurrence at the given position
            // SCIP uses 0-based indexing, convert from 1-based input
            let target_line = line.saturating_sub(1) as i32;
            let target_col = column.saturating_sub(1) as i32;

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
                None => continue, // Try next index
            };

            // Get the symbol from this occurrence
            let symbol = &occ.symbol;

            // If this occurrence is already a definition, return it
            if occ.symbol_roles & DEFINITION_ROLE != 0 {
                return Ok(Some(Location {
                    file_path: self.project_root.join(&relative_path),
                    line: (occ.range[0] as usize) + 1,
                    column: (occ.range[1] as usize) + 1,
                    end_line: Some((occ.range.get(3).unwrap_or(&occ.range[0]) + 1) as usize),
                    end_column: Some((*occ.range.get(4).unwrap_or(&occ.range[2]) as usize) + 1),
                }));
            }

            // Otherwise, search for the definition of this symbol across all indexes
            for search_index in &self.indexes {
                for document in &search_index.documents {
                    for occurrence in &document.occurrences {
                        if occurrence.symbol == *symbol && (occurrence.symbol_roles & DEFINITION_ROLE != 0) {
                            return Ok(Some(Location {
                                file_path: self.project_root.join(&document.relative_path),
                                line: (occurrence.range[0] as usize) + 1,
                                column: (occurrence.range[1] as usize) + 1,
                                end_line: Some((occurrence.range.get(3).unwrap_or(&occurrence.range[0]) + 1) as usize),
                                end_column: Some((*occurrence.range.get(4).unwrap_or(&occurrence.range[2]) as usize) + 1),
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol_name: &str, include_declarations: bool) -> Result<Vec<Reference>> {
        let mut references = Vec::new();

        const DEFINITION_ROLE: i32 = 1;

        // DEBUG: Log index and document counts
        eprintln!("[DEBUG] Searching {} indexes", self.indexes.len());
        for (i, index) in self.indexes.iter().enumerate() {
            let total_docs = index.documents.len();
            let test_docs = index.documents.iter().filter(|d| d.relative_path.contains("test")).count();
            eprintln!("[DEBUG] Index {}: {} total docs, {} test docs", i, total_docs, test_docs);
        }

        // Search across all indexes
        let mut test_symbols_checked = 0;
        for index in &self.indexes {
            for document in &index.documents {
                let is_test_file = document.relative_path.contains("test");
                for occurrence in &document.occurrences {
                    // DEBUG: Count test file occurrences and sample symbols
                    if is_test_file {
                        test_symbols_checked += 1;
                        if test_symbols_checked <= 5 {
                            eprintln!("[DEBUG] Test file symbol sample: {}", occurrence.symbol);
                        }
                    }

                    // Simple substring match for now - could be enhanced to parse SCIP symbols
                    if occurrence.symbol.contains(symbol_name) {
                        // DEBUG: Log ALL Factory matches to see what they look like
                        eprintln!("[DEBUG] Factory match in {}: symbol={}", document.relative_path, occurrence.symbol);

                        // DEBUG: Log matches from test files
                        if is_test_file {
                            eprintln!("[DEBUG] *** Match in test file: {} symbol: {}", document.relative_path, occurrence.symbol);
                        }

                        let is_definition = occurrence.symbol_roles & DEFINITION_ROLE != 0;

                        if !is_definition || include_declarations {
                            if occurrence.range.len() >= 3 {
                                references.push(Reference {
                                    location: Location {
                                        file_path: self.project_root.join(&document.relative_path),
                                        line: (occurrence.range[0] as usize) + 1,
                                        column: (occurrence.range[1] as usize) + 1,
                                        end_line: Some((occurrence.range.get(3).unwrap_or(&occurrence.range[0]) + 1) as usize),
                                        end_column: Some((*occurrence.range.get(4).unwrap_or(&occurrence.range[2]) as usize) + 1),
                                    },
                                    kind: if is_definition { ReferenceKind::Definition } else { ReferenceKind::Reference },
                                    context: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        eprintln!("[DEBUG] Total test file occurrences checked: {}", test_symbols_checked);

        Ok(references)
    }
}
