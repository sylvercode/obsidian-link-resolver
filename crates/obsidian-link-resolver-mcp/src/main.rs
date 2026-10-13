//! Entry point for the Obsidian Link Resolver MCP server.

use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};

mod mcp;

/// Command-line configuration for the MCP server.
#[derive(Debug, Parser)]
#[command(name = "obsidian-link-resolver-mcp")]
struct Args {
    /// Allowed vault root; repeat to allow multiple vaults.
    #[arg(long, value_name = "DIR", required = true, action = clap::ArgAction::Append)]
    vault: Vec<String>,
}

/// Start the MCP stdio server and wait until the client disconnects.
///
/// # Errors
///
/// Returns an error if the stdio transport cannot initialize or the server stops unexpectedly.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let server = mcp::ResolverServer::new(args.vault)?;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
