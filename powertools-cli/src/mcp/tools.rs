use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content},
    tool, ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use anyhow::Result;

use crate::commands;
use crate::OutputFormat;
use crate::watcher::FileWatcher;

/// Powertools MCP Service
#[derive(Clone)]
pub struct PowertoolsService {
    tool_router: ToolRouter<Self>,
    watcher: Arc<Mutex<Option<FileWatcher>>>,
    project_root: PathBuf,
}

impl std::fmt::Debug for PowertoolsService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PowertoolsService")
            .field("project_root", &self.project_root)
            .finish()
    }
}

impl PowertoolsService {
    pub fn new(project_root: PathBuf) -> Result<Self> {
        Ok(Self {
            tool_router: Self::tool_router(),
            watcher: Arc::new(Mutex::new(None)),
            project_root,
        })
    }

    pub async fn start_watcher(&self, debounce: Duration, auto_install: bool) -> Result<()> {
        let mut watcher_guard = self.watcher.lock().await;

        if watcher_guard.is_some() {
            return Ok(()); // Already started
        }

        let mut watcher = FileWatcher::new(self.project_root.clone())?;
        watcher.start(debounce, auto_install).await?;
        *watcher_guard = Some(watcher);

        Ok(())
    }

    pub async fn stop_watcher(&self) {
        let mut watcher_guard = self.watcher.lock().await;
        if let Some(mut watcher) = watcher_guard.take() {
            watcher.stop();
        }
    }

