//! Contract tests for the published MCP tool interface.

#[path = "support/mcp_client.rs"]
mod mcp_client;
#[path = "contract/mcp_compatibility.rs"]
mod mcp_compatibility;
#[path = "contract/mcp_determinism.rs"]
mod mcp_determinism;
#[path = "contract/mcp_link_generation.rs"]
mod mcp_link_generation;
#[path = "contract/mcp_tool.rs"]
mod mcp_tool;
#[path = "contract/mcp_tool_metadata.rs"]
mod mcp_tool_metadata;
