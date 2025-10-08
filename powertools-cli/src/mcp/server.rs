use anyhow::Result;
use rmcp::{Server, ServerInfo, StdioTransport};
use tracing::info;

use super::tools;

/// Run the MCP server using stdio transport
pub async fn run_mcp_server() -> Result<()> {
    info!("Starting powertools MCP server");

    // Create server info
    let server_info = ServerInfo {
        name: "powertools".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // Create server with our tools
    let mut server = Server::new(server_info);

    // Register all tools
    for tool in tools::get_tools() {
        server.add_tool(tool);
    }

    // Create stdio transport
    let transport = StdioTransport::new();

    // Run the server
    info!("MCP server ready, listening on stdio");
    server.run(transport).await?;

    Ok(())
}
