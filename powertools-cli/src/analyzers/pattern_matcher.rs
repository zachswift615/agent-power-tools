use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use crate::core::{SearchResult, Language};
use crate::analyzers::TreeSitterAnalyzer;

pub struct PatternMatcher {
    analyzer: TreeSitterAnalyzer,
}

impl PatternMatcher {
    pub fn new() -> Result<Self> {
        Ok(Self {
            analyzer: TreeSitterAnalyzer::new()?,
        })
    }

    pub fn search_directory(
        &mut self,
        dir: &Path,
        pattern: &str,
        extensions: Vec<String>,
        max_results: usize,
    ) -> Result<Vec<SearchResult>> {
        let files = self.collect_files(dir, extensions)?;

        // Process files in parallel for better performance
        let pattern = pattern.to_string();
        let results: Vec<Vec<SearchResult>> = files
            .par_iter()
            .filter_map(|file| {
                let mut local_analyzer = TreeSitterAnalyzer::new().ok()?;
                local_analyzer.search_pattern(file, &pattern, max_results).ok()
            })
            .collect();

        // Flatten and limit results
        let mut all_results: Vec<SearchResult> = results.into_iter().flatten().collect();
        all_results.truncate(max_results);

        Ok(all_results)
    }

    pub fn search_file(
        &mut self,
        file: &Path,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<SearchResult>> {
        self.analyzer.search_pattern(file, pattern, max_results)
    }

    fn collect_files(&self, dir: &Path, extensions: Vec<String>) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(dir)
            .standard_filters(true) // Respect .gitignore
            .build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Check if file has a supported extension
                if extensions.is_empty() {
                    // No filter, check if language is supported
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if Language::from_extension(ext).tree_sitter_language().is_some() {
                            files.push(path.to_path_buf());
                        }
                    }
                } else {
                    // Filter by provided extensions
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if extensions.iter().any(|e| e.trim_start_matches('.') == ext) {
                            files.push(path.to_path_buf());
                        }
                    }
                }
            }
        }

        Ok(files)
    }
}

/// Common query patterns for different languages
#[allow(dead_code)]
pub struct QueryPatterns;

#[allow(dead_code)]
impl QueryPatterns {
    pub fn function_by_name(name: &str, language: Language) -> String {
        match language {
            Language::Rust => format!(r#"(function_item name: (identifier) @name (#eq? @name "{}")) @func"#, name),
            Language::TypeScript | Language::JavaScript => {
                format!(r#"(function_declaration name: (identifier) @name (#eq? @name "{}")) @func"#, name)
            }
            Language::Python => format!(r#"(function_definition name: (identifier) @name (#eq? @name "{}")) @func"#, name),
            Language::Go => format!(r#"(function_declaration name: (identifier) @name (#eq? @name "{}")) @func"#, name),
            _ => String::new(),
        }
    }

    pub fn class_by_name(name: &str, language: Language) -> String {
        match language {
            Language::Rust => format!(r#"(struct_item name: (type_identifier) @name (#eq? @name "{}")) @struct"#, name),
            Language::TypeScript | Language::JavaScript => {
                format!(r#"(class_declaration name: (identifier) @name (#eq? @name "{}")) @class"#, name)
            }
            Language::Python => format!(r#"(class_definition name: (identifier) @name (#eq? @name "{}")) @class"#, name),
            Language::Go => format!(r#"(type_declaration (type_spec name: (type_identifier) @name (#eq? @name "{}"))) @type"#, name),
            _ => String::new(),
        }
    }

    pub fn all_functions(language: Language) -> &'static str {
        match language {
            Language::Rust => "(function_item) @func",
            Language::TypeScript | Language::JavaScript => "[
                (function_declaration)
                (arrow_function)
                (method_definition)
            ] @func",
            Language::Python => "(function_definition) @func",
            Language::Swift => "[
                (function_declaration)
                (init_declaration)
                (deinit_declaration)
            ] @func",
            Language::Go => "[
                (function_declaration)
                (method_declaration)
            ] @func",
            Language::Java => "(method_declaration) @func",
            _ => "",
        }
    }

    pub fn all_classes(language: Language) -> &'static str {
        match language {
            Language::Rust => "[
                (struct_item)
                (enum_item)
                (trait_item)
            ] @type",
            Language::TypeScript | Language::JavaScript => "(class_declaration) @class",
            Language::Python => "(class_definition) @class",
            Language::Swift => "[
                (class_declaration)
                (protocol_declaration)
            ] @type",
            Language::Go => "(type_declaration) @type",
            Language::Java => "[
                (class_declaration)
                (interface_declaration)
            ] @type",
            _ => "",
        }
    }

    pub fn imports(language: Language) -> &'static str {
        match language {
            Language::Rust => "(use_declaration) @import",
            Language::TypeScript | Language::JavaScript => "[
                (import_statement)
                (import_from_statement)
            ] @import",
            Language::Python => "[
                (import_statement)
                (import_from_statement)
            ] @import",
            Language::Swift => "(import_declaration) @import",
            Language::Go => "(import_declaration) @import",
            Language::Java => "(import_declaration) @import",
            _ => "",
        }
    }
}