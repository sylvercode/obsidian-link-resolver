//! Vault detection, enumeration, and index management.
//!
//! This module handles locating an Obsidian vault (by looking for an `.obsidian` directory),
//! enumerating all notes and attachments in it, and performing name-based resolution
//! to identify target files.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path};

use walkdir::WalkDir;

/// Indicates how the vault root was determined.
///
/// A vault root can be explicitly provided by the caller or auto-detected
/// by walking up from the context file to find the nearest `.obsidian` directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultSource {
    /// The vault root was explicitly provided by the caller (e.g., via `--vault` flag).
    #[serde(rename = "explicit")]
    Explicit,
    /// The vault root was auto-detected by walking up from the context file.
    #[serde(rename = "detected")]
    Detected,
}

/// A single entry in the vault's note/attachment index.
///
/// This represents one markdown file or attachment found during vault enumeration.
/// The index is populated by a single directory walk that records paths only (no file body reads)
/// to keep enumeration fast and keep per-call resolution work proportional to one target file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteIndexEntry {
    /// The vault-relative, forward-slash-normalized path to the file
    /// (e.g., `"Project Plan.md"` or `"folder/sub/Note.md"` or `"assets/diagram.png"`).
    pub rel_path: String,
    /// For markdown files: the `.md` file stem (e.g., `"Project Plan"` from `"Project Plan.md"`).
    /// For non-markdown attachments: the full filename (e.g., `"diagram.png"`).
    /// This is the name used for bare-name matching in link resolution.
    pub name: String,
    /// `true` if this is a markdown file (`.md` extension), `false` for attachments.
    pub is_markdown: bool,
}

/// Represents a file that contains a link being resolved.
///
/// The context file's location determines the vault root (by walking up to `.obsidian`)
/// and is used for same-file references (e.g., `[[#Heading]]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextFile {
    /// Absolute path to the file containing the link.
    pub path: String,
}

/// An Obsidian vault, with its root, source, and enumerated entries.
///
/// A `Vault` represents a single Obsidian vault: its root directory (containing `.obsidian/`),
/// how the root was determined, and the complete index of all notes and attachments.
/// The index is built once during vault initialization and reused for all resolutions,
/// ensuring consistent and fast name-based lookups.
///
/// # Fields
///
/// - `root`: Absolute path to the vault root directory
/// - `source`: Whether the root was explicit or auto-detected
/// - `entries`: Complete index of all markdown notes and attachments, enumerated in a single directory walk
///
/// # Invariants
///
/// - The `root` path always points to a directory containing an `.obsidian/` subdirectory (or is otherwise valid)
/// - Entries are enumerated without reading file bodies; enumeration order does not affect resolution (see [`NoteIndexEntry`])
/// - Name resolution is case-insensitive for both file basenames and nested heading/block targets
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    /// Absolute path to the vault root directory.
    pub root: String,
    /// Whether the vault root was explicitly provided or auto-detected.
    pub source: VaultSource,
    /// Complete enumerated index of all markdown notes and attachments in the vault.
    pub entries: Vec<NoteIndexEntry>,
}

/// Errors that can occur while resolving note names inside a vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameResolutionError {
    /// No matching note or attachment was found.
    Unresolved { reason: String },
    /// More than one candidate matched the requested name.
    Ambiguous {
        candidates: Vec<String>,
        reason: String,
    },
}

/// Detect the vault root, enumerate its entries, and build a [`Vault`].
pub fn detect_root(context_path: &str, explicit_root: Option<&str>) -> Result<Vault, String> {
    let context_abs = fs::canonicalize(context_path)
        .map_err(|error| format!("failed to read context path '{context_path}': {error}"))?;
    let (root_path, source) = if let Some(explicit_root) =
        explicit_root.filter(|value| !value.trim().is_empty())
    {
        let root_path = fs::canonicalize(explicit_root)
            .map_err(|error| format!("failed to read vault root '{explicit_root}': {error}"))?;
        if !context_abs.starts_with(&root_path) {
            return Err(format!(
                "context path '{context_path}' is outside the explicit vault root '{}', refusing to resolve links",
                root_path.display()
            ));
        }
        (root_path, VaultSource::Explicit)
    } else {
        let mut current = context_abs.parent().map(Path::to_path_buf).ok_or_else(|| {
            format!("vault could not be determined from context path '{context_path}'")
        })?;
        loop {
            if current.join(".obsidian").is_dir() {
                break (current, VaultSource::Detected);
            }
            if !current.pop() {
                return Err(format!(
                    "vault could not be determined from context path '{context_path}'"
                ));
            }
        }
    };

    let entries = enumerate_vault(root_path.to_string_lossy().as_ref())?;
    Ok(Vault {
        root: root_path.to_string_lossy().into_owned(),
        source,
        entries,
    })
}

