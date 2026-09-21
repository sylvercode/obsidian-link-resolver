//! Obsidian link parsing and representation.
//!
//! This module defines the [`Link`] entity, which represents a parsed Obsidian link
//! in a canonical form. It supports both wikilink (`[[...]]`) and markdown-style (`[text](...)`) link formats,
//! embeds, aliases, folder paths, headings, and block references.

use serde::{Deserialize, Serialize};

/// Distinguishes the syntax style of an Obsidian link.
///
/// Links can be written as either wikilinks (double-bracket syntax) or markdown-style links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStyle {
    /// Wikilink style: `[[Note]]` or `[[Note#Heading]]`
    #[serde(rename = "wikilink")]
    Wikilink,
    /// Markdown link style: `[text](target)` or `[text](target#heading)`
    #[serde(rename = "markdown")]
    Markdown,
}

/// A parsed Obsidian link in canonical form.
///
/// An instance of `Link` represents a fully parsed Obsidian link, breaking down the raw text
/// into its semantic components: optional folder path, note name, heading path, block reference, and alias.
/// The structure is independent of the input link style; both wikilinks and markdown-style links
/// parse into the same `Link` entity.
///
/// # Fields
///
/// - `raw`: The original unparsed link text
/// - `style`: The syntax style (Wikilink or Markdown) of the original link
/// - `is_embed`: `true` if the link is an embed (`![[...]]` or `![...](...))`
/// - `folder_path`: Optional folder or path prefix (e.g., `"folder/sub"` from `[[folder/sub/Note]]`)
/// - `note_name`: Optional target note name; `None` means a same-file reference (e.g., `[[#Heading]]`)
/// - `heading_path`: Ordered list of heading names from the link (e.g., `["Milestones", "Q4"]` from `[[#Milestones#Q4]]`)
/// - `block_id`: Optional block reference (e.g., `"abc123"` from `[[Note#^abc123]]`)
/// - `alias`: Optional custom display text (e.g., `"Custom Text"` from `[[Note|Custom Text]]`)
///
/// # Constraints
///
/// - A link cannot reference both a heading and a block ID simultaneously (they are mutually exclusive)
/// - A same-file reference (absent `note_name`) must carry either a `heading_path` or `block_id`
/// - All heading path components and block IDs are matched case-insensitively, consistent with Obsidian's default behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// The original unparsed link text from the source markdown.
    pub raw: String,
    /// The syntax style (Wikilink or Markdown) of the original link.
    pub style: LinkStyle,
    /// `true` if the link is an embed (e.g., `![[...]]`), which may affect display and behavior.
    pub is_embed: bool,
    /// Optional folder or path prefix; `None` means no folder qualifier (e.g., `[[Note]]` vs. `[[folder/sub/Note]]`).
    pub folder_path: Option<String>,
    /// Optional target note name; `None` indicates a same-file reference (e.g., `[[#Heading]]`).
    pub note_name: Option<String>,
    /// Ordered list of heading names for nested heading references (e.g., `["Design", "API"]` from `[[Note#Design#API]]`).
    /// Empty for links that do not reference headings.
    pub heading_path: Vec<String>,
    /// Optional block reference ID (e.g., `"abc123"` from `[[Note#^abc123]]`).
    /// Mutually exclusive with `heading_path` being non-empty.
    pub block_id: Option<String>,
    /// Optional alias or custom display text (e.g., `"Custom Text"` from `[[Note|Custom Text]]`).
    pub alias: Option<String>,
}

impl Link {
    /// Create a new empty `Link` from a raw string.
    ///
    /// Initializes a `Link` with default values (Wikilink style, not an embed, no parsed components).
    /// This is typically used as a starting point before parsing; full parsing logic is implemented
    /// by the vault resolution pipeline.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw link text to store
    ///
    /// # Example
    ///
    /// ```ignore
    /// let link = Link::new("[[Project Plan#Milestones]]");
    /// assert_eq!(link.raw, "[[Project Plan#Milestones]]");
    /// assert_eq!(link.note_name, None); // Not parsed yet
    /// ```
    pub fn new(raw: impl Into<String>) -> Self {
        Self {
            raw: raw.into(),
            style: LinkStyle::Wikilink,
            is_embed: false,
            folder_path: None,
            note_name: None,
            heading_path: Vec::new(),
            block_id: None,
            alias: None,
        }
    }
}
