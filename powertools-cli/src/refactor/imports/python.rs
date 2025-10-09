use super::{ImportAnalyzer, ImportKind, ImportLocation, ImportStatement};
use anyhow::{Context, Result};
use rustpython_parser::{ast, Parse};
use std::fs;
use std::path::Path;

pub struct PythonImportAnalyzer;

impl PythonImportAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn extract_import(&self, stmt: &ast::StmtImport) -> Vec<ImportStatement> {
        stmt.names
            .iter()
            .map(|alias| {
                let source = alias.name.to_string();
                let alias_name = alias.asname.as_ref().map(|a| a.to_string());

                ImportStatement {
                    source: source.clone(),
                    symbols: vec![source],
                    location: ImportLocation {
                        line: 0, // TODO: Get proper location from rustpython
                        column: 0,
                        end_line: 0,
                        end_column: 0,
                    },
                    kind: ImportKind::SimpleImport,
                    alias: alias_name,
                }
            })
            .collect()
    }

    fn extract_import_from(&self, stmt: &ast::StmtImportFrom) -> ImportStatement {
        let source = stmt
            .module
            .as_ref()
            .map(|m| m.to_string())
            .unwrap_or_else(|| ".".to_string());

        let symbols: Vec<String> = stmt
            .names
            .iter()
            .map(|alias| alias.name.to_string())
            .collect();

        // Check if it's a wildcard import
        let is_wildcard = symbols.iter().any(|s| s == "*");

        ImportStatement {
            source,
            symbols,
            location: ImportLocation {
                line: 0, // TODO: Get proper location from rustpython
                column: 0,
                end_line: 0,
                end_column: 0,
            },
            kind: if is_wildcard {
                ImportKind::Namespace
            } else {
                ImportKind::FromImport
            },
            alias: None,
        }
    }
}

impl ImportAnalyzer for PythonImportAnalyzer {
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let module = ast::Suite::parse(&content, "<string>")
            .with_context(|| format!("Failed to parse Python file: {}", file.display()))?;

        let mut imports = Vec::new();

        for stmt in &module {
            match stmt {
                ast::Stmt::Import(import_stmt) => {
                    imports.extend(self.extract_import(import_stmt));
                }
                ast::Stmt::ImportFrom(from_stmt) => {
                    imports.push(self.extract_import_from(from_stmt));
                }
                _ => {}
            }
        }

        Ok(imports)
    }

    fn add_import(&self, file: &Path, import: &ImportStatement) -> Result<String> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        // Generate the import statement
        let import_line = match import.kind {
            ImportKind::SimpleImport => {
                if let Some(alias) = &import.alias {
                    format!("import {} as {}\n", import.source, alias)
                } else {
                    format!("import {}\n", import.source)
                }
            }
            ImportKind::FromImport => {
                let symbols = import.symbols.join(", ");
                format!("from {} import {}\n", import.source, symbols)
            }
            ImportKind::Namespace => {
                format!("from {} import *\n", import.source)
            }
            _ => {
                anyhow::bail!("Unsupported import kind for Python: {:?}", import.kind);
            }
        };

        // Find the position to insert (after the last import, or at the start)
        let existing_imports = self.find_imports(file)?;
        let insert_pos = if let Some(last_import) = existing_imports.last() {
            // Find the byte offset for the end of the last import
            content
                .lines()
                .take(last_import.location.end_line)
                .map(|line| line.len() + 1) // +1 for newline
                .sum::<usize>()
        } else {
            0
        };

        let mut new_content = content.clone();
        new_content.insert_str(insert_pos, &import_line);

        Ok(new_content)
    }

    fn remove_import(&self, file: &Path, symbol: &str) -> Result<String> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let imports = self.find_imports(file)?;
        let mut lines: Vec<&str> = content.lines().collect();

        for import in imports {
            // Check if this import contains the symbol we want to remove
            if import.symbols.contains(&symbol.to_string()) || import.source == symbol {
                // Remove the line(s) containing this import
                for line_num in import.location.line..=import.location.end_line {
                    if line_num > 0 && line_num <= lines.len() {
                        lines[line_num - 1] = "";
                    }
                }
            }
        }

        // Filter out empty lines and rejoin
        let new_content = lines
            .into_iter()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(new_content + "\n")
    }

    fn update_import_path(&self, file: &Path, old_path: &str, new_path: &str) -> Result<String> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let imports = self.find_imports(file)?;
        let mut new_content = content.clone();

        for import in imports {
            if import.source == old_path {
                // Replace the source path
                let old_line = &content.lines().nth(import.location.line - 1).unwrap();
                let new_line = old_line.replace(old_path, new_path);

                new_content = new_content.replace(old_line, &new_line);
            }
        }

        Ok(new_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_find_simple_import() {
        let code = r#"
import os
import sys
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = PythonImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "os");
        assert_eq!(imports[0].kind, ImportKind::SimpleImport);
        assert_eq!(imports[1].source, "sys");
    }

    #[test]
    fn test_find_from_import() {
        let code = r#"
from typing import List, Dict
from pathlib import Path
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = PythonImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "typing");
        assert_eq!(imports[0].symbols, vec!["List", "Dict"]);
        assert_eq!(imports[0].kind, ImportKind::FromImport);
    }

    #[test]
    fn test_find_import_with_alias() {
        let code = r#"
import numpy as np
import pandas as pd
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = PythonImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "numpy");
        assert_eq!(imports[0].alias, Some("np".to_string()));
        assert_eq!(imports[1].source, "pandas");
        assert_eq!(imports[1].alias, Some("pd".to_string()));
    }

    #[test]
    fn test_find_wildcard_import() {
        let code = r#"
from os import *
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = PythonImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "os");
        assert_eq!(imports[0].kind, ImportKind::Namespace);
    }
}
