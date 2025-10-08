use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content},
    tool, ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

use crate::commands;
use crate::OutputFormat;

/// Powertools MCP Service
#[derive(Debug, Clone)]
pub struct PowertoolsService {
    tool_router: ToolRouter<Self>,
}

impl PowertoolsService {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

// Tool parameter types
#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexProjectParams {
    /// Path to the project directory
    #[serde(default)]
    pub path: Option<String>,

    /// Languages to index (e.g., ["typescript", "python"]). If empty, indexes all detected languages.
    #[serde(default)]
    pub languages: Vec<String>,

    /// Automatically install missing indexers without prompting
    #[serde(default = "default_true")]
    pub auto_install: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GotoDefinitionParams {
    /// Location in format 'file:line:column' (e.g., 'src/utils.ts:42:10')
    pub location: String,

    /// Project root directory (defaults to current directory)
    #[serde(default)]
    pub project_root: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindReferencesParams {
    /// Symbol name or file:line:column location
    pub symbol: String,

    /// Include declarations in results
    #[serde(default)]
    pub include_declarations: bool,

    /// Project root directory (defaults to current directory)
    #[serde(default)]
    pub project_root: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchAstParams {
    /// Tree-sitter query pattern
    pub pattern: String,

    /// Path to search in
    #[serde(default)]
    pub path: Option<String>,

    /// File extensions to include (e.g., [".rs", ".ts"])
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Maximum number of results
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFunctionsParams {
    /// Path to analyze
    #[serde(default)]
    pub path: Option<String>,

    /// Include private functions
    #[serde(default)]
    pub include_private: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListClassesParams {
    /// Path to analyze
    #[serde(default)]
    pub path: Option<String>,

    /// Include nested classes
    #[serde(default)]
    pub include_nested: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectStatsParams {
    /// Path to analyze
    #[serde(default)]
    pub path: Option<String>,

    /// Show detailed breakdown
    #[serde(default)]
    pub detailed: bool,
}

fn default_true() -> bool {
    true
}

fn default_max_results() -> usize {
    50
}

// Tool implementations
#[rmcp::tool_router]
impl PowertoolsService {
    /// Index a project for semantic navigation
    #[tool(description = "Index a project for semantic code navigation. Supports TypeScript, JavaScript, Python, and Rust. Automatically detects all languages in the project.")]
    async fn index_project(
        &self,
        Parameters(params): Parameters<IndexProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        let path_buf = params.path.map(PathBuf::from);
        let format = OutputFormat::Json;

        match commands::index::run(
            path_buf,
            false,
            params.languages,
            params.auto_install,
            &format,
        )
        .await
        {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true,
                    "message": "Project indexed successfully"
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to index project: {}", e),
            )])),
        }
    }

    /// Find where a symbol is defined
    #[tool(description = "Find where a symbol is defined. Provide a file path with line and column (e.g., 'src/file.ts:10:5').")]
    async fn goto_definition(
        &self,
        Parameters(params): Parameters<GotoDefinitionParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_root = params
            .project_root
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let format = OutputFormat::Json;

        match commands::definition::run(params.location, project_root, &format).await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to find definition: {}",
                e
            ))])),
        }
    }

    /// Find all references to a symbol
    #[tool(description = "Find all references to a symbol across the codebase. Returns file paths, line numbers, and context.")]
    async fn find_references(
        &self,
        Parameters(params): Parameters<FindReferencesParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_root = params
            .project_root
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let format = OutputFormat::Json;

        match commands::references::run(
            params.symbol,
            params.include_declarations,
            project_root,
            &format,
        )
        .await
        {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to find references: {}",
                e
            ))])),
        }
    }

    /// Search for code patterns using tree-sitter queries
    #[tool(description = "Search for code patterns using tree-sitter queries. Useful for finding specific code structures.")]
    async fn search_ast(
        &self,
        Parameters(params): Parameters<SearchAstParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = params.path.map(PathBuf::from);
        let format = OutputFormat::Json;

        match commands::search_ast::run(
            params.pattern,
            path,
            params.extensions,
            params.max_results,
            &format,
        )
        .await
        {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to search AST: {}",
                e
            ))])),
        }
    }

    /// List all functions in a file or directory
    #[tool(description = "List all functions in a file or directory. Returns function names, locations, and signatures.")]
    async fn list_functions(
        &self,
        Parameters(params): Parameters<ListFunctionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = params.path.map(PathBuf::from);
        let format = OutputFormat::Json;

        match commands::functions::run(path, params.include_private, &format).await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to list functions: {}",
                e
            ))])),
        }
    }

    /// List all classes, structs, or interfaces
    #[tool(description = "List all classes, structs, or interfaces in a file or directory.")]
    async fn list_classes(
        &self,
        Parameters(params): Parameters<ListClassesParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = params.path.map(PathBuf::from);
        let format = OutputFormat::Json;

        match commands::classes::run(path, params.include_nested, &format).await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to list classes: {}",
                e
            ))])),
        }
    }

    /// Get project statistics
    #[tool(description = "Get statistics about the codebase (file counts, line counts, languages detected).")]
    async fn project_stats(
        &self,
        Parameters(params): Parameters<ProjectStatsParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = params.path.map(PathBuf::from);
        let format = OutputFormat::Json;

        match commands::stats::run(path, params.detailed, &format).await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get project stats: {}",
                e
            ))])),
        }
    }
}

// Server handler implementation
#[rmcp::tool_handler]
impl ServerHandler for PowertoolsService {}
