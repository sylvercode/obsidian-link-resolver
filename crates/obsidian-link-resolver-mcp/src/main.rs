//! Entry point for the Obsidian Link Resolver MCP server.

use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use std::error::Error;
use structured_logger::{json::new_writer, Builder};

mod link_generation;
mod mcp;

/// Fixed log target routed to stderr when diagnostics are enabled.
const DIAGNOSTICS_TARGET: &str = "obsidian_link_resolver_mcp::diagnostics";

/// Command-line configuration for the MCP server.
#[derive(Debug, Parser)]
#[command(name = "obsidian-link-resolver-mcp")]
struct Args {
    /// Allowed vault root; repeat to allow multiple vaults.
    #[arg(long, value_name = "DIR", required = true, action = clap::ArgAction::Append)]
    vault: Vec<String>,
    /// Emit redacted operational events as JSON lines on stderr.
    #[arg(long)]
    diagnostics: bool,
}

/// Initialize the opt-in synchronous JSON logger with strict target routing.
///
/// # Errors
///
/// Returns an error if another global logger was already installed.
fn initialize_diagnostics() -> Result<(), log::SetLoggerError> {
    Builder::with_level("INFO")
        .with_default_writer(new_writer(std::io::sink()))
        .with_target_writer(DIAGNOSTICS_TARGET, new_writer(std::io::stderr()))
        .try_init()
}

/// Start the MCP stdio server and wait until the client disconnects.
///
/// # Errors
///
/// Returns an error if the stdio transport cannot initialize or the server stops unexpectedly.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.diagnostics {
        initialize_diagnostics()?;
    }
    let server = mcp::ResolverServer::new(args.vault)?;
    if args.diagnostics {
        log::info!(target: DIAGNOSTICS_TARGET, event = "server_started"; "MCP server started");
    }

    let result: Result<(), Box<dyn Error>> = async {
        let service = server.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            if args.diagnostics {
                log::info!(target: DIAGNOSTICS_TARGET, event = "server_stopped"; "MCP server stopped");
            }
            Ok(())
        }
        Err(error) => {
            if args.diagnostics {
                log::info!(target: DIAGNOSTICS_TARGET, event = "server_stopped", failure_kind = "transport"; "MCP server stopped after transport failure");
                std::process::exit(1);
            }
            Err(error)
        }
    }
}
