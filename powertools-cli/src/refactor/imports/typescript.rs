use super::{ImportAnalyzer, ImportKind, ImportLocation, ImportStatement};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Parser};

pub struct TypeScriptImportAnalyzer;

impl TypeScriptImportAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn create_parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .expect("Failed to load TypeScript grammar");
        parser
    }

    fn extract_import_from_node(&self, node: Node, source: &str) -> Option<ImportStatement> {
        let kind = node.kind();

        match kind {
            "import_statement" => self.extract_import_statement(node, source),
            "import_clause" => None, // Handled by import_statement
            _ => None,
        }
    }

    fn extract_import_statement(&self, node: Node, source: &str) -> Option<ImportStatement> {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        // Find the string literal (source)
        let source_node = children.iter().find(|n| n.kind() == "string")?;
        let source_text = source[source_node.byte_range()].trim_matches(|c| c == '"' || c == '\'');

        // Find import clause or named imports
        let mut symbols = Vec::new();
        let mut kind = ImportKind::SideEffect;
        let mut alias = None;

        for child in &children {
            match child.kind() {
                "import_clause" => {
                    let (parsed_symbols, parsed_kind, parsed_alias) =
                        self.extract_import_clause(*child, source);
                    symbols = parsed_symbols;
                    kind = parsed_kind;
                    alias = parsed_alias;
                }
                "identifier" if symbols.is_empty() => {
                    // Default import
                    symbols.push(source[child.byte_range()].to_string());
                    kind = ImportKind::Default;
                }
                _ => {}
            }
        }

        let location = ImportLocation {
            line: node.start_position().row + 1,
            column: node.start_position().column,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column,
        };

        Some(ImportStatement {
            source: source_text.to_string(),
            symbols,
            location,
            kind,
            alias,
        })
    }

    fn extract_import_clause(
        &self,
        node: Node,
        source: &str,
    ) -> (Vec<String>, ImportKind, Option<String>) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        for child in children {
            match child.kind() {
                "named_imports" => {
                    let symbols = self.extract_named_imports(child, source);
                    return (symbols, ImportKind::Named, None);
                }
                "namespace_import" => {
                    let (symbol, alias) = self.extract_namespace_import(child, source);
                    return (vec![symbol], ImportKind::Namespace, alias);
                }
                "identifier" => {
                    // Default import
                    return (
                        vec![source[child.byte_range()].to_string()],
                        ImportKind::Default,
                        None,
                    );
                }
                _ => {}
            }
        }

        (Vec::new(), ImportKind::SideEffect, None)
    }

    fn extract_named_imports(&self, node: Node, source: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "import_specifier" {
                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    if spec_child.kind() == "identifier" {
                        symbols.push(source[spec_child.byte_range()].to_string());
                        break; // Only take the first identifier (not the alias)
                    }
                }
            }
        }

        symbols
    }

    fn extract_namespace_import(&self, node: Node, source: &str) -> (String, Option<String>) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        // Format: * as Name
        for (i, child) in children.iter().enumerate() {
            if child.kind() == "identifier" && i > 0 {
                let name = source[child.byte_range()].to_string();
                return ("*".to_string(), Some(name.clone()));
            }
        }

        ("*".to_string(), None)
    }
}

impl ImportAnalyzer for TypeScriptImportAnalyzer {
    fn find_imports(&self, file: &Path) -> Result<Vec<ImportStatement>> {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let mut parser = Self::create_parser();
        let tree = parser
            .parse(&content, None)
            .context("Failed to parse TypeScript file")?;

        let root = tree.root_node();
        let mut imports = Vec::new();

        // Traverse the tree to find import statements
        let mut visit_stack = vec![root];

        while let Some(node) = visit_stack.pop() {
            if node.kind() == "import_statement" {
                if let Some(import) = self.extract_import_from_node(node, &content) {
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

        // Generate the import statement
        let import_line = match import.kind {
            ImportKind::Named => {
                let symbols = import.symbols.join(", ");
                format!("import {{ {} }} from '{}';\n", symbols, import.source)
            }
            ImportKind::Default => {
                format!("import {} from '{}';\n", import.symbols[0], import.source)
            }
            ImportKind::Namespace => {
                if let Some(alias) = &import.alias {
                    format!("import * as {} from '{}';\n", alias, import.source)
                } else {
                    format!("import * from '{}';\n", import.source)
                }
            }
            ImportKind::SideEffect => {
                format!("import '{}';\n", import.source)
            }
            _ => {
                anyhow::bail!("Unsupported import kind for TypeScript: {:?}", import.kind);
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
            if import.symbols.contains(&symbol.to_string()) {
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

        // Simple string replacement for now
        // TODO: Use AST-based replacement for more precision
        let new_content = content.replace(
            &format!("'{}'", old_path),
            &format!("'{}'", new_path),
        );
        let new_content = new_content.replace(
            &format!("\"{}\"", old_path),
            &format!("\"{}\"", new_path),
        );

        Ok(new_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_find_named_imports() {
        let code = r#"
import { useState, useEffect } from 'react';
import { Button } from './components';
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = TypeScriptImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "react");
        assert_eq!(imports[0].symbols, vec!["useState", "useEffect"]);
        assert_eq!(imports[0].kind, ImportKind::Named);
    }

    #[test]
    fn test_find_default_import() {
        let code = r#"
import React from 'react';
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = TypeScriptImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "react");
        assert_eq!(imports[0].symbols, vec!["React"]);
        assert_eq!(imports[0].kind, ImportKind::Default);
    }

    #[test]
    fn test_find_namespace_import() {
        let code = r#"
import * as React from 'react';
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();

        let analyzer = TypeScriptImportAnalyzer::new();
        let imports = analyzer.find_imports(file.path()).unwrap();

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "react");
        assert_eq!(imports[0].kind, ImportKind::Namespace);
        assert_eq!(imports[0].alias, Some("React".to_string()));
    }
}
