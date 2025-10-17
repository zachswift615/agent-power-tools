mod agent;
mod llm;
mod tools;
mod types;
mod ui;

use agent::{messages::Command, messages::UIUpdate, AgentActor};
use anyhow::Result;
use llm::{openai::OpenAICompatibleProvider, GenerationConfig};
use std::sync::Arc;
use tools::{
    bash::BashTool, edit::EditTool, glob::GlobTool, grep::GrepTool, read::ReadTool,
    registry::ToolRegistry, write::WriteTool,
};
use tokio::sync::mpsc;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Create LLM provider
    let llm_provider = Arc::new(OpenAICompatibleProvider::new(
        "http://localhost:1234/v1".to_string(),
        None,
    ));

    // Create tool registry
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(BashTool::new(120)))?;
    tool_registry.register(Arc::new(ReadTool::new()))?;
    tool_registry.register(Arc::new(WriteTool::new()))?;
    tool_registry.register(Arc::new(EditTool::new()))?;
    tool_registry.register(Arc::new(GrepTool::new()))?;
    tool_registry.register(Arc::new(GlobTool::new()))?;
    let tool_registry = Arc::new(tool_registry);

    // Create channels
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(100);
    let (ui_tx, ui_rx) = mpsc::channel::<UIUpdate>(100);

    // Create agent actor
    let config = GenerationConfig {
        model: "qwen2.5-coder-7b-instruct".to_string(),
        temperature: 0.7,
        max_tokens: Some(4096),
    };
    let mut agent = AgentActor::new(
        llm_provider,
        tool_registry,
        config,
        ui_tx,
        cmd_rx,
    );

    // Spawn agent actor
    tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            tracing::error!("Agent error: {}", e);
        }
    });

    // Run TUI
    let mut app = App::new(cmd_tx, ui_rx);
    app.run().await?;

    Ok(())
}
