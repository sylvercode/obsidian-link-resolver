//! MCP tool adapter for the existing Obsidian link resolver.

use crate::cache::VaultCache;
use obsidian_link_resolver::error_for_link;
use obsidian_link_resolver::resolve_with_vault;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerConfig},
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Fixed target used for allow-listed operational diagnostics.
const DIAGNOSTICS_TARGET: &str = "obsidian_link_resolver_mcp::diagnostics";

/// Process-local sequence assigned to each tool request.
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Typed arguments accepted by the `resolve_obsidian_link` tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ResolveRequest {
    /// Raw Obsidian link text to resolve.
    #[schemars(length(min = 1))]
    link: String,
    /// Path to the file containing the link; may be absolute or relative to the server's working directory.
    #[schemars(length(min = 1))]
    context_path: String,
    /// Optional allowed vault root, which must match a launch-time `--vault` root.
    #[serde(default, deserialize_with = "deserialize_non_empty_vault_root")]
    #[schemars(with = "String", length(min = 1))]
    vault_root: Option<String>,
    /// Whether to include structured emplacement details when available; defaults to false.
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
    /// Process-scoped index snapshots keyed by canonical vault root.
    cache: Arc<VaultCache>,
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
            cache: Arc::new(VaultCache::new()),
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
        description = "Resolve an Obsidian link from the file containing it. Provide the raw link and context path; optionally select an allowed vault root and request structured emplacement. The result preserves the resolver outcomes: resolved, unresolved, sub_target_not_found, ambiguous, or error. A simple target line is sufficient for a single point target. Set with_emplacement when reading a precise section or shard; it provides the containing heading stack, section range, and structured block when available. Attachments do not gain emplacement data."
    )]
    async fn resolve_obsidian_link(
        &self,
        Parameters(request): Parameters<ResolveRequest>,
    ) -> Result<CallToolResult, String> {
        let request_seq = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        log::info!(target: DIAGNOSTICS_TARGET, event = "request_started", request_seq = request_seq; "MCP request started");

        if request.link.is_empty() || request.context_path.is_empty() {
            log_request_failed(request_seq, "invalid_arguments", started);
            return Err("link and context_path must not be empty".to_owned());
        }
        let vault_root = match select_allowed_vault(
            &request.context_path,
            request.vault_root.as_deref(),
            &self.allowed_vaults,
        ) {
            Ok(vault_root) => vault_root,
            Err(error) => {
                log_request_failed(request_seq, "root_selection", started);
                return Err(error);
            }
        };
        let generation = match self.cache.get_or_refresh(&vault_root).await {
            Ok(generation) => generation,
            Err(reason) => {
                let structured = match serde_json::to_value(error_for_link(&request.link, reason)) {
                    Ok(structured) => structured,
                    Err(_) => {
                        log_request_failed(request_seq, "internal", started);
                        return Err("resolver result could not be serialized".to_owned());
                    }
                };
                log_request_completed(request_seq, "error", started);
                return Ok(CallToolResult::structured(structured));
            }
        };
        let link = request.link;
        let context_path = request.context_path;
        let with_emplacement = request.with_emplacement;
        let result = match tokio::task::spawn_blocking(move || {
            resolve_with_vault(&link, &context_path, &generation.vault, with_emplacement)
        })
        .await
        {
            Ok(result) => result,
            Err(error) => {
                log_request_failed(request_seq, "internal", started);
                return Err(format!("resolver task failed: {error}"));
            }
        };
        let structured = match serde_json::to_value(result) {
            Ok(structured) => structured,
            Err(error) => {
                log_request_failed(request_seq, "internal", started);
                return Err(error.to_string());
            }
        };
        log_request_completed(request_seq, resolver_status(&structured), started);
        Ok(CallToolResult::structured(structured))
    }
}

/// Record a completed resolver response using only its fixed outcome category.
///
/// # Parameters
///
/// - `request_seq`: Process-local request sequence.
/// - `status`: Resolver outcome category, selected from the fixed status allow-list.
/// - `started`: Monotonic request start time used to calculate elapsed milliseconds.
fn log_request_completed(request_seq: u64, status: &'static str, started: Instant) {
    log::info!(target: DIAGNOSTICS_TARGET, event = "request_completed", request_seq = request_seq, status = status, duration_ms = elapsed_milliseconds(started); "MCP request completed");
}

/// Record a failed request with a fixed failure category and no raw error details.
///
/// # Parameters
///
/// - `request_seq`: Process-local request sequence.
/// - `failure_kind`: Fixed category selected at the failure boundary.
/// - `started`: Monotonic request start time used to calculate elapsed milliseconds.
fn log_request_failed(request_seq: u64, failure_kind: &'static str, started: Instant) {
    log::info!(target: DIAGNOSTICS_TARGET, event = "request_failed", request_seq = request_seq, failure_kind = failure_kind, duration_ms = elapsed_milliseconds(started); "MCP request failed");
}

/// Convert elapsed monotonic time to a bounded millisecond field.
///
/// # Parameters
///
/// - `started`: Monotonic start time.
///
/// # Returns
///
/// Elapsed milliseconds capped at the largest representable `u64`.
fn elapsed_milliseconds(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

/// Map serialized resolver status to the fixed diagnostic status allow-list.
///
/// # Parameters
///
/// - `result`: Serialized resolver result record.
///
/// # Returns
///
/// One of the five stable resolver outcome values.
fn resolver_status(result: &serde_json::Value) -> &'static str {
    match result.get("status").and_then(serde_json::Value::as_str) {
        Some("resolved") => "resolved",
        Some("unresolved") => "unresolved",
        Some("sub_target_not_found") => "sub_target_not_found",
        Some("ambiguous") => "ambiguous",
        _ => "error",
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