    pub async fn is_watcher_running(&self) -> bool {
        let watcher_guard = self.watcher.lock().await;
        watcher_guard.as_ref().map_or(false, |w| w.is_running())
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

    /// Maximum number of results to return (default: 100)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Number of results to skip (default: 0)
    #[serde(default)]
    pub offset: usize,
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

    /// Maximum number of results to return (default: 100)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Number of results to skip (default: 0)
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFunctionsParams {
    /// Path to analyze
    #[serde(default)]
    pub path: Option<String>,

    /// Include private functions
    #[serde(default)]
    pub include_private: bool,

    /// Maximum number of results to return (default: 100)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Number of results to skip (default: 0)
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListClassesParams {
    /// Path to analyze
    #[serde(default)]
    pub path: Option<String>,

    /// Include nested classes
    #[serde(default)]
    pub include_nested: bool,

    /// Maximum number of results to return (default: 100)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Number of results to skip (default: 0)
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectStatsParams {
    /// Path to analyze
    #[serde(default)]
    pub path: Option<String>,

    /// Show detailed breakdown
    #[serde(default)]
    #[allow(dead_code)]
    pub detailed: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchReplaceParams {
    /// Regex pattern to search for
    pub pattern: String,

    /// Replacement text (supports capture groups like $1, $2)
    pub replacement: String,

    /// File glob pattern (e.g., "*.rs", "**/*.ts")
    #[serde(default)]
    pub file_pattern: Option<String>,

    /// Path to search in (defaults to current directory)
    #[serde(default)]
    pub path: Option<String>,

    /// Preview changes without applying (default: true for safety)
    #[serde(default = "default_true")]
    pub preview: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameSymbolParams {
    /// File path where the symbol is located
    pub file: String,

    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,

    /// New name for the symbol
    pub new_name: String,

    /// Project root directory (defaults to current directory)
    #[serde(default)]
    pub project_root: Option<String>,

    /// Preview changes without applying (default: true for safety)
    #[serde(default = "default_true")]
    pub preview: bool,

    /// Update imports/exports (default: true)
    #[serde(default = "default_true")]
    pub update_imports: bool,
}

fn default_true() -> bool {
    true
}

fn default_max_results() -> usize {
    50
}

fn default_limit() -> usize {
    100
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

        match commands::definition::find_definition(params.location, project_root).await {
            Ok(location) => {
                let result = serde_json::json!({
                    "location": location
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                )]))
            },
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

        match commands::references::find_references(
            params.symbol,
            project_root,
            params.include_declarations,
        )
        .await
        {
            Ok(references) => {
                let total = references.len();
                let paginated: Vec<_> = references
                    .into_iter()
                    .skip(params.offset)
                    .take(params.limit)
                    .collect();
                let has_more = params.offset + paginated.len() < total;

                let result = serde_json::json!({
                    "count": total,
                    "limit": params.limit,
                    "offset": params.offset,
                    "has_more": has_more,
                    "references": paginated
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                )]))
            },
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

        // Ensure max_results is large enough for pagination
        let effective_max_results = params.max_results.max(params.offset + params.limit);

        match commands::search_ast::search_patterns(
            params.pattern,
            path,
            params.extensions,
            effective_max_results,
        )
        .await
        {
            Ok(results) => {
                let total = results.len();
                let paginated: Vec<_> = results
                    .into_iter()
                    .skip(params.offset)
                    .take(params.limit)
                    .collect();
                let has_more = params.offset + paginated.len() < total;

                let result = serde_json::json!({
                    "count": total,
                    "limit": params.limit,
                    "offset": params.offset,
                    "has_more": has_more,
                    "results": paginated
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                )]))
            },
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

        match commands::functions::find_functions(path, params.include_private).await {
            Ok(functions) => {
                let total = functions.len();
                let paginated: Vec<_> = functions
                    .into_iter()
                    .skip(params.offset)
                    .take(params.limit)
                    .collect();
                let has_more = params.offset + paginated.len() < total;

                let result = serde_json::json!({
                    "count": total,
                    "limit": params.limit,
                    "offset": params.offset,
                    "has_more": has_more,
                    "functions": paginated
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                )]))
            },
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

        match commands::classes::find_classes(path, params.include_nested).await {
            Ok(classes) => {
                let total = classes.len();
                let paginated: Vec<_> = classes
                    .into_iter()
                    .skip(params.offset)
                    .take(params.limit)
                    .collect();
                let has_more = params.offset + paginated.len() < total;

                let result = serde_json::json!({
                    "count": total,
                    "limit": params.limit,
                    "offset": params.offset,
                    "has_more": has_more,
                    "classes": paginated
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                )]))
            },
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

        match commands::stats::get_stats(path).await {
            Ok(stats) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&stats).unwrap_or_else(|_|
                        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
                    )
                )]))
            },
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get project stats: {}",
                e
            ))])),
        }
    }

    /// Stop the file watcher
    #[tool(description = "Stop the automatic file watcher and re-indexing.")]
    async fn watcher_stop(&self) -> Result<CallToolResult, McpError> {
        self.stop_watcher().await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "success": true,
                "message": "File watcher stopped"
            })
            .to_string(),
        )]))
    }

    /// Start the file watcher
    #[tool(description = "Start the automatic file watcher and re-indexing.")]
    async fn watcher_start(&self) -> Result<CallToolResult, McpError> {
        match self.start_watcher(Duration::from_secs(2), true).await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "success": true,
                    "message": "File watcher started"
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to start watcher: {}",
                e
            ))])),
        }
    }

    /// Get file watcher status
    #[tool(description = "Get the current status of the file watcher.")]
    async fn get_watcher_status(&self) -> Result<CallToolResult, McpError> {
        let is_running = self.is_watcher_running().await;

        let status = serde_json::json!({
            "is_running": is_running,
            "project_root": self.project_root.display().to_string(),
            "debounce_ms": if is_running { 2000 } else { 0 },
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&status).unwrap_or_else(|_| status.to_string())
        )]))
    }

    /// Batch replace text across multiple files using regex
    #[tool(description = "Replace text across multiple files using regex patterns. ALWAYS preview first (preview=true) to see changes before applying.")]
    async fn batch_replace(
        &self,
        Parameters(params): Parameters<BatchReplaceParams>,
    ) -> Result<CallToolResult, McpError> {
        use crate::refactor::BatchReplacer;

        let path = params.path.map(PathBuf::from).unwrap_or_else(|| self.project_root.clone());

        let replacer = match BatchReplacer::new(
            &params.pattern,
            params.replacement.clone(),
            params.file_pattern.clone(),
            path,
        ) {
            Ok(r) => r,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(format!(
                "Invalid regex pattern: {}",
                e
            ))])),
        };

        if params.preview {
            // Preview mode - show what would change
            match replacer.preview() {
                Ok(diffs) => {
                    let result = serde_json::json!({
                        "preview": true,
                        "num_files": diffs.len(),
                        "total_changes": diffs.iter().map(|d| d.num_changes).sum::<usize>(),
                        "diffs": diffs,
                    });
                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                    )]))
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to preview changes: {}",
                    e
                ))])),
            }
        } else {
            // Apply mode - make the changes
            match replacer.apply() {
                Ok(result) => {
                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_|
                            serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
                        )
                    )]))
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to apply changes: {}",
                    e
                ))])),
            }
        }
    }

    /// Rename a symbol across the codebase
    #[tool(description = "Rename a symbol across the entire codebase with semantic awareness. ALWAYS preview first (preview=true) to see all changes. Uses SCIP indexes for precise symbol resolution.")]
    async fn rename_symbol(
        &self,
        Parameters(params): Parameters<RenameSymbolParams>,
    ) -> Result<CallToolResult, McpError> {
        use crate::indexers::ScipQuery;
        use crate::refactor::{RenameOptions, SymbolRenamer, TransactionMode};

        let project_root = params.project_root
            .map(PathBuf::from)
            .unwrap_or_else(|| self.project_root.clone());

        // Load SCIP index
        let scip_query = match ScipQuery::from_project(project_root.clone()) {
            Ok(q) => q,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to load SCIP index: {}. Run 'powertools index' first.",
                e
            ))])),
        };

        // Create renamer
        let renamer = SymbolRenamer::new(&scip_query, project_root.clone());

        // Build options
        let options = RenameOptions {
            file_path: PathBuf::from(&params.file),
            line: params.line,
            column: params.column,
            new_name: params.new_name.clone(),
            update_imports: params.update_imports,
            mode: if params.preview {
                TransactionMode::DryRun
            } else {
                TransactionMode::Execute
            },
        };

        if params.preview {
            // Preview mode - show what would change
            match renamer.preview(options) {
                Ok(summary) => {
                    let result = serde_json::json!({
                        "preview": true,
                        "total_files": summary.total_files,
                        "total_changes": summary.total_changes,
                        "total_import_changes": summary.total_import_changes,
                        "overall_risk": summary.overall_risk,
                        "warnings": summary.warnings,
                        "file_changes": summary.file_changes,
                    });
                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                    )]))
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to preview rename: {}",
                    e
                ))])),
            }
        } else {
            // Apply mode - make the changes
            match renamer.rename(options) {
                Ok(result) => {
                    let response = serde_json::json!({
                        "success": true,
                        "old_name": result.old_name,
                        "new_name": result.new_name,
                        "references_updated": result.references_updated,
                        "files_modified": result.files_modified,
                        "imports_updated": result.imports_updated,
                        "modified_files": result.transaction_result.files_modified,
                    });
                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string())
                    )]))
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to rename symbol: {}",
                    e
                ))])),
            }
        }
    }
}

// Server handler implementation
#[rmcp::tool_handler]
impl ServerHandler for PowertoolsService {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        use rmcp::model::{Implementation, ServerCapabilities, ToolsCapability, ProtocolVersion};

        rmcp::model::ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: None,
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "powertools".to_string(),
                title: Some("Powertools MCP Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://github.com/zachswift615/agent-power-tools".to_string()),
            },
            instructions: Some(
                "Powertools provides semantic code navigation and analysis. \
                 Use index_project first to build the code index, then use other tools \
                 for navigation and analysis.".to_string()
            ),
        }
    }
}
