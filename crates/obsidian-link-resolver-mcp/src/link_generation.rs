//! Request models and vault-boundary checks for generated Obsidian links.

use obsidian_link_resolver::link::{encode_markdown_destination, parse_link, Link};
use obsidian_link_resolver::note::scan_note;
use obsidian_link_resolver::vault::{
    is_markdown_file_name, is_supported_linkable_name, strip_markdown_extension,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Caller-selected intent for validating a generated destination.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LinkIntent {
    /// Require the destination to exist and match exactly.
    Existing,
    /// Produce a syntax-checked candidate without claiming the destination exists.
    Future,
}

/// Output syntax requested for the generated link.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LinkFormat {
    /// Obsidian double-bracket syntax.
    Wikilink,
    /// Obsidian-compatible Markdown syntax.
    Markdown,
}

/// Whether generation returns a navigable link or embedded content syntax.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LinkOutput {
    /// Generate a normal link.
    Link,
    /// Generate an embed.
    Embed,
}

/// Strict arguments accepted by `generate_obsidian_link`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LinkGenerationRequest {
    /// Existing or safely planned source note path.
    #[schemars(length(min = 1))]
    pub context_path: String,
    /// Exact vault-relative note or attachment destination.
    #[schemars(length(min = 1))]
    pub target_path: String,
    /// Whether the destination must already exist or is intentionally future content.
    pub intent: LinkIntent,
    /// Requested syntax; omitted requests use Wikilink syntax.
    #[serde(default = "default_format")]
    pub format: LinkFormat,
    /// Requested insertion form; omitted requests generate a normal link.
    #[serde(default = "default_output")]
    pub output: LinkOutput,
    /// Whether to omit the note path for a same-file subtarget.
    #[serde(default)]
    pub same_file: bool,
    /// Requested heading path, mutually exclusive with `block_id`.
    #[serde(default)]
    pub heading: Option<String>,
    /// Requested block identifier, mutually exclusive with `heading`.
    #[serde(default)]
    pub block_id: Option<String>,
    /// Optional visible label for Wikilink or Markdown output.
    #[serde(default)]
    pub label: Option<String>,
    /// Optional root selector, which must match a launch-time allowed root.
    #[serde(default, deserialize_with = "deserialize_non_empty_vault_root")]
    #[schemars(with = "String", length(min = 1))]
    pub vault_root: Option<String>,
}

/// Caller-facing status of a generated candidate.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GenerationStatus {
    /// Candidate was proven to resolve to the exact requested destination.
    Validated,
    /// Candidate is representable and allowed, but destination existence was not asserted.
    Unverified,
    /// Request or destination failed validation.
    Rejected,
}

/// Destination verification outcome separate from syntax and path checks.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DestinationValidation {
    /// Resolved to the exact requested destination.
    Validated,
    /// Intentionally not checked as an existing destination.
    Unverified,
    /// Missing, ambiguous, or not the requested destination.
    Failed,
}

/// Structured result returned by `generate_obsidian_link`.
#[derive(Debug, Serialize)]
pub(crate) struct GenerationResult {
    /// Candidate insertion text, absent when the request is not representable.
    pub link_text: Option<String>,
    /// Aggregate status intended for callers.
    pub status: GenerationStatus,
    /// Whether the requested syntax was represented faithfully.
    pub syntax_valid: bool,
    /// Whether the source and target remain inside the selected startup root.
    pub path_allowed: bool,
    /// Exact destination-validation result, independent of syntax/path checks.
    pub destination_validation: DestinationValidation,
    /// Canonical vault-relative target path when known.
    pub target_path: Option<String>,
    /// Requested link syntax.
    pub format: LinkFormat,
    /// Requested link or embed output.
    pub output: LinkOutput,
    /// Concise explanation for an unverified or rejected result.
    pub reason: Option<String>,
}

impl GenerationResult {
    /// Create a rejected result for a request that cannot safely reach a vault destination.
    ///
    /// # Parameters
    ///
    /// - `format`: Requested output syntax to preserve in the result.
    /// - `output`: Requested link or embed form to preserve in the result.
    /// - `reason`: Concise explanation of the rejection.
    ///
    /// # Returns
    ///
    /// A rejected record with no candidate and no destination claim.
    pub(crate) fn rejected(
        format: LinkFormat,
        output: LinkOutput,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            link_text: None,
            status: GenerationStatus::Rejected,
            syntax_valid: false,
            path_allowed: false,
            destination_validation: DestinationValidation::Failed,
            target_path: None,
            format,
            output,
            reason: Some(reason.into()),
        }
    }
}

/// Deserialize an optional vault root while rejecting explicit null and empty values.
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

