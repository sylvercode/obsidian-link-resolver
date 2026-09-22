//! Resolution output types and exit-code mapping.
//!
//! This module defines the [`ResolutionTarget`] output record, which represents the result
//! of resolving an Obsidian link. It includes the outcome status, the target file path and line,
//! optional structured emplacement information, and machine-readable reasons for non-success outcomes.

use serde::{Deserialize, Serialize};

/// The outcome of a link resolution operation.
///
/// Every resolution result falls into exactly one of five mutually exclusive outcomes.
/// The status is serialized in compact form in the machine-readable JSON output
/// and is mapped to a process exit code via the [`ResolutionTarget::exit_code`] method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// The link was successfully resolved to a target file and location.
    /// `exit_code: 0`
    Resolved,
    /// The target note or path does not exist in the vault.
    /// `exit_code: 2`
    Unresolved,
    /// The target note exists, but the referenced heading or block ID does not.
    /// `exit_code: 3`
    SubTargetNotFound,
    /// Multiple notes in the vault match the same name (ambiguous reference).
    /// The `candidates` field lists all matching paths.
    /// `exit_code: 4`
    Ambiguous,
    /// An error occurred during resolution (e.g., vault could not be determined, I/O failure).
    /// The `reason` field provides details.
    /// `exit_code: 1`
    Error,
}

/// A line-number range within a note.
///
/// Used to describe the span of a heading section or the whole-file range.
/// Line numbers are 1-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    /// The 1-based line number where the range begins (inclusive).
    pub begin: u32,
    /// The 1-based line number where the range ends (inclusive).
    pub end: u32,
}

/// A reference to a heading and its section range within a note.
///
/// Represents one heading in the structured emplacement hierarchy.
/// The heading's section spans from `begin` through `end`, inclusive, where `end` is the
/// last line belonging to that heading's section before the next heading of equal or higher
/// level, or the final line of the file if no such heading follows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadingRef {
    /// The text content of the heading (without the `#` prefix).
    pub text: String,
    /// The heading level (1–6, corresponding to `#`, `##`, ..., `######`).
    pub level: u8,
    /// The 1-based line number where this heading appears in the note.
    pub begin: u32,
    /// The 1-based line number where this heading's section ends, inclusive.
    /// For a heading at EOF, this equals the file's total line count.
    pub end: u32,
}

/// The structured heading stack and section range containing a resolved target.
///
/// Provides a rich view of where a resolved link points: the ordered heading hierarchy
/// (outermost to innermost) and the line range of the section containing the target.
/// This enables agents to slice and read only the relevant part of a large note.
///
/// # Example
///
/// For a link `[[Note#Design#API#Auth]]` resolving to a line nested under three headings,
/// `heading_stack` would be `[Design (level 1), API (level 2), Auth (level 3)]`,
/// and `section` would span the lines of the Auth section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredEmplacement {
    /// The ordered list of headings from outermost to innermost that contain the target line.
    /// Empty for targets in notes with no headings.
    pub heading_stack: Vec<HeadingRef>,
    /// The line range of the section containing the target (or the whole file if no headings).
    pub section: LineRange,
}

/// The result of resolving an Obsidian link to its target.
///
/// This record encodes the complete outcome of a link resolution operation.
/// The exact fields present depend on the outcome:
/// - **Resolved**: `status`, `target_path`, `target_line` (or `None` for whole-file),
///   `is_embed`, and optionally `emplacement`
/// - **Unresolved**: `status`, `reason`
/// - **SubTargetNotFound**: `status`, `target_path`, `reason`
/// - **Ambiguous**: `status`, `candidates` (sorted list of conflicting paths), `reason`
/// - **Error**: `status`, `reason`
///
/// The JSON serialization is deterministic and compact:
/// - Inapplicable optional fields are omitted (via `#[serde(skip_serializing_if)]`)
/// - Paths are vault-relative with forward-slash separators
/// - Candidates are sorted by vault-relative path using ordinal (byte-wise) comparison
/// - Field order is fixed
/// - No non-deterministic fields (timestamps, random values) appear in the primary record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionTarget {
    /// The outcome status of the resolution.
    pub status: Status,
    /// The vault-relative, forward-slash-normalized path to the target file.
    /// Present for `Resolved` and `SubTargetNotFound`; `None` for other outcomes.
    pub target_path: Option<String>,
    /// The 1-based line number of the target (for heading, block, or same-file references).
    /// `None` indicates the whole file is the target (no specific line).
    /// Always `None` for non-markdown attachments.
    pub target_line: Option<u32>,
    /// `true` if the original link was an embed (e.g., `![[...]]`), affecting display behavior.
    pub is_embed: bool,
    /// The alias text if the original link included one (e.g., `"Custom Text"` from `[[Note|Custom Text]]`).
    pub alias: Option<String>,
    /// For `Ambiguous` outcomes, the sorted list of conflicting vault-relative paths.
    /// Sorted by path using ordinal (byte-wise) comparison for determinism.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<String>>,
    /// A human-readable reason for non-success outcomes.
    /// Present for `Unresolved`, `SubTargetNotFound`, `Ambiguous`, and `Error`.
    pub reason: Option<String>,
    /// The structured heading stack and section range, if requested and the target is inside a note.
    /// `None` for attachments or when not requested.
    pub emplacement: Option<StructuredEmplacement>,
}

impl ResolutionTarget {
    /// Serialize the result as compact machine-readable JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Render a human-readable summary of the result.
    pub fn to_human_string(&self) -> String {
        match self.status {
            Status::Resolved => format!(
                "resolved: {}{}",
                self.target_path.as_deref().unwrap_or("<unknown>"),
                self.target_line
                    .map(|line| format!(":{line}"))
                    .unwrap_or_default()
            ),
            Status::Unresolved => format!("unresolved: {}", self.reason.as_deref().unwrap_or("target not found")),
            Status::SubTargetNotFound => format!(
                "sub_target_not_found: {}",
                self.reason.as_deref().unwrap_or("sub-target not found")
            ),
            Status::Ambiguous => format!("ambiguous: {}", self.reason.as_deref().unwrap_or("multiple matches")),
            Status::Error => format!("error: {}", self.reason.as_deref().unwrap_or("resolution failed")),
        }
    }
}

impl ResolutionTarget {
    /// Map the resolution outcome status to a process exit code.
    ///
    /// # Exit Code Mapping
    ///
    /// - `0`: Resolved — the link was successfully resolved
    /// - `1`: Error — vault could not be determined, I/O error, or other unrecoverable failure
    /// - `2`: Unresolved — the target note or path does not exist
    /// - `3`: SubTargetNotFound — the note exists but the heading or block ID does not
    /// - `4`: Ambiguous — multiple notes match the same name
    ///
    /// # Returns
    ///
    /// An i32 suitable for passing to `std::process::exit()`.
    pub fn exit_code(&self) -> i32 {
        match self.status {
            Status::Resolved => 0,
            Status::Error => 1,
            Status::Unresolved => 2,
            Status::SubTargetNotFound => 3,
            Status::Ambiguous => 4,
        }
    }
}
