use anyhow::Result;
use rmcp::ServiceExt;
use std::time::Duration;
use tracing::info;

use super::tools::PowertoolsService;

/// Run the MCP server using stdio transport
pub async fn run_mcp_server() -> Result<()> {
    info!("Starting powertools MCP server");

    // Get current directory
    let current_dir = std::env::current_dir()?;

    // Create the service with watcher
    let service = PowertoolsService::new(current_dir.clone())?;

    // Start file watcher automatically
    info!("Starting automatic file watcher for: {}", current_dir.display());
    service.start_watcher(Duration::from_secs(2), true).await?;

    // Start the server with stdio transport
    info!("MCP server ready, listening on stdio");
    let peer = service
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    // Wait for the service to complete
    peer.waiting().await?;

    Ok(())
}
