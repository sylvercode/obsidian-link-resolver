//! # Obsidian Link Resolver
//!
//! A fast, deterministic Rust library for resolving Obsidian markdown links to their target files
//! and locations within a vault.
//!
//! ## Core Functionality
//!
//! The resolver takes three inputs:
//! - An Obsidian link string (e.g., `[[Project Plan#Milestones]]`)
//! - The path of the context file containing the link
//! - An optional explicit vault root (auto-detected if not provided)
//!
//! It returns a [`ResolutionTarget`] containing:
//! - The resolved target file path (vault-relative, forward-slash normalized)
//! - The 1-based line number of the target (for heading/block links)
//! - A structured emplacement showing the heading stack and section ranges (optional)
//! - A status indicating success, ambiguity, or error, with the appropriate exit code
//!
//! ## Link Styles Supported
//!
//! - Wikilinks: `[[Note]]`, `[[Note#Heading]]`, `[[Note#^blockid]]`
//! - Aliased: `[[Note|Custom Text]]`
//! - Embeds: `![[Note]]`
//! - Markdown links: `[text](target)`
//! - Same-file: `[[#Heading]]`
//! - Path-qualified: `[[folder/sub/Note]]`
//!
//! ## Performance Characteristics
//!
//! - Vault enumeration: single directory walk (names only, no body reads)
//! - Warm-run target: ≤100 ms per resolution on vaults up to ~5,000 notes
//! - Cold startup: low single-digit milliseconds
//! - FFI embedding: reusable session handle for many consecutive resolutions without per-call spawn
//!
//! ## Determinism
//!
//! All output is deterministic:
//! - Paths are vault-relative with forward-slash separators
//! - Candidate lists are sorted by vault-relative path (ordinal comparison)
//! - JSON output is compact, single-line, with fixed field order
//! - No non-deterministic fields (timestamps, etc.) in primary result objects

pub mod cli;
pub mod ffi;
pub mod link;
pub mod note;
pub mod output;
pub mod resolve;
pub mod vault;

use crate::link::parse_link;
use crate::output::ResolutionTarget;
use crate::resolve::resolve_link;
use crate::vault::{detect_root, ContextFile};

/// Resolve an Obsidian link to its target file and location within a vault.
///
/// # Arguments
///
/// * `link` - The Obsidian link string (e.g., `"[[Project Plan#Milestones]]"` or `"[[#Heading]]"` for same-file)
/// * `context_path` - Absolute path of the file containing the link
/// * `vault` - Optional explicit vault root path; if `None`, the resolver walks up from the context file
///   to find the nearest `.obsidian` directory
/// * `with_emplacement` - If `true`, include the structured heading stack and section ranges in the result
///
/// # Returns
///
/// A [`ResolutionTarget`] containing:
/// - `status`: Outcome code (resolved, unresolved, sub_target_not_found, ambiguous, or error)
/// - `target_path`: Vault-relative, forward-slash-normalized path (present if resolved or unresolved with fallback path)
/// - `target_range`: target interval of the target (present for headings, blocks, and same-file references; `None` for whole-file targets)
/// - `is_embed`: `true` if the original link was an embed (`![[...]]`)
/// - `display_text`: If the link contained display text (e.g., `[[Note|Custom Text]]`), the label text
/// - `candidates`: If status is `ambiguous`, a sorted list of conflicting vault-relative paths
/// - `reason`: Human-readable error or disambiguation reason
/// - `emplacement`: If requested and the target is inside a note, the ordered heading stack and section ranges
///
/// # Exit Code Mapping
///
/// - `0`: resolved
/// - `1`: error (vault could not be determined, I/O issues, etc.)
/// - `2`: unresolved (target note/path not found)
/// - `3`: sub_target_not_found (note exists but heading/block does not)
/// - `4`: ambiguous (multiple notes match the same name)
pub fn resolve(
    link: &str,
    context_path: &str,
    vault: Option<&str>,
    with_emplacement: bool,
) -> ResolutionTarget {
    let parsed = match parse_link(link) {
        Ok(link) => link,
        Err(error) => {
            return ResolutionTarget {
                status: crate::output::Status::Error,
                target_path: None,
                target_range: None,
                is_embed: false,
                display_text: None,
                candidates: None,
                reason: Some(error.to_string()),
                emplacement: None,
            };
        }
    };

    let vault = match detect_root(context_path, vault) {
        Ok(vault) => vault,
        Err(reason) => {
            return ResolutionTarget {
                status: crate::output::Status::Error,
                target_path: None,
                target_range: None,
                is_embed: parsed.is_embed,
                display_text: parsed.display_text.clone(),
                candidates: None,
                reason: Some(reason),
                emplacement: None,
            };
        }
    };

    let context = ContextFile {
        path: context_path.to_string(),
    };

    resolve_link(&parsed, &context, &vault, with_emplacement)
}

pub fn resolve_placeholder() -> &'static str {
    "obsidian-link-resolver"
}
