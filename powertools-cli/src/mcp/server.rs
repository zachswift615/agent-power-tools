use anyhow::Result;
use rmcp::ServiceExt;
use tracing::info;

use super::tools::PowertoolsService;

/// Run the MCP server using stdio transport
pub async fn run_mcp_server() -> Result<()> {
    info!("Starting powertools MCP server");

    // Create the service
    let service = PowertoolsService::new();

    // Start the server with stdio transport
    info!("MCP server ready, listening on stdio");
    let peer = service
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;

    // Wait for the service to complete
    peer.waiting().await?;

    Ok(())
}
