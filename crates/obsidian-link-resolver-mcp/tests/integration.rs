//! End-to-end tests for MCP stdio behavior.

#[path = "integration/installers.rs"]
mod installers;
#[path = "support/mcp_client.rs"]
mod mcp_client;
#[path = "integration/mcp_diagnostics.rs"]
mod mcp_diagnostics;
#[path = "integration/mcp_link_generation.rs"]
mod mcp_link_generation;
#[path = "integration/mcp_resolver_parity.rs"]
mod mcp_resolver_parity;
#[path = "integration/mcp_server_info.rs"]
mod mcp_server_info;
#[path = "integration/mcp_stdio.rs"]
mod mcp_stdio;
#[path = "integration/release_assets.rs"]
mod release_assets;
