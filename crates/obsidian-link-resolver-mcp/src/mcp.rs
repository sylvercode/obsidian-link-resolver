//! MCP tool adapter for the existing Obsidian link resolver.

use obsidian_link_resolver::resolve;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerConfig},
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Typed arguments accepted by the `resolve_obsidian_link` tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ResolveRequest {
    /// Raw Obsidian link text to resolve.
    #[schemars(length(min = 1))]
    link: String,
    /// Path to the note containing the link.
    #[schemars(length(min = 1))]
    context_path: String,
    /// Optional vault root, which must be one of the launch-time allowed roots.
    #[serde(default, deserialize_with = "deserialize_non_empty_vault_root")]
    #[schemars(with = "String", length(min = 1))]
    vault_root: Option<String>,
    /// Whether to include the resolver's structured emplacement details.
    #[serde(default)]
    with_emplacement: bool,
}

/// Deserialize an optional vault path while rejecting explicit null or empty values.
///
/// # Parameters
///
/// - `deserializer`: JSON value supplied for the `vault_root` field.
///
/// # Returns
///
/// `Some` for a non-empty string and an error for invalid values.
fn deserialize_non_empty_vault_root<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let vault_root = String::deserialize(deserializer)?;
    if vault_root.is_empty() {
        return Err(serde::de::Error::custom("vault_root must not be empty"));
    }
    Ok(Some(vault_root))
}

/// MCP server implementation for the resolver tool.
#[derive(Debug, Clone)]
pub(crate) struct ResolverServer {
    allowed_vaults: Vec<PathBuf>,
    tool_router: ToolRouter<Self>,
}

impl ResolverServer {
    /// Create a resolver server with the vault roots it is permitted to access.
    ///
    /// # Parameters
    ///
    /// - `allowed_vaults`: Launch-time vault roots available to requests.
    ///
    /// # Returns
    ///
    /// A server with the resolver tool registered.
    pub(crate) fn new(allowed_vaults: Vec<String>) -> Result<Self, String> {
        if allowed_vaults.is_empty() {
            return Err("at least one allowed vault root is required".to_owned());
        }
        let allowed_vaults = allowed_vaults
            .iter()
            .map(|root| {
                let canonical_root = fs::canonicalize(root)
                    .map_err(|_| "an allowed vault root could not be accessed".to_owned())?;
                if !canonical_root.is_dir() {
                    return Err("an allowed vault root must be a directory".to_owned());
                }
                Ok(canonical_root)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            allowed_vaults,
            tool_router: Self::tool_router(),
        })
    }
}

#[tool_router]
impl ResolverServer {
    /// Resolve an Obsidian link using the existing resolver semantics.
    ///
    /// # Parameters
    ///
    /// - `request`: Typed link, context, and optional resolution settings.
    ///
    /// # Returns
    ///
    /// The resolver record as MCP structured content and compact JSON text.
    #[tool(
        name = "resolve_obsidian_link",
        description = "Resolve an Obsidian link from its containing note using the existing resolver."
    )]
    async fn resolve_obsidian_link(
        &self,
        Parameters(request): Parameters<ResolveRequest>,
    ) -> Result<CallToolResult, String> {
        if request.link.is_empty() || request.context_path.is_empty() {
            return Err("link and context_path must not be empty".to_owned());
        }
        let vault_root = select_allowed_vault(
            &request.context_path,
            request.vault_root.as_deref(),
            &self.allowed_vaults,
        )?;
        let link = request.link;
        let context_path = request.context_path;
        let with_emplacement = request.with_emplacement;
        let result = tokio::task::spawn_blocking(move || {
            resolve(
                &link,
                &context_path,
                Some(vault_root.to_string_lossy().as_ref()),
                with_emplacement,
            )
        })
        .await
        .map_err(|error| format!("resolver task failed: {error}"))?;
        let structured = serde_json::to_value(result).map_err(|error| error.to_string())?;
        Ok(CallToolResult::structured(structured))
    }
}

/// Select an allowed vault containing the context and validate any requested root.
fn select_allowed_vault(
    context_path: &str,
    requested_root: Option<&str>,
    allowed_vaults: &[PathBuf],
) -> Result<PathBuf, String> {
    let context_path = fs::canonicalize(context_path)
        .map_err(|_| "context_path must be inside a configured vault".to_owned())?;
    let containing_roots = allowed_vaults
        .iter()
        .filter(|root| context_path.starts_with(root.as_path()))
        .collect::<Vec<_>>();

    if let Some(requested_root) = requested_root {
        let requested_root = fs::canonicalize(requested_root)
            .map_err(|_| "vault_root is not an allowed configured vault".to_owned())?;
        if !allowed_vaults.contains(&requested_root) {
            return Err("vault_root is not an allowed configured vault".to_owned());
        }
        if !context_path.starts_with(&requested_root) {
            return Err("context_path must be inside the selected vault".to_owned());
        }
        return Ok(requested_root);
    }

    containing_roots
        .into_iter()
        .max_by_key(|root| root.components().count())
        .map(|root| (*root).clone())
        .ok_or_else(|| "context_path must be inside a configured vault".to_owned())
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ResolverServer {
    /// Return the server capabilities advertised during MCP initialization.
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Resolve Obsidian links through the existing resolver.")
    }
}
