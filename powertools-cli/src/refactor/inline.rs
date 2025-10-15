use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

use crate::core::{Location, Reference};
use crate::indexers::ScipQuery;

use super::preview::{PreviewChange, PreviewDiff, RefactoringSummary};
use super::transaction::{RefactoringTransaction, TransactionMode, TransactionResult};

/// Options for inline variable operation
#[derive(Debug, Clone)]
pub struct InlineOptions {
    /// The file where the variable is located
    pub file_path: PathBuf,

    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,

    /// Transaction mode (Execute or DryRun)
    pub mode: TransactionMode,
}

/// Result of an inline operation
#[derive(Debug, Clone, Serialize)]
pub struct InlineResult {
    /// Variable name that was inlined
    pub variable_name: String,

    /// The initializer value that replaced the variable
    pub initializer_value: String,

    /// Number of usages replaced
    pub usages_replaced: usize,

    /// Number of files modified
    pub files_modified: usize,

    /// Transaction result
    pub transaction_result: TransactionResult,
}

/// Information about a variable declaration
#[derive(Debug, Clone)]
struct VariableDeclaration {
    /// Variable name
    name: String,

    /// Initializer value (the expression assigned to the variable)
    initializer: String,

    /// Location of the declaration
    location: Location,

    /// Start and end byte positions of the entire declaration statement
    #[allow(dead_code)] // Reserved for potential multi-line declaration removal
    declaration_start_byte: usize,
    #[allow(dead_code)] // Reserved for potential multi-line declaration removal
    declaration_end_byte: usize,

    /// Whether the variable is reassigned (mutable)
    is_mutable: bool,
}

/// Inline a variable across the codebase
pub struct VariableInliner<'a> {
    #[allow(dead_code)] // Reserved for future SCIP-based reference finding
    scip_query: &'a ScipQuery,
    #[allow(dead_code)] // Reserved for future use in path resolution
    project_root: PathBuf,
}

impl<'a> VariableInliner<'a> {
    pub fn new(scip_query: &'a ScipQuery, project_root: PathBuf) -> Self {
        Self {
            scip_query,
            project_root,
        }
    }

    /// Perform the inline operation
    pub fn inline(&self, options: InlineOptions) -> Result<InlineResult> {
        // Extract variable declaration using tree-sitter
        let file_content = fs::read_to_string(&options.file_path)
            .with_context(|| format!("Failed to read file: {}", options.file_path.display()))?;

        let mut var_decl = self.extract_variable_declaration(
            &options.file_path,
            &file_content,
            options.line,
            options.column,
        )?;

        // Set the actual file path
        var_decl.location.file_path = options.file_path.clone();

        // Step 4: Safety validations
        self.validate_can_inline(&var_decl)?;

        // Step 5: Find all references to this variable using tree-sitter
        // (More reliable than SCIP for function-local variables)
        let usages = self.find_variable_references_tree_sitter(
            &options.file_path,
            &var_decl.name,
            var_decl.location.line,
        )?;

        if usages.is_empty() {
            anyhow::bail!(
                "Variable '{}' is declared but never used. Consider removing it instead.",
                var_decl.name
            );
        }

        // Step 5: Group references by file (should all be in same file for now)
        let mut references_by_file: HashMap<PathBuf, Vec<Reference>> = HashMap::new();
        for reference in usages {
            references_by_file
                .entry(reference.location.file_path.clone())
                .or_insert_with(Vec::new)
                .push(reference);
        }

        // Step 6: Build a transaction with all file changes
        let mut transaction = RefactoringTransaction::new(options.mode);

        for (file_path, file_refs) in &references_by_file {
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

            let new_content = self.inline_variable_in_file(
                &content,
                file_refs,
                &var_decl.name,
                &var_decl.initializer,
                &var_decl,
            )?;

            transaction.add_operation(file_path.clone(), content, new_content)?;
        }

        // Step 7: Commit the transaction
        let transaction_result = transaction.commit()?;

        Ok(InlineResult {
            variable_name: var_decl.name.clone(),
            initializer_value: var_decl.initializer.clone(),
            usages_replaced: references_by_file.values().map(|v| v.len()).sum(),
            files_modified: transaction_result.files_modified.len(),
            transaction_result,
        })
    }

