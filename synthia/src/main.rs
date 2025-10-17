mod agent;
mod config;
mod llm;
mod tools;
mod types;
mod ui;

use agent::{messages::Command, messages::UIUpdate, AgentActor};
use anyhow::Result;
use config::Config;
use llm::{openai::OpenAICompatibleProvider, GenerationConfig};
use std::sync::Arc;
use tools::{
    bash::BashTool, edit::EditTool, git::GitTool, glob::GlobTool, grep::GrepTool,
    powertools::PowertoolsTool, read::ReadTool, registry::ToolRegistry,
    webfetch::WebFetchTool, workshop::WorkshopTool, write::WriteTool,
};
use tokio::sync::mpsc;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::load()?;
    tracing::info!("Configuration loaded successfully");

    // Create LLM provider
    let llm_provider = Arc::new(OpenAICompatibleProvider::new(
        config.llm.api_base.clone(),
        config.llm.api_key.clone(),
    ));

    // Create tool registry with configured timeouts
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(BashTool::new(config.timeouts.bash_timeout)))?;
    tool_registry.register(Arc::new(ReadTool::new()))?;
    tool_registry.register(Arc::new(WriteTool::new()))?;
    tool_registry.register(Arc::new(EditTool::new()))?;
    tool_registry.register(Arc::new(GrepTool::new()))?;
    tool_registry.register(Arc::new(GlobTool::new()))?;
    tool_registry.register(Arc::new(WebFetchTool::new()))?;
    tool_registry.register(Arc::new(GitTool::new(config.timeouts.git_timeout)))?;
    tool_registry.register(Arc::new(PowertoolsTool::new()))?;
    tool_registry.register(Arc::new(WorkshopTool::new(config.timeouts.workshop_timeout)))?;
    let tool_registry = Arc::new(tool_registry);

    // Create channels
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(100);
    let (ui_tx, ui_rx) = mpsc::channel::<UIUpdate>(100);

    // Create generation config from loaded settings
    let gen_config = GenerationConfig {
        model: config.llm.model.clone(),
        temperature: config.llm.temperature,
        max_tokens: config.llm.max_tokens,
    };
    let mut agent = AgentActor::new(
        llm_provider,
        tool_registry,
        gen_config,
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