/// Return the canonical configured root containing an existing or planned source path.
///
/// The nearest existing source-path prefix is canonicalized so a symlink cannot redirect a
/// planned path outside an allowed root.
///
/// # Parameters
///
/// - `context_path`: Absolute or working-directory-relative source path.
/// - `requested_root`: Optional caller-selected root that must be launch-allow-listed.
/// - `allowed_roots`: Canonical roots configured when the MCP server started.
///
/// # Returns
///
/// The most specific matching canonical root, or a boundary error.
pub(crate) fn select_generation_root(
    context_path: &str,
    requested_root: Option<&str>,
    allowed_roots: &[PathBuf],
) -> Result<PathBuf, String> {
    let context_path = absolute_lexical_path(Path::new(context_path))?;
    let context_prefix = canonical_existing_prefix(&context_path)?;
    let containing_roots = allowed_roots
        .iter()
        .filter(|root| context_prefix.starts_with(root.as_path()))
        .collect::<Vec<_>>();

    if let Some(requested_root) = requested_root {
        let requested_root = fs::canonicalize(requested_root)
            .map_err(|_| "vault_root is not an allowed configured vault".to_owned())?;
        if !allowed_roots.contains(&requested_root) {
            return Err("vault_root is not an allowed configured vault".to_owned());
        }
        if !context_prefix.starts_with(&requested_root) {
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

/// Resolve a vault-relative target and ensure its existing prefix stays inside the root.
///
/// # Parameters
///
/// - `root`: Canonical configured vault root.
/// - `target_path`: Vault-relative path supplied by the caller.
///
/// # Returns
///
/// The normalized target path under the canonical root, or a boundary error.
pub(crate) fn target_path_within_root(root: &Path, target_path: &str) -> Result<PathBuf, String> {
    let target = Path::new(target_path);
    if target.as_os_str().is_empty()
        || target.is_absolute()
        || target_path.contains('\\')
        || target.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("target_path must be a non-empty vault-relative path".to_owned());
    }
    let joined = root.join(target);
    if !joined.starts_with(root) {
        return Err("target_path must remain inside the configured vault".to_owned());
    }
    let existing_prefix = canonical_existing_prefix(&joined)?;
    if !existing_prefix.starts_with(root) {
        return Err("target_path resolves outside the configured vault".to_owned());
    }
    Ok(joined)
}

/// Render a requested candidate and verify that the shared parser preserves its meaning.
///
/// # Parameters
///
/// - `request`: Typed generation arguments.
/// - `target_path`: Normalized vault-relative destination path.
/// - `context_relative_path`: Existing context path relative to the selected root, when known.
///
/// # Returns
///
/// A candidate link string or an explanation when the request cannot be represented faithfully.
pub(crate) fn render_candidate(
    request: &LinkGenerationRequest,
    target_path: &str,
    context_relative_path: Option<&str>,
) -> Result<String, String> {
    let target_file_name = target_path.rsplit('/').next().unwrap_or_default();
    if !is_supported_linkable_name(target_file_name) {
        return Err("target_path has an unsupported or invalid file name".to_owned());
    }
    if request.heading.is_some() && request.block_id.is_some() {
        return Err("heading and block_id are mutually exclusive".to_owned());
    }
    if request
        .heading
        .as_ref()
        .is_some_and(|heading| heading.is_empty() || heading.trim() != heading)
        || request
            .block_id
            .as_ref()
            .is_some_and(|block_id| block_id.is_empty() || block_id.trim() != block_id)
    {
        return Err("subtarget values must be non-empty and trimmed".to_owned());
    }
    if request
        .label
        .as_ref()
        .is_some_and(|label| !is_representable_display_label(label))
    {
        return Err("display label cannot be represented faithfully".to_owned());
    }
    if matches!(request.format, LinkFormat::Wikilink)
        && (request
            .heading
            .as_deref()
            .is_some_and(contains_wikilink_delimiter)
            || request
                .block_id
                .as_deref()
                .is_some_and(contains_wikilink_delimiter))
    {
        return Err("Wikilink subtarget contains a structural delimiter".to_owned());
    }

    let is_note = is_markdown_file_name(target_file_name);
    if !is_note && (request.heading.is_some() || request.block_id.is_some() || request.same_file) {
        return Err("attachments cannot have same-file, heading, or block targets".to_owned());
    }
    if request.same_file {
        let context_relative_path = context_relative_path
            .ok_or_else(|| "same_file requires an existing contextual note".to_owned())?;
        if request.heading.is_none() && request.block_id.is_none() {
            return Err("same_file requires a heading or block_id".to_owned());
        }
        if !target_path.eq_ignore_ascii_case(context_relative_path) {
            return Err("same_file target must be the contextual note".to_owned());
        }
    }

    let target_token = if request.same_file {
        String::new()
    } else if matches!(request.format, LinkFormat::Wikilink) && is_note {
        strip_markdown_extension(target_path).to_owned()
    } else {
        target_path.to_owned()
    };
    let subtarget = if let Some(heading) = request.heading.as_deref() {
        let segments = heading.split('#').collect::<Vec<_>>();
        if segments
            .iter()
            .any(|segment| segment.trim().is_empty() || segment.starts_with('^'))
        {
            return Err("heading path contains an unrepresentable segment".to_owned());
        }
        format!("#{}", segments.join("#"))
    } else if let Some(block_id) = request.block_id.as_deref() {
        if block_id.starts_with('^')
            || block_id.contains('#')
            || block_id.chars().any(char::is_whitespace)
        {
            return Err("block_id contains an unrepresentable character".to_owned());
        }
        format!("#^{block_id}")
    } else {
        String::new()
    };
    let destination = match request.format {
        LinkFormat::Wikilink => format!("{target_token}{subtarget}"),
        LinkFormat::Markdown => encode_markdown_destination(&format!("{target_token}{subtarget}")),
    };
    let embed_prefix = if matches!(request.output, LinkOutput::Embed) {
        "!"
    } else {
        ""
    };
    let candidate = match request.format {
        LinkFormat::Wikilink => {
            let alias = request
                .label
                .as_ref()
                .map(|label| format!("|{label}"))
                .unwrap_or_default();
            format!("{embed_prefix}[[{destination}{alias}]]")
        }
        LinkFormat::Markdown => {
            let label = request
                .label
                .clone()
                .unwrap_or_else(|| derive_markdown_label(target_path, &subtarget));
            if !is_representable_display_label(&label) {
                return Err("display label cannot be represented faithfully".to_owned());
            }
            let embed_marker = if matches!(request.output, LinkOutput::Embed) {
                "!"
            } else {
                ""
            };
            format!("{embed_marker}[{label}]({destination})")
        }
    };
    let parsed = parse_link(&candidate).map_err(|error| error.to_string())?;
    if !parsed_matches_request(&parsed, request, target_path, &subtarget, is_note) {
        return Err("generated syntax does not round-trip to the requested destination".to_owned());
    }
    Ok(candidate)
}

/// Report whether a requested subtarget occurs exactly once in its destination note.
///
/// The resolver intentionally selects the first matching heading or block, so generation uses
/// this conservative check before claiming that a duplicated selector is exact.
///
/// # Parameters
///
/// - `target_file`: Existing destination note path.
/// - `request`: Original generation request.
///
/// # Returns
///
/// `true` when there is no subtarget or the requested selector occurs once.
pub(crate) fn subtarget_is_unique(target_file: &Path, request: &LinkGenerationRequest) -> bool {
    if request.heading.is_none() && request.block_id.is_none() {
        return true;
    }
    let Ok(contents) = fs::read_to_string(target_file) else {
        return false;
    };
    let scan = scan_note(&contents);
    if let Some(heading) = request.heading.as_deref() {
        let requested_heading = heading.rsplit('#').next().unwrap_or(heading);
        let matches = scan
            .headings
            .iter()
            .filter(|heading| heading.text.eq_ignore_ascii_case(requested_heading))
            .count();
        matches == 1
    } else if let Some(block_id) = request.block_id.as_deref() {
        let matches = scan
            .block_ids
            .iter()
            .filter(|reference| reference.id.eq_ignore_ascii_case(block_id))
            .count();
        matches == 1
    } else {
        true
    }
}

/// Check that a display label can be emitted without Markdown escaping.
fn is_representable_display_label(label: &str) -> bool {
    !label.is_empty()
        && label.trim() == label
        && !label
            .chars()
            .any(|character| matches!(character, '[' | ']' | '\\' | '\n' | '\r'))
}

/// Check for characters that can terminate or split a Wikilink selector.
fn contains_wikilink_delimiter(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, '[' | ']' | '\n' | '\r'))
}

