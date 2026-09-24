//! Obsidian link parsing and representation.
//!
//! This module defines the [`Link`] entity, which represents a parsed Obsidian link
//! in a canonical form. It supports both wikilink (`[[...]]`) and markdown-style (`[text](...)`) link formats,
//! embeds, aliases, folder paths, headings, and block references.

use serde::{Deserialize, Serialize};

use crate::vault::is_supported_linkable_name;

type ParsedTarget = (Option<String>, Option<String>, Vec<String>, Option<String>);

/// Error returned when parsing an Obsidian link fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkParseError {
    /// Human-readable parse failure description.
    pub message: String,
}

impl core::fmt::Display for LinkParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LinkParseError {}

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
/// into its semantic components: optional folder path, note name, heading path, and block reference.
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
    /// Optional display text read from a wikilink display text or markdown label.
    pub display_text: Option<String>,
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
            display_text: None,
        }
    }
}

/// Parse a raw Obsidian link string into the canonical [`Link`] representation.
pub fn parse_link(raw: &str) -> Result<Link, LinkParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LinkParseError {
            message: "link is empty".to_string(),
        });
    }

    if let Some(inner) = trimmed
        .strip_prefix("![[")
        .and_then(|value| value.strip_suffix("]]"))
    {
        return parse_wikilink(raw, inner, true);
    }

    if let Some(inner) = trimmed
        .strip_prefix("[[")
        .and_then(|value| value.strip_suffix("]]"))
    {
        return parse_wikilink(raw, inner, false);
    }

    if let Some((prefix, is_embed)) = trimmed.strip_prefix("!").map(|value| (value, true)) {
        if let Some(inner) = prefix
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(')'))
        {
            return parse_markdown(raw, inner, is_embed);
        }
    }

    if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_markdown(raw, inner, false);
    }

    Err(LinkParseError {
        message: format!("unsupported link syntax: {raw}"),
    })
}

fn parse_wikilink(raw: &str, inner: &str, is_embed: bool) -> Result<Link, LinkParseError> {
    let mut parts = inner.splitn(2, '|');
    let target = parts.next().unwrap_or_default().trim();
    let display_text = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let (folder_path, note_name, heading_path, block_id) = parse_target(target)?;

    Ok(Link {
        raw: raw.to_string(),
        style: LinkStyle::Wikilink,
        is_embed,
        folder_path,
        note_name,
        heading_path,
        block_id,
        display_text,
    })
}

fn parse_markdown(raw: &str, inner: &str, is_embed: bool) -> Result<Link, LinkParseError> {
    let open_paren = inner.find("](").ok_or_else(|| LinkParseError {
        message: format!("unsupported markdown link syntax: {raw}"),
    })?;
    let display = inner[..open_paren].trim();
    let target = &inner[open_paren + 2..];
    let decoded = percent_decode(target.trim());
    let (folder_path, note_name, heading_path, block_id) = parse_target(&decoded)?;

    let display_text = (!display.is_empty()).then(|| display.to_string());
    Ok(Link {
        raw: raw.to_string(),
        style: LinkStyle::Markdown,
        is_embed,
        folder_path,
        note_name,
        heading_path,
        block_id,
        display_text,
    })
}

fn parse_target(target: &str) -> Result<ParsedTarget, LinkParseError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(LinkParseError {
            message: "link target is empty".to_string(),
        });
    }

    let (note_part, subtarget_part) = if let Some(stripped) = target.strip_prefix('#') {
        (None, Some(stripped))
    } else if let Some((note, subtarget)) = target.split_once('#') {
        (Some(note), Some(subtarget))
    } else {
        (Some(target), None)
    };

    let (folder_path, note_name) = match note_part.map(str::trim).filter(|value| !value.is_empty())
    {
        Some(note_part) => {
            if let Some((folder, note)) = note_part.rsplit_once('/') {
                let folder = folder.trim();
                let note = note.trim();
                if note.is_empty() {
                    return Err(LinkParseError {
                        message: format!("missing note name in link target: {target}"),
                    });
                }
                if !is_supported_linkable_name(note) {
                    return Err(LinkParseError {
                        message: format!("invalid note name in link target: {target}"),
                    });
                }
                (
                    (!folder.is_empty()).then(|| folder.to_string()),
                    Some(note.to_string()),
                )
            } else {
                if !is_supported_linkable_name(note_part) {
                    return Err(LinkParseError {
                        message: format!("invalid note name in link target: {target}"),
                    });
                }
                (None, Some(note_part.to_string()))
            }
        }
        None => (None, None),
    };

    let mut heading_path = Vec::new();
    let mut block_id = None;

    if let Some(subtarget) = subtarget_part
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let segments: Vec<&str> = subtarget.split('#').collect();
        for (index, segment) in segments.iter().enumerate() {
            let segment = segment.trim();
            if segment.is_empty() {
                return Err(LinkParseError {
                    message: format!("empty sub-target segment in link target: {target}"),
                });
            }
            if let Some(block) = segment.strip_prefix('^') {
                if index != segments.len() - 1 || !heading_path.is_empty() {
                    return Err(LinkParseError {
                        message: format!(
                            "a link cannot target both a heading and a block id: {target}"
                        ),
                    });
                }
                if block.trim().is_empty() {
                    return Err(LinkParseError {
                        message: format!("block id is empty in link target: {target}"),
                    });
                }
                block_id = Some(block.trim().to_string());
            } else {
                if block_id.is_some() {
                    return Err(LinkParseError {
                        message: format!(
                            "a link cannot target both a heading and a block id: {target}"
                        ),
                    });
                }
                heading_path.push(segment.to_string());
            }
        }
    }

    if note_name.is_none() && heading_path.is_empty() && block_id.is_none() {
        return Err(LinkParseError {
            message: format!("a same-file link must target a heading or block id: {target}"),
        });
    }

    Ok((folder_path, note_name, heading_path, block_id))
}

fn percent_decode(input: &str) -> String {
    urlencoding::decode(input)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| input.to_string())
}
