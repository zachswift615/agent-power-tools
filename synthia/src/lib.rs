// synthia/src/lib.rs
pub mod agent;
pub mod config;
pub mod context_manager;
pub mod jsonl_logger;
pub mod llm;
pub mod permission_config;
pub mod project;
pub mod project_context;
pub mod session;
pub mod tools;
pub mod types;
pub mod ui;

// Re-export key types
pub use context_manager::TokenStats;