/// Compare a parsed candidate against the exact requested path and subtarget.
///
/// # Parameters
///
/// - `parsed`: Link parsed by the shared resolver parser.
/// - `request`: Original generation request.
/// - `target_path`: Normalized vault-relative target path.
/// - `subtarget`: Encoded selector suffix used for the candidate.
/// - `is_note`: Whether the destination is a Markdown note.
///
/// # Returns
///
/// `true` only when style, destination, selector, and embed metadata match.
fn parsed_matches_request(
    parsed: &Link,
    request: &LinkGenerationRequest,
    target_path: &str,
    subtarget: &str,
    is_note: bool,
) -> bool {
    let expected_style = match request.format {
        LinkFormat::Wikilink => obsidian_link_resolver::link::LinkStyle::Wikilink,
        LinkFormat::Markdown => obsidian_link_resolver::link::LinkStyle::Markdown,
    };
    if parsed.style != expected_style
        || parsed.is_embed != matches!(request.output, LinkOutput::Embed)
    {
        return false;
    }
    if request.same_file {
        if parsed.note_name.is_some() || parsed.folder_path.is_some() {
            return false;
        }
    } else {
        let expected_path = if matches!(request.format, LinkFormat::Wikilink) && is_note {
            strip_markdown_extension(target_path)
        } else {
            target_path
        };
        let parsed_path = match (&parsed.folder_path, &parsed.note_name) {
            (Some(folder), Some(name)) => format!("{folder}/{name}"),
            (None, Some(name)) => name.clone(),
            _ => return false,
        };
        if !parsed_path.eq_ignore_ascii_case(expected_path) {
            return false;
        }
    }
    let expected_headings = request
        .heading
        .as_deref()
        .map(|heading| heading.split('#').map(str::to_owned).collect::<Vec<_>>())
        .unwrap_or_default();
    let expected_display_text = match request.format {
        LinkFormat::Wikilink => request.label.clone(),
        LinkFormat::Markdown => Some(
            request
                .label
                .clone()
                .unwrap_or_else(|| derive_markdown_label(target_path, subtarget)),
        ),
    };
    parsed.heading_path == expected_headings
        && parsed.block_id.as_deref() == request.block_id.as_deref()
        && parsed.display_text == expected_display_text
        && subtarget_matches_request(subtarget, request)
}