/// Enumerate note and attachment paths in a vault without reading file bodies.
pub fn enumerate_vault(root: &str) -> Result<Vec<NoteIndexEntry>, String> {
    let root_path = Path::new(root);
    if !root_path.is_dir() {
        return Err(format!("vault root '{root}' is not a directory"));
    }

    let mut entries = Vec::new();
    let walker = WalkDir::new(root_path).into_iter().filter_entry(|entry| {
        !is_hidden_dir(entry)
            && entry
                .file_name()
                .to_str()
                .map(is_supported_linkable_name)
                .unwrap_or(false)
    });
    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        if path == root_path || !path.is_file() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("path is not valid UTF-8: {}", path.display()))?;
        if !is_supported_linkable_name(file_name) {
            continue;
        }

        let rel_path = path
            .strip_prefix(root_path)
            .map_err(|error| format!("failed to relativize path '{}': {error}", path.display()))?;
        let rel_path = normalize_path(rel_path);
        let is_markdown = path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        let name = if is_markdown {
            path.file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| format!("path is not valid UTF-8: {}", path.display()))?
                .to_string()
        } else {
            file_name.to_string()
        };

        entries.push(NoteIndexEntry {
            rel_path,
            name,
            is_markdown,
        });
    }

    Ok(entries)
}

/// Resolve a note name against the vault index.
pub fn resolve_name(
    vault: &Vault,
    note_name: &str,
    folder_path: Option<&str>,
) -> Result<NoteIndexEntry, NameResolutionError> {
    let note_name = note_name.trim();
    if !is_supported_linkable_name(note_name) {
        return Err(NameResolutionError::Unresolved {
            reason: format!("note '{note_name}' is not a supported linkable name"),
        });
    }
    let path_query = folder_path
        .map(|folder| folder.trim())
        .filter(|folder| !folder.is_empty())
        .map(|folder| format!("{folder}/{note_name}"));

    let mut matches: Vec<NoteIndexEntry> = vault
        .entries
        .iter()
        .filter(|entry| {
            if let Some(path_query) = &path_query {
                path_matches(entry, path_query)
            } else if note_name.to_ascii_lowercase().ends_with(".md") {
                entry
                    .name
                    .eq_ignore_ascii_case(note_name.trim_end_matches(".md"))
            } else {
                entry.name.eq_ignore_ascii_case(note_name)
            }
        })
        .cloned()
        .collect();

    if matches.is_empty() {
        let reason = if path_query.is_some() {
            format!("path-qualified note '{note_name}' not found in vault")
        } else {
            format!("note '{note_name}' not found in vault")
        };
        return Err(NameResolutionError::Unresolved { reason });
    }

    if matches.len() > 1 {
        matches.sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
        let candidates = matches.into_iter().map(|entry| entry.rel_path).collect();
        return Err(NameResolutionError::Ambiguous {
            candidates,
            reason: format!("note '{note_name}' matches multiple notes"),
        });
    }

    Ok(matches.remove(0))
}

fn path_matches(entry: &NoteIndexEntry, path_query: &str) -> bool {
    if entry.rel_path.eq_ignore_ascii_case(path_query) {
        return true;
    }

    if entry.is_markdown {
        let mut query_with_extension = path_query.to_string();
        if !query_with_extension.to_ascii_lowercase().ends_with(".md") {
            query_with_extension.push_str(".md");
        }
        entry.rel_path.eq_ignore_ascii_case(&query_with_extension)
    } else {
        false
    }
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn is_hidden_dir(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

pub(crate) fn is_supported_linkable_name(file_name: &str) -> bool {
    if file_name.is_empty() || file_name.starts_with('.') {
        return false;
    }
    if file_name.contains("..") {
        return false;
    }
    const INVALID: &[char] = &[
        '*', '"', '/', '\\', '<', '>', ':', '|', '?', '#', '[', ']', '^',
    ];
    if file_name.chars().any(|ch| INVALID.contains(&ch)) || file_name.contains("%%") {
        return false;
    }
    let ext = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let supported = [
        "md", "base", "canvas", "avif", "bmp", "gif", "jpeg", "jpg", "png", "svg", "webp", "flac",
        "m4a", "mp3", "ogg", "wav", "webm", "3gp", "mkv", "mov", "mp4", "ogv", "pdf",
    ];
    ext.is_empty() || supported.contains(&ext.as_str())
}
