use anyhow::Result;
use std::path::Path;
use tree_sitter::{Query, QueryCursor};
use crate::core::{Symbol, SymbolKind, Location, Language};
use crate::analyzers::TreeSitterAnalyzer;

pub struct ClassFinder {
    analyzer: TreeSitterAnalyzer,
}

impl ClassFinder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            analyzer: TreeSitterAnalyzer::new()?,
        })
    }

    pub fn find_in_file(&mut self, file_path: &Path, include_nested: bool) -> Result<Vec<Symbol>> {
        let analyzed = self.analyzer.analyze_file(file_path)?;

        let query_str = match analyzed.language {
            Language::Rust => r#"
                (struct_item name: (type_identifier) @name) @struct
                (enum_item name: (type_identifier) @name) @enum
                (trait_item name: (type_identifier) @name) @trait
            "#,
            Language::TypeScript | Language::JavaScript => r#"
                (class_declaration name: (identifier) @name) @class
                (interface_declaration name: (identifier) @name) @interface
            "#,
            Language::Python => r#"
                (class_definition name: (identifier) @name) @class
            "#,
            Language::Go => r#"
                (type_declaration (type_spec name: (type_identifier) @name)) @type
            "#,
            Language::Java => r#"
                (class_declaration name: (identifier) @name) @class
                (interface_declaration name: (identifier) @name) @interface
            "#,
            Language::Cpp | Language::C => r#"
                (class_specifier name: (type_identifier) @name) @class
                (struct_specifier name: (type_identifier) @name) @struct
            "#,
            _ => return Ok(Vec::new()),
        };

        let query = Query::new(&analyzed.language.tree_sitter_language().unwrap(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, analyzed.tree.root_node(), analyzed.content.as_bytes());

        let mut symbols = Vec::new();

        for m in matches {
            let name_capture = m.captures.iter()
                .find(|c| query.capture_names()[c.index as usize] == "name");

            if let Some(name_capture) = name_capture {
                let name_node = name_capture.node;
                let name = analyzed.content[name_node.byte_range()].to_string();

                // Determine the symbol kind based on the capture
                let kind = if m.captures.iter().any(|c| {
                    let capture_name = &query.capture_names()[c.index as usize];
                    *capture_name == "interface"
                }) {
                    SymbolKind::Interface
                } else if m.captures.iter().any(|c| {
                    let capture_name = &query.capture_names()[c.index as usize];
                    *capture_name == "trait"
                }) {
                    SymbolKind::Trait
                } else if m.captures.iter().any(|c| {
                    let capture_name = &query.capture_names()[c.index as usize];
                    *capture_name == "enum"
                }) {
                    SymbolKind::Enum
                } else if m.captures.iter().any(|c| {
                    let capture_name = &query.capture_names()[c.index as usize];
                    *capture_name == "struct"
                }) {
                    SymbolKind::Struct
                } else {
                    SymbolKind::Class
                };

                // Get the full node for position information
                let full_node = m.captures.iter()
                    .find(|c| {
                        let capture_name = &query.capture_names()[c.index as usize];
                        *capture_name == "class" || *capture_name == "struct" ||
                        *capture_name == "enum" || *capture_name == "trait" ||
                        *capture_name == "interface" || *capture_name == "type"
                    })
                    .map(|c| c.node);

                if let Some(node) = full_node {
                    let start = node.start_position();
                    let end = node.end_position();

                    // Check if this is a nested class
                    if !include_nested && self.is_nested(&node) {
                        continue;
                    }

                    symbols.push(Symbol {
                        name,
                        kind,
                        location: Location {
                            file_path: file_path.to_path_buf(),
                            line: start.row + 1,
                            column: start.column + 1,
                            end_line: Some(end.row + 1),
                            end_column: Some(end.column + 1),
                        },
                        container: self.get_container(&node, &analyzed),
                        signature: None,
                        documentation: None,
                    });
                }
            }
        }

        Ok(symbols)
    }

    fn is_nested(&self, node: &tree_sitter::Node) -> bool {
        // Check if this node has a parent that is also a class/struct
        let mut parent = node.parent();
        while let Some(p) = parent {
            if matches!(p.kind(), "class_declaration" | "struct_item" | "class_definition") {
                return true;
            }
            parent = p.parent();
        }
        false
    }

    fn get_container(&self, node: &tree_sitter::Node, analyzed: &crate::analyzers::AnalyzedFile) -> Option<String> {
        // Try to find the containing module/namespace
        let mut parent = node.parent();
        while let Some(p) = parent {
            if matches!(p.kind(), "module" | "namespace_declaration" | "package_declaration") {
                if let Some(name_node) = p.child_by_field_name("name") {
                    return Some(analyzed.content[name_node.byte_range()].to_string());
                }
            }
            parent = p.parent();
        }
        None
    }
}