    /// Generate a preview of the inline operation
    pub fn preview(&self, options: InlineOptions) -> Result<RefactoringSummary> {
        let file_content = fs::read_to_string(&options.file_path)
            .with_context(|| format!("Failed to read file: {}", options.file_path.display()))?;

        let mut var_decl = self.extract_variable_declaration(
            &options.file_path,
            &file_content,
            options.line,
            options.column,
        )?;

        // Set the actual file path
        var_decl.location.file_path = options.file_path.clone();

        self.validate_can_inline(&var_decl)?;

        // Find all references using tree-sitter (more reliable than SCIP for local variables)
        let usages = self.find_variable_references_tree_sitter(
            &options.file_path,
            &var_decl.name,
            var_decl.location.line,
        )?;

        if usages.is_empty() {
            anyhow::bail!(
                "Variable '{}' is declared but never used",
                var_decl.name
            );
        }

        let mut references_by_file: HashMap<PathBuf, Vec<Reference>> = HashMap::new();
        for reference in usages {
            references_by_file
                .entry(reference.location.file_path.clone())
                .or_insert_with(Vec::new)
                .push(reference);
        }

        // Build preview diffs
        let mut file_changes = Vec::new();

        for (file_path, file_refs) in &references_by_file {
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

            let mut diff = PreviewDiff::new(file_path.clone());

            // Add change for variable declaration removal
            let decl_line_content = content
                .lines()
                .nth(var_decl.location.line - 1)
                .unwrap_or("")
                .to_string();

            diff.add_change(PreviewChange {
                line: var_decl.location.line,
                column: var_decl.location.column,
                original: decl_line_content.clone(),
                replacement: String::new(), // Will be removed
                line_content: decl_line_content,
            });

            // Add changes for each usage
            for reference in file_refs {
                let line_content = content
                    .lines()
                    .nth(reference.location.line - 1)
                    .unwrap_or("")
                    .to_string();

                diff.add_change(PreviewChange {
                    line: reference.location.line,
                    column: reference.location.column,
                    original: var_decl.name.clone(),
                    replacement: var_decl.initializer.clone(),
                    line_content,
                });
            }

            file_changes.push(diff);
        }

        Ok(RefactoringSummary::new(file_changes))
    }

    /// Extract variable declaration information from AST
    fn extract_variable_declaration(
        &self,
        file_path: &PathBuf,
        content: &str,
        line: usize,
        column: usize,
    ) -> Result<VariableDeclaration> {
        // Determine file extension to choose parser
        let extension = file_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension found"))?;

        match extension {
            "ts" | "tsx" | "js" | "jsx" => {
                self.extract_typescript_variable(content, line, column)
            }
            "rs" => self.extract_rust_variable(content, line, column),
            "py" => self.extract_python_variable(content, line, column),
            "cpp" | "cc" | "cxx" | "hpp" | "h" => {
                self.extract_cpp_variable(content, line, column)
            }
            _ => anyhow::bail!("Unsupported file extension: {}", extension),
        }
    }

