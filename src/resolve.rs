//! Core link resolution pipeline.
//!
//! This module combines the parsed link, context file, and vault into a complete resolution operation,
//! handling same-file references, heading/block lookups, and emplacement computation.

use crate::link::Link;
use crate::note::{find_target_range, TargetLookupError};
use crate::output::{ResolutionTarget, Status};
use crate::vault::{resolve_name, ContextFile, NameResolutionError, Vault};
use std::fs;
use std::path::Path;

/// Resolve a parsed Obsidian link to its target within a vault.
///
/// This is the core resolution function. It takes a parsed [`Link`], the [`ContextFile`] containing it,
/// and the [`Vault`] to search in, and returns a complete [`ResolutionTarget`] describing the outcome.
///
/// # Resolution Steps
///
/// 1. **Link parsing and validation**: The link is inspected for same-file references, embedding, and aliasing.
/// 2. **Note lookup**: If the link references a specific note, the vault's note index is searched
///    (case-insensitive bare-name match or exact path-qualified match).
/// 3. **Heading/block resolution**: If the link targets a heading or block ID, the note is parsed
///    to find the matching heading/block in document order (first match wins).
/// 4. **Emplacement (optional)**: If requested and the target is a heading, the heading stack
///    and section range are computed.
///
/// # Arguments
///
/// * `link` - A parsed Obsidian link (structure decomposed into note name, heading path, block ID, etc.)
/// * `context` - The file containing the link; used for same-file references and vault-root detection
/// * `vault` - The vault to search in
/// * `with_emplacement` - If `true`, populate the `emplacement` field in the result (if applicable)
///
/// # Returns
///
/// A [`ResolutionTarget`] with one of five mutually exclusive outcome statuses:
/// - `Resolved`: The link was successfully resolved; `target_path` and (optionally) `target_line` are set
/// - `Unresolved`: The target note does not exist
/// - `SubTargetNotFound`: The note exists but the referenced heading or block does not
/// - `Ambiguous`: Multiple notes match the same name; `candidates` lists them sorted by path
/// - `Error`: A fatal error occurred (e.g., I/O failure, vault misconfiguration)
///
/// # Special Cases
///
/// - **Same-file reference** (e.g., `[[#Heading]]`): Target is resolved against the context file itself
/// - **Embed** (e.g., `![[Note]]`): The `is_embed` field is set to `true` in the result
/// - **Attachment** (non-markdown): `target_line` is `None` and `emplacement` is omitted
/// - **Whole-file target** (no heading/block): `target_line` is `None` with the file as the target
pub fn resolve_link(
    link: &Link,
    context: &ContextFile,
    vault: &Vault,
    _with_emplacement: bool,
) -> ResolutionTarget {
    let context_path = Path::new(&context.path);
    let target_entry = if let Some(note_name) = &link.note_name {
        match resolve_name(vault, note_name, link.folder_path.as_deref()) {
            Ok(entry) => entry,
            Err(NameResolutionError::Unresolved { reason }) => {
                return error_like(Status::Unresolved, None, None, None, link, reason);
            }
            Err(NameResolutionError::Ambiguous { candidates, reason }) => {
                return ResolutionTarget {
                    status: Status::Ambiguous,
                    target_path: None,
                    target_line: None,
                    target_range: None,
                    is_embed: link.is_embed,
                    display_text: link.display_text.clone(),
                    candidates: Some(candidates),
                    reason: Some(reason),
                    emplacement: None,
                };
            }
        }
    } else {
        let rel_path = context_path
            .strip_prefix(&vault.root)
            .map(|path| normalize_path(path))
            .unwrap_or_else(|_| context.path.clone());
        crate::vault::NoteIndexEntry {
            rel_path: rel_path.clone(),
            name: context_path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string(),
            is_markdown: context_path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md")),
        }
    };

    let target_path = Some(target_entry.rel_path.clone());
    let is_markdown = target_entry.is_markdown;

    let target_range = if link.heading_path.is_empty() && link.block_id.is_none() {
        None
    } else if is_markdown {
        let absolute_path = vault_path_join(&vault.root, &target_entry.rel_path);
        let contents = match fs::read_to_string(&absolute_path) {
            Ok(contents) => contents,
            Err(error) => {
                return ResolutionTarget {
                    status: Status::Error,
                    target_path,
                    target_line: None,
                    target_range: None,
                    is_embed: link.is_embed,
                    display_text: link.display_text.clone(),
                    candidates: None,
                    reason: Some(format!("failed to read note '{}': {error}", absolute_path.display())),
                    emplacement: None,
                };
            }
        };

        match find_target_range(&contents, link) {
            Ok(range) => range,
            Err(TargetLookupError::MissingTarget { reason }) => {
                return ResolutionTarget {
                    status: Status::SubTargetNotFound,
                    target_path,
                    target_line: None,
                    target_range: None,
                    is_embed: link.is_embed,
                    display_text: link.display_text.clone(),
                    candidates: None,
                    reason: Some(reason),
                    emplacement: None,
                };
            }
            Err(TargetLookupError::MalformedReference { reason }) => {
                return ResolutionTarget {
                    status: Status::Error,
                    target_path,
                    target_line: None,
                    target_range: None,
                    is_embed: link.is_embed,
                    display_text: link.display_text.clone(),
                    candidates: None,
                    reason: Some(reason),
                    emplacement: None,
                };
            }
        }
    } else {
        None
    };
    let target_line = target_range.as_ref().map(|range| range.begin);

    if !is_markdown && (!link.heading_path.is_empty() || link.block_id.is_some()) {
        return ResolutionTarget {
            status: Status::SubTargetNotFound,
            target_path,
            target_line: None,
            target_range: None,
            is_embed: link.is_embed,
            display_text: link.display_text.clone(),
            candidates: None,
            reason: Some("attachments do not contain headings or block ids".to_string()),
            emplacement: None,
        };
    }

    ResolutionTarget {
        status: Status::Resolved,
        target_path,
        target_line,
        target_range,
        is_embed: link.is_embed,
        display_text: link.display_text.clone(),
        candidates: None,
        reason: None,
        emplacement: None,
    }
}

fn error_like(
    status: Status,
    target_path: Option<String>,
    target_line: Option<u32>,
    target_range: Option<crate::output::LineRange>,
    link: &Link,
    reason: String,
) -> ResolutionTarget {
    ResolutionTarget {
        status,
        target_path,
        target_line,
        target_range,
        is_embed: link.is_embed,
        display_text: link.display_text.clone(),
        candidates: None,
        reason: Some(reason),
        emplacement: None,
    }
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn vault_path_join(root: &str, relative_path: &str) -> std::path::PathBuf {
    Path::new(root).join(relative_path)
}
