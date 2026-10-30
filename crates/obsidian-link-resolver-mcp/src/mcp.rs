//! MCP tool adapter for the existing Obsidian link resolver.

use crate::link_generation::{
    render_candidate, select_generation_root, subtarget_is_unique, target_path_within_root,
    DestinationValidation, GenerationResult, GenerationStatus, LinkGenerationRequest, LinkIntent,
};
use obsidian_link_resolver::cache::VaultCache;
use obsidian_link_resolver::error_for_link;
use obsidian_link_resolver::output::Status;
use obsidian_link_resolver::resolve_with_vault;
use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        tool::{IntoCallToolResult, ToolCallContext},
        wrapper::Parameters,
    },
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ServerCapabilities, ServerConfig,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Fixed target used for allow-listed operational diagnostics.
const DIAGNOSTICS_TARGET: &str = "obsidian_link_resolver_mcp::diagnostics";

/// Stable server name advertised in the MCP handshake.
const SERVER_NAME: &str = "obsidian-link-resolver-mcp";

/// Semantic version of this MCP server build.
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Server version and startup-configured vault roots returned by `get_server_info`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ServerInfo {
    /// Semantic version of the MCP server build (`MAJOR.MINOR.PATCH`, optional pre-release/build suffix). Sourced from the crate's `CARGO_PKG_VERSION`; identical to the handshake `serverInfo.version`.
    version: String,
    /// Ordered as supplied at startup, after canonicalization and de-duplication. Never empty.
    vaults: Vec<ConfiguredVault>,
}

/// One distinct vault root accepted at server startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConfiguredVault {
    /// Canonical absolute path exactly as returned by the operating system (`fs::canonicalize`): symlinks and `.`/`..` resolved; Windows extended-length prefix `\\?\` preserved; no trailing-separator or case rewriting.
    path: String,
}

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
    /// Create a resolver server with canonical, distinct vault roots in startup order.
    ///
    /// Duplicate roots that canonicalize to the same path are retained only at their first
    /// occurrence.
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
        let mut canonical_vaults = Vec::with_capacity(allowed_vaults.len());
        for root in allowed_vaults {
            let canonical_root = fs::canonicalize(root)
                .map_err(|_| "an allowed vault root could not be accessed".to_owned())?;
            if !canonical_root.is_dir() {
                return Err("an allowed vault root must be a directory".to_owned());
            }
            if !canonical_vaults.contains(&canonical_root) {
                canonical_vaults.push(canonical_root);
            }
        }
        Ok(Self {
            allowed_vaults: canonical_vaults,
            cache: Arc::new(VaultCache::new()),
            tool_router: Self::tool_router(),
        })
    }
}

#[tool_router]
impl ResolverServer {
    /// Return the server version and canonical vault roots configured at startup.
    ///
    /// This tool takes no arguments and ignores any supplied argument object. It reads only
    /// immutable in-memory configuration and does not access the cache or filesystem.
    ///
    /// # Returns
    ///
    /// A structured result containing `version` and the ordered `vaults` list.
    #[tool(
        name = "get_server_info",
        description = "Return the running server version and the vault roots configured at startup as canonical paths. Takes no arguments. Call it first to learn which vaults the link tools can access."
    )]
    async fn get_server_info(&self) -> Result<CallToolResult, String> {
        let info = ServerInfo {
            version: SERVER_VERSION.to_owned(),
            vaults: self
                .allowed_vaults
                .iter()
                .map(|root| ConfiguredVault {
                    path: root.to_string_lossy().into_owned(),
                })
                .collect(),
        };
        Ok(CallToolResult::structured(
            serde_json::to_value(info).map_err(|error| error.to_string())?,
        ))
    }

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
        description = "Resolve an Obsidian link from the Markdown file containing it. Supported examples include Wikilinks [[Note]], [[Note#Heading]], [[Note#^blockid]], and Markdown links [text](Note.md) and [text](Note.md#Heading). Pass the link exactly as written in the source; do not convert it to another form. Set context_path to the Markdown file containing the link. A folder of Markdown files works without the Obsidian application, but the server can access only directories configured at startup. Optionally select an allowed vault root and request structured emplacement. The result preserves the resolver outcomes: resolved, unresolved, sub_target_not_found, ambiguous, or error. A simple target line is sufficient for a single point target. Set with_emplacement when reading a precise section or shard; it provides the containing heading stack, section range, and structured block when available. Attachments do not gain emplacement data."
    )]
    async fn resolve_obsidian_link(
        &self,
        Parameters(request): Parameters<ResolveRequest>,
    ) -> Result<CallToolResult, String> {
        let request_seq = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        log::info!(target: DIAGNOSTICS_TARGET, event = "request_started", request_seq = request_seq; "MCP request started");

        self.resolve_request(request, request_seq, started).await
    }

    /// Generate a syntax-checked Obsidian link candidate within an allowed vault.
    ///
    /// # Parameters
    ///
    /// - `request`: Strict destination, context, intent, and output settings.
    ///
    /// # Returns
    ///
    /// A structured generation result, or an MCP error for malformed required arguments.
    #[tool(
        name = "generate_obsidian_link",
        description = "Generate Obsidian link text for an exact vault-relative note or supported attachment path. Choose intent as existing or future; future results are unverified and never create files. The source context and destination must remain inside a vault configured at startup. The default format is wikilink and the default output is link."
    )]
    async fn generate_obsidian_link(
        &self,
        Parameters(request): Parameters<LinkGenerationRequest>,
    ) -> Result<CallToolResult, String> {
        self.generate_request(request).await
    }
}