/// Verify that the generated selector suffix corresponds to the request fields.
///
/// # Parameters
///
/// - `subtarget`: Rendered suffix after the destination path.
/// - `request`: Original request selectors.
///
/// # Returns
///
/// `true` when the suffix is exactly the requested heading, block, or empty selector.
fn subtarget_matches_request(subtarget: &str, request: &LinkGenerationRequest) -> bool {
    match (&request.heading, &request.block_id) {
        (Some(heading), None) => subtarget == format!("#{heading}"),
        (None, Some(block_id)) => subtarget == format!("#^{block_id}"),
        (None, None) => subtarget.is_empty(),
        _ => false,
    }
}

/// Derive a Markdown label from a destination path and selector.
///
/// # Parameters
///
/// - `target_path`: Vault-relative destination path.
/// - `subtarget`: Rendered heading or block selector.
///
/// # Returns
///
/// A concise label based on the target filename and optional selector.
fn derive_markdown_label(target_path: &str, subtarget: &str) -> String {
    let file_name = target_path.rsplit('/').next().unwrap_or(target_path);
    let target_name = strip_markdown_extension(file_name);
    format!("{target_name}{subtarget}")
}

/// Convert a path to absolute form without collapsing symlinks or parent components.
///
/// # Parameters
///
/// - `path`: Absolute or process-working-directory-relative path.
///
/// # Returns
///
/// An absolute path preserving the caller's component order.
fn absolute_lexical_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| "working directory could not be determined".to_owned())?
            .join(path)
    };
    Ok(absolute)
}

/// Canonicalize the nearest existing prefix of a path whose final components may be planned.
///
/// # Parameters
///
/// - `path`: Absolute lexical path that may not exist yet.
///
/// # Returns
///
/// The canonical existing prefix, rejecting dangling symlinks and non-directory parents.
fn canonical_existing_prefix(path: &Path) -> Result<PathBuf, String> {
    let mut existing = path.to_path_buf();
    let mut missing_components = Vec::new();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let component = existing
                    .file_name()
                    .ok_or_else(|| "path has no existing filesystem prefix".to_owned())?;
                missing_components.push(component.to_os_string());
                if !existing.pop() {
                    return Err("path has no existing filesystem prefix".to_owned());
                }
            }
            Err(_) => return Err("path prefix could not be inspected".to_owned()),
        }
    }
    if !missing_components.is_empty() && !existing.is_dir() {
        return Err("planned path continues beneath a non-directory".to_owned());
    }
    let canonical = fs::canonicalize(&existing)
        .map_err(|_| "path prefix could not be canonicalized".to_owned())?;
    Ok(canonical)
}

/// Return the default Wikilink output format.
/// Return the default Wikilink output format.
fn default_format() -> LinkFormat {
    LinkFormat::Wikilink
}

/// Return the default normal-link output form.
/// Return the default normal-link output form.
fn default_output() -> LinkOutput {
    LinkOutput::Link
}
