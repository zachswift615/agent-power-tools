use rmcp::Tool;
use rmcp_macros::tool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use anyhow::Result;

use crate::commands;
use crate::OutputFormat;

/// Index a project for semantic navigation
#[tool(
    name = "index_project",
    description = "Index a project for semantic code navigation. Supports TypeScript, JavaScript, Python, and Rust. Automatically detects all languages in the project."
)]
pub async fn index_project(
    #[argument(description = "Path to the project directory")]
    path: Option<String>,

    #[argument(description = "Languages to index (e.g., ['typescript', 'python']). If empty, indexes all detected languages.")]
    languages: Option<Vec<String>>,

    #[argument(description = "Automatically install missing indexers without prompting")]
    auto_install: Option<bool>,
) -> Result<Value> {
    let path_buf = path.map(PathBuf::from);
    let langs = languages.unwrap_or_default();
    let auto = auto_install.unwrap_or(true);
    let format = OutputFormat::Json;

    commands::index::run(path_buf, false, langs, auto, &format).await?;

    Ok(serde_json::json!({
        "success": true,
        "message": "Project indexed successfully"
    }))
}

/// Find the definition of a symbol at a specific location
#[tool(
    name = "goto_definition",
    description = "Find where a symbol is defined. Provide a file path with line and column (e.g., 'src/file.ts:10:5')."
)]
pub async fn goto_definition(
    #[argument(description = "Location in format 'file:line:column' (e.g., 'src/utils.ts:42:10')")]
    location: String,

    #[argument(description = "Project root directory (defaults to current directory)")]
    project_root: Option<String>,
) -> Result<Value> {
    let root = project_root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let format = OutputFormat::Json;

    // Capture output
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output_clone = output.clone();

    // TODO: Modify commands to return results instead of printing
    // For now, we'll call the command and parse its output
    commands::definition::run(location, root, &format).await?;

    Ok(serde_json::json!({
        "success": true
    }))
}

/// Find all references to a symbol
#[tool(
    name = "find_references",
    description = "Find all references to a symbol across the codebase. Returns file paths, line numbers, and context."
)]
pub async fn find_references(
    #[argument(description = "Symbol name to search for")]
    symbol: String,

    #[argument(description = "Include symbol declarations in results")]
    include_declarations: Option<bool>,

    #[argument(description = "Project root directory (defaults to current directory)")]
    project_root: Option<String>,
) -> Result<Value> {
    let root = project_root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let include_decls = include_declarations.unwrap_or(false);
    let format = OutputFormat::Json;

    commands::references::run(symbol, include_decls, root, &format).await?;

    Ok(serde_json::json!({
        "success": true
    }))
}

/// Search for AST patterns using tree-sitter queries
#[tool(
    name = "search_ast",
    description = "Search for code patterns using tree-sitter queries. Useful for finding specific code structures."
)]
pub async fn search_ast(
    #[argument(description = "Tree-sitter query pattern (e.g., '(function_item) @func')")]
    pattern: String,

    #[argument(description = "Path to search in")]
    path: Option<String>,

    #[argument(description = "File extensions to search (e.g., ['.ts', '.rs'])")]
    extensions: Option<Vec<String>>,

    #[argument(description = "Maximum number of results to return")]
    max_results: Option<usize>,
) -> Result<Value> {
    let path_buf = path.map(PathBuf::from);
    let exts = extensions.unwrap_or_default();
    let max = max_results.unwrap_or(50);
    let format = OutputFormat::Json;

    commands::search_ast::run(pattern, path_buf, exts, max, &format).await?;

    Ok(serde_json::json!({
        "success": true
    }))
}

/// List all functions in a file or project
#[tool(
    name = "list_functions",
    description = "List all functions in a file or directory. Returns function names, locations, and signatures."
)]
pub async fn list_functions(
    #[argument(description = "File or directory path")]
    path: Option<String>,

    #[argument(description = "Include private/internal functions")]
    include_private: Option<bool>,
) -> Result<Value> {
    let path_buf = path.map(PathBuf::from);
    let include_priv = include_private.unwrap_or(false);
    let format = OutputFormat::Json;

    commands::functions::run(path_buf, include_priv, &format).await?;

    Ok(serde_json::json!({
        "success": true
    }))
}

/// List all classes/structs in a file or project
#[tool(
    name = "list_classes",
    description = "List all classes, structs, or interfaces in a file or directory."
)]
pub async fn list_classes(
    #[argument(description = "File or directory path")]
    path: Option<String>,

    #[argument(description = "Include nested classes")]
    include_nested: Option<bool>,
) -> Result<Value> {
    let path_buf = path.map(PathBuf::from);
    let include_nest = include_nested.unwrap_or(false);
    let format = OutputFormat::Json;

    commands::classes::run(path_buf, include_nest, &format).await?;

    Ok(serde_json::json!({
        "success": true
    }))
}

/// Get project statistics
#[tool(
    name = "project_stats",
    description = "Get statistics about the codebase (file counts, line counts, languages detected)."
)]
pub async fn project_stats(
    #[argument(description = "Path to analyze")]
    path: Option<String>,

    #[argument(description = "Show detailed breakdown")]
    detailed: Option<bool>,
) -> Result<Value> {
    let path_buf = path.map(PathBuf::from);
    let detail = detailed.unwrap_or(false);
    let format = OutputFormat::Json;

    commands::stats::run(path_buf, detail, &format).await?;

    Ok(serde_json::json!({
        "success": true
    }))
}

/// Get all available tools
pub fn get_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(index_project),
        Box::new(goto_definition),
        Box::new(find_references),
        Box::new(search_ast),
        Box::new(list_functions),
        Box::new(list_classes),
        Box::new(project_stats),
    ]
}
