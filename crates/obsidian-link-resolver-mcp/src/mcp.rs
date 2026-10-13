//! MCP tool adapter for the existing Obsidian link resolver.

use obsidian_link_resolver::resolve;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerConfig},
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;

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
    /// Optional vault root, overriding the launch-time default.
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
    default_vault: Option<String>,
    tool_router: ToolRouter<Self>,
}

impl ResolverServer {
    /// Create a resolver server with an optional default vault root.
    ///
    /// # Parameters
    ///
    /// - `default_vault`: Launch-time vault root used when a request omits `vault_root`.
    ///
    /// # Returns
    ///
    /// A server with the resolver tool registered.
    pub(crate) fn new(default_vault: Option<String>) -> Self {
        Self {
            default_vault,
            tool_router: Self::tool_router(),
        }
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
        let vault_root = request
            .vault_root
            .as_deref()
            .or(self.default_vault.as_deref());
        let result = resolve(
            &request.link,
            &request.context_path,
            vault_root,
            request.with_emplacement,
        );
        let structured = serde_json::to_value(result).map_err(|error| error.to_string())?;
        Ok(CallToolResult::structured(structured))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ResolverServer {
    /// Return the server capabilities advertised during MCP initialization.
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Resolve Obsidian links through the existing resolver.")
    }
}