    /// Extract TypeScript/JavaScript variable declaration
    fn extract_typescript_variable(
        &self,
        content: &str,
        line: usize,
        column: usize,
    ) -> Result<VariableDeclaration> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .context("Failed to load TypeScript grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse TypeScript code"))?;

        let root_node = tree.root_node();
        let target_byte = self.position_to_byte(content, line, column)?;

        // Find the variable declaration node containing this position
        let var_node = self
            .find_node_at_position(root_node, target_byte, "lexical_declaration")
            .or_else(|| self.find_node_at_position(root_node, target_byte, "variable_declaration"))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No variable declaration found at line {}, column {}",
                    line,
                    column
                )
            })?;

        // Extract variable name and initializer
        let mut name = String::new();
        let mut initializer = String::new();
        let mut is_mutable = false;

        let mut cursor = var_node.walk();
        for child in var_node.children(&mut cursor) {
            match child.kind() {
                "variable_declarator" => {
                    let (var_name, var_init) = self.extract_typescript_declarator(child, content)?;
                    name = var_name;
                    initializer = var_init;
                }
                "let" | "var" => {
                    is_mutable = true;
                }
                _ => {}
            }
        }

        if name.is_empty() {
            anyhow::bail!("Could not extract variable name from declaration");
        }

        if initializer.is_empty() {
            anyhow::bail!(
                "Variable '{}' has no initializer - cannot inline",
                name
            );
        }

        Ok(VariableDeclaration {
            name,
            initializer,
            location: Location {
                file_path: Default::default(), // Will be set by caller
                line,
                column,
                end_line: None,
                end_column: None,
            },
            declaration_start_byte: var_node.start_byte(),
            declaration_end_byte: var_node.end_byte(),
            is_mutable,
        })
    }

    fn extract_typescript_declarator(
        &self,
        node: Node,
        content: &str,
    ) -> Result<(String, String)> {
        let mut name = String::new();
        let mut initializer = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = content[child.byte_range()].to_string();
                    }
                }
                _ if child.kind().contains("expression") || child.kind() == "string" || child.kind() == "number" || child.kind() == "true" || child.kind() == "false" => {
                    initializer = content[child.byte_range()].trim().to_string();
                }
                _ => {}
            }
        }

        Ok((name, initializer))
    }

    /// Extract Rust variable declaration
    fn extract_rust_variable(
        &self,
        content: &str,
        line: usize,
        column: usize,
    ) -> Result<VariableDeclaration> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .context("Failed to load Rust grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Rust code"))?;

        let root_node = tree.root_node();
        let target_byte = self.position_to_byte(content, line, column)?;

        let var_node = self
            .find_node_at_position(root_node, target_byte, "let_declaration")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No let declaration found at line {}, column {}",
                    line,
                    column
                )
            })?;

        let mut name = String::new();
        let mut initializer = String::new();
        let mut is_mutable = false;

        let mut cursor = var_node.walk();
        for child in var_node.children(&mut cursor) {
            match child.kind() {
                "identifier" | "pattern" => {
                    if name.is_empty() {
                        name = content[child.byte_range()].to_string();
                    }
                }
                "mutable_specifier" => {
                    is_mutable = true;
                }
                _ if child.kind().ends_with("_expression") || child.kind() == "string_literal" || child.kind() == "integer_literal" => {
                    initializer = content[child.byte_range()].trim().to_string();
                }
                _ => {}
            }
        }

        if name.is_empty() {
            anyhow::bail!("Could not extract variable name from declaration");
        }

        if initializer.is_empty() {
            anyhow::bail!(
                "Variable '{}' has no initializer - cannot inline",
                name
            );
        }

        Ok(VariableDeclaration {
            name,
            initializer,
            location: Location {
                file_path: Default::default(),
                line,
                column,
                end_line: None,
                end_column: None,
            },
            declaration_start_byte: var_node.start_byte(),
            declaration_end_byte: var_node.end_byte(),
            is_mutable,
        })
    }

    /// Extract Python variable declaration
    fn extract_python_variable(
        &self,
        content: &str,
        line: usize,
        column: usize,
    ) -> Result<VariableDeclaration> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .context("Failed to load Python grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python code"))?;

        let root_node = tree.root_node();
        let target_byte = self.position_to_byte(content, line, column)?;

        let var_node = self
            .find_node_at_position(root_node, target_byte, "assignment")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No assignment statement found at line {}, column {}",
                    line,
                    column
                )
            })?;

        let mut name = String::new();
        let mut initializer = String::new();

        let mut cursor = var_node.walk();
        let children: Vec<Node> = var_node.children(&mut cursor).collect();

        // Python: name = value
        if children.len() >= 3 {
            name = content[children[0].byte_range()].to_string();
            initializer = content[children[2].byte_range()].trim().to_string();
        }

        if name.is_empty() || initializer.is_empty() {
            anyhow::bail!("Could not extract Python assignment");
        }

        // Python variables are always mutable
        Ok(VariableDeclaration {
            name,
            initializer,
            location: Location {
                file_path: Default::default(),
                line,
                column,
                end_line: None,
                end_column: None,
            },
            declaration_start_byte: var_node.start_byte(),
            declaration_end_byte: var_node.end_byte(),
            is_mutable: true, // Python doesn't have const
        })
    }

    /// Extract C++ variable declaration
    fn extract_cpp_variable(
        &self,
        content: &str,
        line: usize,
        column: usize,
    ) -> Result<VariableDeclaration> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .context("Failed to load C++ grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse C++ code"))?;

        let root_node = tree.root_node();
        let target_byte = self.position_to_byte(content, line, column)?;

        let var_node = self
            .find_node_at_position(root_node, target_byte, "declaration")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No declaration found at line {}, column {}",
                    line,
                    column
                )
            })?;

        let mut name = String::new();
        let mut initializer = String::new();
        let mut is_const = false;

        let mut cursor = var_node.walk();
        for child in var_node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = content[child.byte_range()].to_string();
                    }
                }
                "const" => {
                    is_const = true;
                }
                "init_declarator" => {
                    let (var_name, var_init) = self.extract_cpp_declarator(child, content)?;
                    name = var_name;
                    initializer = var_init;
                }
                _ => {}
            }
        }

        if name.is_empty() {
            anyhow::bail!("Could not extract variable name from declaration");
        }

        if initializer.is_empty() {
            anyhow::bail!(
                "Variable '{}' has no initializer - cannot inline",
                name
            );
        }

        Ok(VariableDeclaration {
            name,
            initializer,
            location: Location {
                file_path: Default::default(),
                line,
                column,
                end_line: None,
                end_column: None,
            },
            declaration_start_byte: var_node.start_byte(),
            declaration_end_byte: var_node.end_byte(),
            is_mutable: !is_const,
        })
    }

    fn extract_cpp_declarator(
        &self,
        node: Node,
        content: &str,
    ) -> Result<(String, String)> {
        let mut name = String::new();
        let mut initializer = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = content[child.byte_range()].to_string();
                    }
                }
                _ if child.kind().contains("expression") || child.kind() == "number_literal" || child.kind() == "string_literal" => {
                    initializer = content[child.byte_range()].trim().to_string();
                }
                _ => {}
            }
        }

        Ok((name, initializer))
    }

    /// Find a node of a specific kind at the given byte position
    fn find_node_at_position<'b>(&self, node: Node<'b>, byte: usize, kind: &str) -> Option<Node<'b>> {
        if node.kind() == kind && node.start_byte() <= byte && byte <= node.end_byte() {
            return Some(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_node_at_position(child, byte, kind) {
                return Some(found);
            }
        }

        None
    }

    /// Extract symbol name from location
    #[allow(dead_code)]
    fn extract_symbol_name(&self, location: &Location) -> Result<String> {
        let content = fs::read_to_string(&location.file_path)
            .with_context(|| format!("Failed to read file: {}", location.file_path.display()))?;

        let line = content
            .lines()
            .nth(location.line - 1)
            .ok_or_else(|| anyhow::anyhow!("Line {} not found in file", location.line))?;

        // Extract the identifier at the column position
        let start_col = location.column - 1;
        let chars: Vec<char> = line.chars().collect();

        if start_col >= chars.len() {
            anyhow::bail!("Column {} out of bounds in line {}", location.column, location.line);
        }

        // Find the start of the identifier (go backwards)
        let mut id_start = start_col;
        while id_start > 0 && (chars[id_start - 1].is_alphanumeric() || chars[id_start - 1] == '_') {
            id_start -= 1;
        }

        // Find the end of the identifier (go forwards)
        let mut id_end = start_col;
        while id_end < chars.len() && (chars[id_end].is_alphanumeric() || chars[id_end] == '_') {
            id_end += 1;
        }

        let symbol_name: String = chars[id_start..id_end].iter().collect();

        if symbol_name.is_empty() {
            anyhow::bail!("No identifier found at location");
        }

        Ok(symbol_name)
    }

    /// Convert line/column position to byte offset
    fn position_to_byte(&self, content: &str, line: usize, column: usize) -> Result<usize> {
        let mut byte_offset = 0;
        let mut current_line = 1;
        let mut current_column = 0;

        for ch in content.chars() {
            if current_line == line && current_column == column - 1 {
                return Ok(byte_offset);
            }

            if ch == '\n' {
                current_line += 1;
                current_column = 0;
            } else {
                current_column += 1;
            }

            byte_offset += ch.len_utf8();
        }

        Ok(byte_offset)
    }

    /// Validate that the variable can be safely inlined
    fn validate_can_inline(&self, var_decl: &VariableDeclaration) -> Result<()> {
        // Check 1: Variable should not be mutable
        if var_decl.is_mutable {
            anyhow::bail!(
                "Cannot inline mutable variable '{}'. Only const/immutable variables can be safely inlined.",
                var_decl.name
            );
        }

        // Check 2: Initializer should not have obvious side effects
        if self.has_side_effects(&var_decl.initializer) {
            anyhow::bail!(
                "Cannot inline variable '{}' because its initializer may have side effects: {}",
                var_decl.name,
                var_decl.initializer
            );
        }

        Ok(())
    }

    /// Check if an expression has side effects (simple heuristic)
    fn has_side_effects(&self, expr: &str) -> bool {
        // Simple heuristic: look for function calls (parentheses)
        // This is a conservative check - may reject some safe cases
        expr.contains('(') && expr.contains(')')
    }

    /// Replace all occurrences of the variable with its initializer
    fn inline_variable_in_file(
        &self,
        content: &str,
        references: &[Reference],
        var_name: &str,
        initializer: &str,
        var_decl: &VariableDeclaration,
    ) -> Result<String> {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // Sort references by line and column in reverse order
        let mut sorted_refs = references.to_vec();
        sorted_refs.sort_by(|a, b| {
            b.location
                .line
                .cmp(&a.location.line)
                .then(b.location.column.cmp(&a.location.column))
        });

        // Replace all usages
        for reference in sorted_refs {
            let line_idx = reference.location.line - 1;
            if line_idx >= lines.len() {
                continue;
            }

            let line = &lines[line_idx];
            let col_idx = reference.location.column - 1;

            // Ensure the symbol actually exists at this location
            if !self.symbol_at_position(line, col_idx, var_name) {
                continue;
            }

            // Replace the variable with the initializer
            let chars: Vec<char> = line.chars().collect();

            let mut start = col_idx;
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
                start -= 1;
            }

            let mut end = col_idx;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }

            // Determine if we need parentheses
            let replacement = if self.needs_parentheses(initializer) {
                format!("({})", initializer)
            } else {
                initializer.to_string()
            };

            let new_line = format!(
                "{}{}{}",
                chars[..start].iter().collect::<String>(),
                replacement,
                chars[end..].iter().collect::<String>()
            );

            lines[line_idx] = new_line;
        }

        // Remove the variable declaration line
        let decl_line_idx = var_decl.location.line - 1;
        if decl_line_idx < lines.len() {
            lines.remove(decl_line_idx);
        }

        Ok(lines.join("\n"))
    }

    /// Check if a symbol exists at a specific position in a line
    fn symbol_at_position(&self, line: &str, col_idx: usize, symbol: &str) -> bool {
        let chars: Vec<char> = line.chars().collect();

        if col_idx >= chars.len() {
            return false;
        }

        let mut start = col_idx;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        let mut end = col_idx;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        let found_symbol: String = chars[start..end].iter().collect();
        found_symbol == symbol
    }

    /// Determine if the initializer needs parentheses when inlined
    fn needs_parentheses(&self, initializer: &str) -> bool {
        // Add parentheses for:
        // - Binary operations (contains operators outside of strings)
        // - Multi-line expressions
        // - Anything with commas (except single function calls)

        if initializer.contains('\n') {
            return true;
        }

        // Check for binary operators (simple heuristic)
        let operators = ["+", "-", "*", "/", "%", "&&", "||", "==", "!=", "<", ">", "<=", ">="];
        operators.iter().any(|op| initializer.contains(op))
    }

    /// Find all references to a variable using tree-sitter AST traversal
    /// This is more reliable than SCIP for local variables, which SCIP indexers often skip
    fn find_variable_references_tree_sitter(
        &self,
        file_path: &PathBuf,
        var_name: &str,
        declaration_line: usize,
    ) -> Result<Vec<Reference>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // Determine file extension to choose parser
        let extension = file_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension found"))?;

        match extension {
            "ts" | "tsx" | "js" | "jsx" => {
                self.find_typescript_identifiers(&content, var_name, declaration_line, file_path)
            }
            "rs" => {
                self.find_rust_identifiers(&content, var_name, declaration_line, file_path)
            }
            "py" => {
                self.find_python_identifiers(&content, var_name, declaration_line, file_path)
            }
            "cpp" | "cc" | "cxx" | "hpp" | "h" => {
                self.find_cpp_identifiers(&content, var_name, declaration_line, file_path)
            }
            _ => anyhow::bail!("Unsupported file extension: {}", extension),
        }
    }

    /// Find TypeScript/JavaScript identifiers matching var_name
    fn find_typescript_identifiers(
        &self,
        content: &str,
        var_name: &str,
        declaration_line: usize,
        file_path: &PathBuf,
    ) -> Result<Vec<Reference>> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .context("Failed to load TypeScript grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse TypeScript code"))?;

        let root_node = tree.root_node();
        let mut references = Vec::new();

        self.collect_identifiers(root_node, content, var_name, declaration_line, file_path, &mut references);

        Ok(references)
    }

    /// Find Rust identifiers matching var_name
    fn find_rust_identifiers(
        &self,
        content: &str,
        var_name: &str,
        declaration_line: usize,
        file_path: &PathBuf,
    ) -> Result<Vec<Reference>> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .context("Failed to load Rust grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Rust code"))?;

        let root_node = tree.root_node();
        let mut references = Vec::new();

        self.collect_identifiers(root_node, content, var_name, declaration_line, file_path, &mut references);

        Ok(references)
    }

    /// Find Python identifiers matching var_name
    fn find_python_identifiers(
        &self,
        content: &str,
        var_name: &str,
        declaration_line: usize,
        file_path: &PathBuf,
    ) -> Result<Vec<Reference>> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .context("Failed to load Python grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python code"))?;

        let root_node = tree.root_node();
        let mut references = Vec::new();

        self.collect_identifiers(root_node, content, var_name, declaration_line, file_path, &mut references);

        Ok(references)
    }

    /// Find C++ identifiers matching var_name
    fn find_cpp_identifiers(
        &self,
        content: &str,
        var_name: &str,
        declaration_line: usize,
        file_path: &PathBuf,
    ) -> Result<Vec<Reference>> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .context("Failed to load C++ grammar")?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse C++ code"))?;

        let root_node = tree.root_node();
        let mut references = Vec::new();

        self.collect_identifiers(root_node, content, var_name, declaration_line, file_path, &mut references);

        Ok(references)
    }

    /// Recursively collect all identifier nodes matching var_name
    fn collect_identifiers(
        &self,
        node: Node,
        content: &str,
        var_name: &str,
        declaration_line: usize,
        file_path: &PathBuf,
        references: &mut Vec<Reference>,
    ) {
        // Check if this node is an identifier matching our variable name
        if node.kind() == "identifier" {
            let node_text = &content[node.byte_range()];
            let node_line = node.start_position().row + 1; // tree-sitter uses 0-indexed rows

            if node_text == var_name && node_line > declaration_line {
                references.push(Reference {
                    location: Location {
                        file_path: file_path.clone(),
                        line: node_line,
                        column: node.start_position().column + 1, // tree-sitter uses 0-indexed columns
                        end_line: Some(node.end_position().row + 1),
                        end_column: Some(node.end_position().column + 1),
                    },
                    kind: crate::core::ReferenceKind::Reference,
                    context: None,
                });
            }
        }

        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifiers(child, content, var_name, declaration_line, file_path, references);
        }
    }
}
