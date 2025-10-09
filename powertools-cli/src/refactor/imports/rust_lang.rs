use super::{ImportAnalyzer, ImportKind, ImportLocation, ImportStatement};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use syn::{File, Item, UseTree};

pub struct RustImportAnalyzer;

impl RustImportAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn extract_use_tree(
        &self,
        tree: &UseTree,
        prefix: String,
        imports: &mut Vec<ImportStatement>,
        line: usize,
    ) {
        match tree {
            UseTree::Path(path) => {
                let new_prefix = if prefix.is_empty() {
                    path.ident.to_string()
                } else {
                    format!("{}::{}", prefix, path.ident)
                };
                self.extract_use_tree(&path.tree, new_prefix, imports, line);
            }
            UseTree::Name(name) => {
                imports.push(ImportStatement {
                    source: prefix.clone(),
                    symbols: vec![name.ident.to_string()],
                    location: ImportLocation {
                        line,
                        column: 0,
                        end_line: line,
                        end_column: 0,
                    },
                    kind: ImportKind::Use,
                    alias: None,
                });
            }
            UseTree::Rename(rename) => {
                imports.push(ImportStatement {
                    source: prefix.clone(),
                    symbols: vec![rename.ident.to_string()],
                    location: ImportLocation {
                        line,
                        column: 0,
                        end_line: line,
                        end_column: 0,
                    },
                    kind: ImportKind::Use,
                    alias: Some(rename.rename.to_string()),
                });
            }
            UseTree::Glob(_) => {
                imports.push(ImportStatement {
                    source: prefix.clone(),
                    symbols: vec!["*".to_string()],
                    location: ImportLocation {
                        line,
                        column: 0,
                        end_line: line,
                        end_column: 0,
                    },
                    kind: ImportKind::Use,
                    alias: None,
                });
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    self.extract_use_tree(item, prefix.clone(), imports, line);
                }
            }
        }
    }
}

impl ImportAnalyzer for RustImportAnalyzer {
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let ast: File = syn::parse_str(&content)
            .with_context(|| format!("Failed to parse Rust file: {}", file.display()))?;

        let mut imports = Vec::new();
        let mut line = 1;

        for item in ast.items {
            if let Item::Use(use_item) = item {
                self.extract_use_tree(&use_item.tree, String::new(), &mut imports, line);
            }
            // Approximate line counting (not precise, but good enough for now)
            line += 1;
        }

        Ok(imports)
    }

    fn add_import(&self, file: &Path, import: &ImportStatement) -> Result<String> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        // Generate the use statement
        let use_line = if !import.source.is_empty() && !import.symbols.is_empty() {
            if import.symbols[0] == "*" {
                format!("use {}::*;\n", import.source)
            } else if let Some(alias) = &import.alias {
                format!(
                    "use {}::{} as {};\n",
                    import.source, import.symbols[0], alias
                )
            } else {
                format!("use {}::{};\n", import.source, import.symbols[0])
            }
        } else {
            format!("use {};\n", import.source)
        };

        // Find the position to insert (after the last use statement, or at the start)
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
        new_content.insert_str(insert_pos, &use_line);

        Ok(new_content)
    }

    fn remove_import(&self, file: &Path, symbol: &str) -> Result<String> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let imports = self.find_imports(file)?;
        let mut lines: Vec<&str> = content.lines().collect();

        for import in imports {
            // Check if this import contains the symbol we want to remove
            if import.symbols.contains(&symbol.to_string()) || import.source.ends_with(symbol) {
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
            if import.source == old_path || import.source.starts_with(&format!("{}::", old_path)) {
                // Replace the source path
                let lines: Vec<&str> = content.lines().collect();
                if import.location.line > 0 && import.location.line <= lines.len() {
                    let old_line = lines[import.location.line - 1];
                    let new_line = old_line.replace(old_path, new_path);
                    new_content = new_content.replace(old_line, &new_line);
                }
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
    fn test_find_simple_use() {
        let code = r#"
use std::collections::HashMap;
use std::fs;
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = RustImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "std::collections");
        assert_eq!(imports[0].symbols, vec!["HashMap"]);
        assert_eq!(imports[0].kind, ImportKind::Use);
    }

    #[test]
    fn test_find_use_with_alias() {
        let code = r#"
use std::collections::HashMap as Map;
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = RustImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "std::collections");
        assert_eq!(imports[0].symbols, vec!["HashMap"]);
        assert_eq!(imports[0].alias, Some("Map".to_string()));
    }

    #[test]
    fn test_find_glob_use() {
        let code = r#"
use std::collections::*;
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = RustImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "std::collections");
        assert_eq!(imports[0].symbols, vec!["*"]);
    }

    #[test]
    fn test_find_grouped_use() {
        let code = r#"
use std::collections::{HashMap, HashSet};
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = RustImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "std::collections");
        assert_eq!(imports[0].symbols, vec!["HashMap"]);
        assert_eq!(imports[1].symbols, vec!["HashSet"]);
    }
}