impl ResolverServer {
    /// Check request boundaries and return a future candidate without claiming validation.
    ///
    /// # Parameters
    ///
    /// - `request`: Deserialized generation arguments.
    ///
    /// # Returns
    ///
    /// A structured link-generation result or an MCP error for empty required values.
    async fn generate_request(
        &self,
        request: LinkGenerationRequest,
    ) -> Result<CallToolResult, String> {
        if request.context_path.is_empty() || request.target_path.is_empty() {
            return Err("context_path and target_path must not be empty".to_owned());
        }
        let root = match select_generation_root(
            &request.context_path,
            request.vault_root.as_deref(),
            &self.allowed_vaults,
        ) {
            Ok(root) => root,
            Err(reason) => {
                return structured_generation_result(GenerationResult::rejected(
                    request.format,
                    request.output,
                    reason,
                ));
            }
        };
        let target = match target_path_within_root(&root, &request.target_path) {
            Ok(target) => target,
            Err(reason) => {
                return structured_generation_result(GenerationResult::rejected(
                    request.format,
                    request.output,
                    reason,
                ));
            }
        };
        let generation = match self.cache.get_or_refresh_canonical(&root).await {
            Ok(generation) => generation,
            Err(reason) => {
                return structured_generation_result(GenerationResult::rejected(
                    request.format,
                    request.output,
                    reason,
                ));
            }
        };
        let relative_target = target
            .strip_prefix(&root)
            .map_err(|_| "target_path must remain inside the configured vault".to_owned())?;
        let relative_target = relative_target
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let context_absolute = if PathBuf::from(&request.context_path).is_absolute() {
            PathBuf::from(&request.context_path)
        } else {
            std::env::current_dir()
                .map_err(|_| "working directory could not be determined".to_owned())?
                .join(&request.context_path)
        };
        let context_path = fs::canonicalize(&context_absolute).unwrap_or(context_absolute);
        let context_relative = context_path.strip_prefix(&root).ok().map(|path| {
            path.components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        });
        let candidate =
            match render_candidate(&request, &relative_target, context_relative.as_deref()) {
                Ok(candidate) => candidate,
                Err(reason) => {
                    return structured_generation_result(rejected_generation_result(
                        &request,
                        Some(relative_target),
                        reason,
                        true,
                        false,
                    ));
                }
            };
        if matches!(request.intent, LinkIntent::Future) {
            return structured_generation_result(GenerationResult {
                link_text: Some(candidate),
                status: GenerationStatus::Unverified,
                syntax_valid: true,
                path_allowed: true,
                destination_validation: DestinationValidation::Unverified,
                target_path: Some(relative_target),
                format: request.format,
                output: request.output,
                reason: Some("future destination has not been validated".to_owned()),
            });
        }

        let resolution = resolve_with_vault(
            &candidate,
            &context_path.to_string_lossy(),
            &generation.vault,
            false,
        );
        let unique_subtarget = subtarget_is_unique(&target, &request);
        let exact_match = resolution.status == Status::Resolved
            && resolution.target_path.as_deref() == Some(relative_target.as_str())
            && unique_subtarget;
        if exact_match {
            structured_generation_result(GenerationResult {
                link_text: Some(candidate),
                status: GenerationStatus::Validated,
                syntax_valid: true,
                path_allowed: true,
                destination_validation: DestinationValidation::Validated,
                target_path: Some(relative_target),
                format: request.format,
                output: request.output,
                reason: None,
            })
        } else {
            let reason = resolution.reason.unwrap_or_else(|| {
                if !unique_subtarget {
                    "requested subtarget is duplicated and cannot be validated exactly".to_owned()
                } else {
                    "generated link did not resolve to the exact requested destination".to_owned()
                }
            });
            structured_generation_result(rejected_generation_result(
                &request,
                Some(relative_target),
                reason,
                true,
                true,
            ))
        }
    }

