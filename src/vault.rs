//! Vault detection, enumeration, and index management.
//!
//! This module handles locating an Obsidian vault (by looking for an `.obsidian` directory),
//! enumerating all notes and attachments in it, and performing name-based resolution
//! to identify target files.

use serde::{Deserialize, Serialize};

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
