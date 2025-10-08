use crate::core::types::*;
use anyhow::Result;
use serde::Serialize;

#[derive(Clone, Debug)]
pub enum OutputFormat {
    Text,
    Json,
    Markdown,
}

pub struct OutputWriter {
    format: OutputFormat,
}

impl OutputWriter {
    pub fn new(format: &crate::OutputFormat) -> Self {
        let format = match format {
            crate::OutputFormat::Text => OutputFormat::Text,
            crate::OutputFormat::Json => OutputFormat::Json,
            crate::OutputFormat::Markdown => OutputFormat::Markdown,
        };
        Self { format }
    }

    pub fn write_symbols(&self, symbols: &[Symbol]) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(symbols)?);
            }
            OutputFormat::Text => {
                for symbol in symbols {
                    println!(
                        "{:?} {} at {}:{}:{}",
                        symbol.kind,
                        symbol.name,
                        symbol.location.file_path.display(),
                        symbol.location.line,
                        symbol.location.column
                    );
                    if let Some(doc) = &symbol.documentation {
                        println!("  {}", doc);
                    }
                }
            }
            OutputFormat::Markdown => {
                println!("# Symbols\n");
                for symbol in symbols {
                    println!(
                        "- **{}** `{}` - [{}:{}:{}]({}#L{})",
                        format!("{:?}", symbol.kind),
                        symbol.name,
                        symbol.location.file_path.display(),
                        symbol.location.line,
                        symbol.location.column,
                        symbol.location.file_path.display(),
                        symbol.location.line
                    );
                    if let Some(doc) = &symbol.documentation {
                        println!("  {}", doc);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn write_references(&self, references: &[Reference]) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(references)?);
            }
            OutputFormat::Text => {
                for reference in references {
                    println!(
                        "{:?} at {}:{}:{}",
                        reference.kind,
                        reference.location.file_path.display(),
                        reference.location.line,
                        reference.location.column
                    );
                    if let Some(context) = &reference.context {
                        println!("  {}", context);
                    }
                }
            }
            OutputFormat::Markdown => {
                println!("# References\n");
                for reference in references {
                    println!(
                        "- **{:?}** - [{}:{}:{}]({}#L{})",
                        reference.kind,
                        reference.location.file_path.display(),
                        reference.location.line,
                        reference.location.column,
                        reference.location.file_path.display(),
                        reference.location.line
                    );
                    if let Some(context) = &reference.context {
                        println!("  ```");
                        println!("  {}", context);
                        println!("  ```");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn write_search_results(&self, results: &[SearchResult]) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(results)?);
            }
            OutputFormat::Text => {
                for result in results {
                    println!(
                        "{}:{}:{} [{}] {}",
                        result.location.file_path.display(),
                        result.location.line,
                        result.location.column,
                        result.node_type,
                        result.matched_text
                    );
                }
            }
            OutputFormat::Markdown => {
                println!("# Search Results\n");
                for result in results {
                    println!(
                        "## [{}:{}:{}]({}#L{})",
                        result.location.file_path.display(),
                        result.location.line,
                        result.location.column,
                        result.location.file_path.display(),
                        result.location.line
                    );
                    println!("\n**Node Type:** `{}`", result.node_type);
                    println!("\n```{:?}", result.language);
                    if let Some(before) = &result.context_before {
                        println!("{}", before);
                    }
                    println!("{}", result.matched_text);
                    if let Some(after) = &result.context_after {
                        println!("{}", after);
                    }
                    println!("```\n");
                }
            }
        }
        Ok(())
    }

    pub fn write_stats(&self, stats: &IndexStats) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(stats)?);
            }
            OutputFormat::Text => {
                println!("Index Statistics:");
                println!("  Total files: {}", stats.total_files);
                println!("  Total symbols: {}", stats.total_symbols);
                println!("  Index time: {}ms", stats.index_time_ms);
                println!("  Index size: {} bytes", stats.index_size_bytes);
                println!("  Languages:");
                for (lang, count) in &stats.languages {
                    println!("    {:?}: {} files", lang, count);
                }
            }
            OutputFormat::Markdown => {
                println!("# Index Statistics\n");
                println!("| Metric | Value |");
                println!("|--------|-------|");
                println!("| Total files | {} |", stats.total_files);
                println!("| Total symbols | {} |", stats.total_symbols);
                println!("| Index time | {}ms |", stats.index_time_ms);
                println!("| Index size | {} bytes |", stats.index_size_bytes);
                println!("\n## Languages\n");
                for (lang, count) in &stats.languages {
                    println!("- **{:?}**: {} files", lang, count);
                }
            }
        }
        Ok(())
    }

    pub fn write_error(&self, error: &str) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct ErrorResponse {
                    error: String,
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&ErrorResponse {
                        error: error.to_string()
                    })?
                );
            }
            OutputFormat::Text | OutputFormat::Markdown => {
                eprintln!("Error: {}", error);
            }
        }
        Ok(())
    }
}