    /// Validate a request, select its vault generation, and serialize the resolver result.
    ///
    /// # Parameters
    ///
    /// - `request`: Deserialized arguments for one resolver tool call.
    /// - `request_seq`: Process-local identifier used only for diagnostics.
    /// - `started`: Start instant used to report request duration.
    ///
    /// # Returns
    ///
    /// A successful MCP tool result, including resolver error outcomes, or a tool error when
    /// request validation, root selection, or result serialization fails.
    async fn resolve_request(
        &self,
        request: ResolveRequest,
        request_seq: u64,
        started: Instant,
    ) -> Result<CallToolResult, String> {
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
        let generation = match self.cache.get_or_refresh_canonical(&vault_root).await {
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

/// Build a rejected result while preserving independently known path and syntax state.
///
/// # Parameters
///
/// - `request`: Original output preferences.
/// - `target_path`: Normalized destination when it was safely identified.
/// - `reason`: Human-readable rejection reason.
/// - `path_allowed`: Whether source and target passed the configured-root checks.
/// - `syntax_valid`: Whether a candidate passed parser round-tripping.
///
/// # Returns
///
/// A structured rejected generation result.
fn rejected_generation_result(
    request: &LinkGenerationRequest,
    target_path: Option<String>,
    reason: String,
    path_allowed: bool,
    syntax_valid: bool,
) -> GenerationResult {
    GenerationResult {
        link_text: None,
        status: GenerationStatus::Rejected,
        syntax_valid,
        path_allowed,
        destination_validation: DestinationValidation::Failed,
        target_path,
        format: request.format,
        output: request.output,
        reason: Some(reason),
    }
}

/// Serialize a generation record into MCP structured content.
///
/// # Parameters
///
/// - `result`: Structured generation outcome to return.
///
/// # Returns
///
/// A successful MCP tool result containing the JSON record.
fn structured_generation_result(result: GenerationResult) -> Result<CallToolResult, String> {
    let structured = serde_json::to_value(result).map_err(|error| error.to_string())?;
    Ok(CallToolResult::structured(structured))
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
    /// Dispatch a protocol tool call and preserve MCP argument errors.
    ///
    /// # Parameters
    ///
    /// - `request`: MCP tool name and JSON arguments supplied by the client.
    /// - `context`: Server request context provided by the MCP runtime.
    ///
    /// # Returns
    ///
    /// The MCP tool response or protocol error returned by the server handler.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        if request.name != "resolve_obsidian_link" {
            return self
                .tool_router
                .call(ToolCallContext::new(self, request, context))
                .await;
        }

        let request_seq = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        log::info!(target: DIAGNOSTICS_TARGET, event = "request_started", request_seq = request_seq; "MCP request started");

        let arguments = request.arguments.unwrap_or_default();
        let request =
            match serde_json::from_value::<ResolveRequest>(serde_json::Value::Object(arguments)) {
                Ok(request) => request,
                Err(error) => {
                    log_request_failed(request_seq, "invalid_arguments", started);
                    return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                        "failed to deserialize parameters: {error}"
                    ))])
                    .into());
                }
            };

        self.resolve_request(request, request_seq, started)
            .await
            .into_call_tool_result()
    }

    /// Return capabilities, server identity, and instructions advertised during initialization.
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, SERVER_VERSION))
            .with_instructions("Resolve Obsidian links.")
    }
}
