use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor, Node};
use crate::core::{Language, Location, SearchResult};

pub struct TreeSitterAnalyzer {
    parser: Parser,
}

impl TreeSitterAnalyzer {
    pub fn new() -> Result<Self> {
        let parser = Parser::new();
        Ok(Self { parser })
    }

    pub fn analyze_file(&mut self, file_path: &Path) -> Result<AnalyzedFile> {
        let content = fs::read_to_string(file_path)?;
        let language = self.detect_language(file_path)?;

        let tree_sitter_lang = language.tree_sitter_language()
            .ok_or_else(|| anyhow!("Unsupported language: {:?}", language))?;

        self.parser.set_language(&tree_sitter_lang)?;
        let tree = self.parser.parse(&content, None)
            .ok_or_else(|| anyhow!("Failed to parse file"))?;

        Ok(AnalyzedFile {
            path: file_path.to_path_buf(),
            content,
            tree,
            language,
        })
    }

    pub fn search_pattern(
        &mut self,
        file_path: &Path,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<SearchResult>> {
        let analyzed = self.analyze_file(file_path)?;
        let query = Query::new(&analyzed.language.tree_sitter_language().unwrap(), pattern)
            .map_err(|e| anyhow!("Invalid query pattern: {}", e))?;

        let mut query_cursor = QueryCursor::new();
        let matches = query_cursor.matches(&query, analyzed.tree.root_node(), analyzed.content.as_bytes());

        let mut results = Vec::new();
        for m in matches.take(max_results) {
            for capture in m.captures {
                let node = capture.node;
                let start = node.start_position();
                let end = node.end_position();

                let matched_text = &analyzed.content[node.byte_range()];

                // Get context lines
                let lines: Vec<&str> = analyzed.content.lines().collect();
                let context_before = if start.row > 0 {
                    lines.get(start.row - 1).map(|s| s.to_string())
                } else {
                    None
                };
                let context_after = lines.get(end.row + 1).map(|s| s.to_string());

                results.push(SearchResult {
                    location: Location {
                        file_path: file_path.to_path_buf(),
                        line: start.row + 1, // Convert to 1-indexed
                        column: start.column + 1,
                        end_line: Some(end.row + 1),
                        end_column: Some(end.column + 1),
                    },
                    matched_text: matched_text.to_string(),
                    context_before,
                    context_after,
                    language: analyzed.language,
                    node_type: node.kind().to_string(),
                });
            }
        }

        Ok(results)
    }

    pub fn find_functions(&mut self, file_path: &Path) -> Result<Vec<FunctionInfo>> {
        let analyzed = self.analyze_file(file_path)?;
        let query_str = match analyzed.language {
            Language::Rust => r#"
                (function_item name: (identifier) @name) @func
            "#,
            Language::TypeScript | Language::JavaScript => r#"
                (function_declaration name: (identifier) @name) @func
                (method_definition key: (property_identifier) @name) @func
                (arrow_function) @func
            "#,
            Language::Python => r#"
                (function_definition name: (identifier) @name) @func
            "#,
            Language::Go => r#"
                (function_declaration name: (identifier) @name) @func
                (method_declaration name: (field_identifier) @name) @func
            "#,
            _ => return Ok(Vec::new()),
        };

        let query = Query::new(&analyzed.language.tree_sitter_language().unwrap(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, analyzed.tree.root_node(), analyzed.content.as_bytes());

        let mut functions = Vec::new();
        for m in matches {
            let func_node = m.captures.iter().find(|c| query.capture_names()[c.index as usize] == "func")
                .map(|c| c.node);
            let name_node = m.captures.iter().find(|c| query.capture_names()[c.index as usize] == "name")
                .map(|c| c.node);

            if let Some(func_node) = func_node {
                let name = name_node.map(|n| analyzed.content[n.byte_range()].to_string())
                    .unwrap_or_else(|| "<anonymous>".to_string());

                let start = func_node.start_position();
                functions.push(FunctionInfo {
                    name,
                    location: Location {
                        file_path: file_path.to_path_buf(),
                        line: start.row + 1,
                        column: start.column + 1,
                        end_line: None,
                        end_column: None,
                    },
                    is_public: self.is_public_function(&func_node, &analyzed),
                    parameters: self.extract_parameters(&func_node, &analyzed),
                    return_type: self.extract_return_type(&func_node, &analyzed),
                });
            }
        }

        Ok(functions)
    }

    fn detect_language(&self, file_path: &Path) -> Result<Language> {
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow!("No file extension"))?;
        Ok(Language::from_extension(ext))
    }

    fn is_public_function(&self, node: &Node, analyzed: &AnalyzedFile) -> bool {
        // Simple heuristic for now - can be improved per language
        match analyzed.language {
            Language::Rust => {
                // Check if there's a "pub" keyword
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "visibility_modifier" {
                        return true;
                    }
                }
                false
            }
            Language::Python => {
                // Python convention: functions starting with _ are private
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &analyzed.content[name_node.byte_range()];
                    !name.starts_with('_')
                } else {
                    true
                }
            }
            _ => true, // Default to public for other languages
        }
    }

    fn extract_parameters(&self, node: &Node, analyzed: &AnalyzedFile) -> Vec<String> {
        // Simplified parameter extraction
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind().contains("parameter") || child.kind() == "identifier" {
                    let param_text = &analyzed.content[child.byte_range()];
                    if !param_text.is_empty() && param_text != "," && param_text != "(" && param_text != ")" {
                        params.push(param_text.to_string());
                    }
                }
            }
        }

        params
    }

    fn extract_return_type(&self, node: &Node, analyzed: &AnalyzedFile) -> Option<String> {
        // Simplified return type extraction
        node.child_by_field_name("return_type")
            .map(|n| analyzed.content[n.byte_range()].to_string())
    }
}

pub struct AnalyzedFile {
    #[allow(dead_code)]
    pub path: std::path::PathBuf,
    pub content: String,
    pub tree: tree_sitter::Tree,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub location: Location,
    pub is_public: bool,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}