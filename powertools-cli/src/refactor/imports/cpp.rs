use super::{ImportAnalyzer, ImportKind, ImportLocation, ImportStatement};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Parser};

pub struct CppImportAnalyzer;

impl CppImportAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn create_parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .expect("Failed to load C++ grammar");
        parser
    }

    fn extract_include_from_node(&self, node: Node, source: &str) -> Option<ImportStatement> {
        if node.kind() != "preproc_include" {
            return None;
        }

        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        // Find the path node
        for child in children {
            let kind = child.kind();
            if kind == "system_lib_string" || kind == "string_literal" {
                let path_text = source[child.byte_range()]
                    .trim_matches(|c| c == '<' || c == '>' || c == '"');

                let location = ImportLocation {
                    line: node.start_position().row + 1,
                    column: node.start_position().column,
                    end_line: node.end_position().row + 1,
                    end_column: node.end_position().column,
                };

                return Some(ImportStatement {
                    source: path_text.to_string(),
                    symbols: Vec::new(), // C++ includes don't have explicit symbols
                    location,
                    kind: ImportKind::Include,
                    alias: None,
                });
            }
        }

        None
    }
}

impl ImportAnalyzer for CppImportAnalyzer {
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let mut parser = Self::create_parser();
        let tree = parser
            .parse(&content, None)
            .context("Failed to parse C++ file")?;

        let root = tree.root_node();
        let mut imports = Vec::new();

        // Traverse the tree to find #include directives
        let mut visit_stack = vec![root];

        while let Some(node) = visit_stack.pop() {
            if node.kind() == "preproc_include" {
                if let Some(import) = self.extract_include_from_node(node, &content) {
                    imports.push(import);
                }
            }

            // Add children to stack for traversal
            let mut child_cursor = node.walk();
            for child in node.children(&mut child_cursor) {
                visit_stack.push(child);
            }
        }

        Ok(imports)
    }

    fn add_import(&self, file: &Path, import: &ImportStatement) -> Result<String> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        // Determine if it's a system or local include
        let include_line = if import.source.contains('/') && !import.source.starts_with("std") {
            // Local include
            format!("#include \"{}\"\n", import.source)
        } else {
            // System include
            format!("#include <{}>\n", import.source)
        };

        // Find the position to insert (after the last #include, or at the start)
        let existing_imports = self.find_imports(file)?;
        let insert_pos = if let Some(last_import) = existing_imports.last() {
            content
                .lines()
                .take(last_import.location.end_line)
                .map(|line| line.len() + 1)
                .sum::<usize>()
        } else {
            0
        };

        let mut new_content = content.clone();
        new_content.insert_str(insert_pos, &include_line);

        Ok(new_content)
    }

    fn remove_import(&self, file: &Path, symbol: &str) -> Result<String> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let imports = self.find_imports(file)?;
        let mut lines: Vec<&str> = content.lines().collect();

        for import in imports {
            if import.source == symbol || import.source.contains(symbol) {
                for line_num in import.location.line..=import.location.end_line {
                    if line_num > 0 && line_num <= lines.len() {
                        lines[line_num - 1] = "";
                    }
                }
            }
        }

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

        // Simple string replacement
        let new_content = content
            .replace(&format!("<{}>", old_path), &format!("<{}>", new_path))
            .replace(&format!("\"{}\"", old_path), &format!("\"{}\"", new_path));

        Ok(new_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_find_system_includes() {
        let code = r#"
#include <vector>
#include <string>
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = CppImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "vector");
        assert_eq!(imports[0].kind, ImportKind::Include);
        assert_eq!(imports[1].source, "string");
    }

    #[test]
    fn test_find_local_includes() {
        let code = r#"
#include "header.h"
#include "utils/helpers.h"
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = CppImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "header.h");
        assert_eq!(imports[1].source, "utils/helpers.h");
    }
}
