use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod analyzers;
mod commands;
mod core;
mod indexers;

#[derive(Parser)]
#[command(name = "powertools")]
#[command(author, version, about, long_about = None)]
#[command(
    about = "Code indexing and navigation tools for AI agents",
    long_about = "Power tools for code intelligence - provides semantic code navigation, \
                  pattern searching, and code analysis capabilities optimized for AI agents."
)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format (json, text, markdown)
    #[arg(short = 'f', long, global = true, default_value = "text")]
    format: OutputFormat,

    /// Path to the project root (defaults to current directory)
    #[arg(short = 'p', long, global = true)]
    project: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Markdown,
}

#[derive(Subcommand)]
enum Commands {
    /// Build or update the code index for a project
    Index {
        /// Path to index (defaults to current directory)
        path: Option<PathBuf>,

        /// Force full re-indexing
        #[arg(short, long)]
        force: bool,

        /// Languages to index (defaults to all supported)
        #[arg(short, long)]
        languages: Vec<String>,

        /// Automatically install missing indexers without prompting
        #[arg(long)]
        auto_install: bool,
    },

    /// Search for patterns in the AST using tree-sitter queries
    SearchAst {
        /// Tree-sitter query pattern
        pattern: String,

        /// File or directory to search in
        #[arg(short = 'p', long)]
        path: Option<PathBuf>,

        /// File extensions to search (e.g., .rs, .ts)
        #[arg(short = 'e', long)]
        extensions: Vec<String>,

        /// Maximum results to return
        #[arg(short = 'm', long, default_value = "50")]
        max_results: usize,
    },

    /// Go to definition of a symbol
    Definition {
        /// File path and position (file:line:column)
        location: String,
    },

    /// Find all references to a symbol
    References {
        /// Symbol name or file:line:column
        symbol: String,

        /// Include declarations
        #[arg(short, long)]
        include_declarations: bool,
    },

    /// Find implementations of an interface or trait
    Implementations {
        /// Interface or trait name
        name: String,
    },

    /// Find all callers of a function
    Callers {
        /// Function name or file:line:column
        function: String,
    },

    /// Get type information for an expression
    Type {
        /// File path and position (file:line:column)
        location: String,
    },

    /// Find symbols by name
    Symbols {
        /// Symbol name or pattern (supports wildcards)
        query: String,

        /// Symbol kind filter (function, class, interface, etc.)
        #[arg(short, long)]
        kind: Option<String>,
    },

    /// Analyze dependencies of a file or module
    Deps {
        /// File or module to analyze
        path: PathBuf,

        /// Show transitive dependencies
        #[arg(short, long)]
        transitive: bool,

        /// Output as dependency graph
        #[arg(short, long)]
        graph: bool,
    },

    /// Analyze code complexity
    Complexity {
        /// File or directory to analyze
        path: Option<PathBuf>,

        /// Sort by complexity
        #[arg(short, long)]
        sort: bool,
    },

    /// Analyze impact of changes to a symbol
    Impact {
        /// Symbol name or file:line:column
        symbol: String,

        /// Maximum depth to analyze
        #[arg(short, long, default_value = "3")]
        depth: usize,
    },

    /// List all functions in a file or project
    Functions {
        /// File or directory to analyze
        path: Option<PathBuf>,

        /// Include private functions
        #[arg(long)]
        include_private: bool,
    },

    /// List all classes/structs in a file or project
    Classes {
        /// File or directory to analyze
        path: Option<PathBuf>,

        /// Include nested classes
        #[arg(long)]
        include_nested: bool,
    },

    /// Get project statistics
    Stats {
        /// Path to analyze
        path: Option<PathBuf>,

        /// Show detailed breakdown
        #[arg(short, long)]
        detailed: bool,
    },

    /// Clear the index cache
    ClearCache {
        /// Confirmation flag
        #[arg(long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("powertools=debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("powertools=info")
            .init();
    }

    // Set project root
    let project_root = cli.project.unwrap_or_else(|| PathBuf::from("."));

    // Execute command
    match cli.command {
        Commands::Index { path, force, languages, auto_install } => {
            commands::index::run(path, force, languages, auto_install, &cli.format).await?
        }
        Commands::SearchAst {
            pattern,
            path,
            extensions,
            max_results,
        } => {
            commands::search_ast::run(
                pattern,
                path,
                extensions,
                max_results,
                &cli.format,
            )
            .await?
        }
        Commands::Definition { location } => {
            commands::definition::run(location, project_root.clone(), &cli.format).await?
        }
        Commands::References {
            symbol,
            include_declarations,
        } => {
            commands::references::run(symbol, include_declarations, project_root.clone(), &cli.format).await?
        }
        Commands::Functions { path, include_private } => {
            commands::functions::run(path, include_private, &cli.format).await?
        }
        Commands::Classes { path, include_nested } => {
            commands::classes::run(path, include_nested, &cli.format).await?
        }
        Commands::Stats { path, detailed } => {
            commands::stats::run(path, detailed, &cli.format).await?
        }
        _ => {
            eprintln!("Command not yet implemented");
            std::process::exit(1);
        }
    }

    Ok(())
}