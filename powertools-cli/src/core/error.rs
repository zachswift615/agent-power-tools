use thiserror::Error;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum PowerToolsError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Language not supported: {0}")]
    LanguageNotSupported(String),

    #[error("Invalid query pattern: {0}")]
    InvalidQuery(String),

    #[error("Index not found. Please run 'powertools index' first")]
    IndexNotFound,

    #[error("Index corrupted or outdated. Please run 'powertools index --force'")]
    IndexCorrupted,

    #[error("Parse error in {file}: {message}")]
    ParseError { file: PathBuf, message: String },

    #[error("Tree-sitter query error: {0}")]
    QueryError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Invalid location format: {0}")]
    InvalidLocation(String),

    #[error("LSP server error: {0}")]
    LspError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}