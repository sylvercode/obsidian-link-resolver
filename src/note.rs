//! Markdown note parsing for headings and block references.
//!
//! This module provides line-based scanning of markdown notes to extract
//! ATX headings (`#` ... `######`) and trailing block IDs (`^id`).
//! The scanner is designed to be fast and proportional to the size of one note,
//! with proper handling of code fences so `#`-like syntax inside code blocks is ignored.

use crate::link::Link;
use crate::output::LineRange;

/// A block reference extracted from a markdown note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteBlockReference {
    /// The block id without the leading `^`.
    pub id: String,
    /// The 1-based line number where the block reference appears.
    pub line: u32,
}

/// The result of scanning a note for headings and block ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteScan {
    /// ATX headings in document order.
    pub headings: Vec<NoteHeading>,
    /// Trailing block references in document order.
    pub block_ids: Vec<NoteBlockReference>,
    /// The total number of lines in the note.
    pub line_count: u32,
}

/// Errors that can occur while locating a target line in a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetLookupError {
    /// The requested heading or block id was not found.
    MissingTarget { reason: String },
    /// The link reference is structurally invalid for target lookup.
    MalformedReference { reason: String },
}

/// A heading extracted from a markdown note.
///
/// Represents an ATX-style heading (`#`, `##`, ..., `######`) with its text content,
/// level, and 1-based line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteHeading {
    /// The heading text without the `#` prefix (e.g., `"Milestones"` from `## Milestones`).
    pub text: String,
    /// The heading level (1–6, corresponding to the number of `#` characters).
    pub level: u8,
    /// The 1-based line number where this heading appears in the note.
    pub line: u32,
}

/// A contiguous line-number range within a note.
///
/// Used to define the span of a heading section or the whole-file range.
/// Line numbers are 1-based and inclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSection {
    /// The 1-based line number where the section begins (inclusive).
    pub begin: u32,
    /// The 1-based line number where the section ends (inclusive).
    pub end: u32,
}

/// Parse a markdown note and extract all ATX headings.
///
/// Scans the note content line-by-line to find all ATX-style headings (`#` ... `######`).
/// Code fences (triple backticks with optional language specifier) are tracked to ensure
/// that `#` characters inside code blocks are not treated as headings.
///
/// # Arguments
///
/// * `contents` - The full text of the markdown note
///
/// # Returns
///
/// A vector of [`NoteHeading`] entries in document order (top to bottom).
/// Empty if the note contains no headings.
///
/// # Example
///
/// ```ignore
/// let content = "# Design\n## API\n### Auth";
/// let headings = parse_note_headings(content);
/// assert_eq!(headings.len(), 3);
/// assert_eq!(headings[0].text, "Design");
/// assert_eq!(headings[0].level, 1);
/// assert_eq!(headings[0].line, 1);
/// ```
pub fn parse_note_headings(contents: &str) -> Vec<NoteHeading> {
    scan_note(contents).headings
}

/// Compute line-number ranges for sections defined by headings.
///
/// Given a note's content, computes the `begin` and `end` line numbers for each heading's section.
/// A heading's section spans from the heading's line to the line before the next heading
/// of equal or higher level (i.e., a heading with `level \u2264` the current heading's level),
/// or to EOF if no such heading follows.
///
/// # Arguments
///
/// * `contents` - The full text of the markdown note
///
/// # Returns
///
/// A vector of [`NoteSection`] entries (one per heading) in document order.
/// Empty if the note contains no headings.
///
/// # Example
///
/// For a note with headings:
/// ```text
/// 1: # Design
/// 2: ## API
/// 3: ### Auth
/// 4: ## Settings
/// 5: # Summary
/// 6: (EOF)
/// ```
/// - Design (level 1): section `[1, 4]` (ends before the next level-1 heading at line 5)
/// - API (level 2): section `[2, 3]` (ends before the next level-2 heading at line 4)
/// - Auth (level 3): section `[3, 3]` (ends before the next level-2 heading at line 4)
/// - Settings (level 2): section `[4, 4]` (ends before the next level-1 heading at line 5)
/// - Summary (level 1): section `[5, 5]` (ends at EOF)
pub fn parse_note_sections(contents: &str) -> Vec<NoteSection> {
    let scan = scan_note(contents);
    heading_sections(&scan.headings, scan.line_count)
}

/// Scan a note for headings and trailing block ids.
pub fn scan_note(contents: &str) -> NoteScan {
    let mut headings = Vec::new();
    let mut block_ids = Vec::new();
    let mut in_code_fence = false;
    let line_count = contents.lines().count() as u32;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index as u32 + 1;
        let trimmed_end = line.trim_end();
        let trimmed = trimmed_end.trim_start();

        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        if in_code_fence {
            continue;
        }

        if let Some((level, text)) = parse_heading(trimmed) {
            headings.push(NoteHeading {
                text,
                level,
                line: line_number,
            });
        }

        if let Some(block_id) = parse_block_id(trimmed_end) {
            block_ids.push(NoteBlockReference {
                id: block_id,
                line: line_number,
            });
        }
    }

    NoteScan {
        headings,
        block_ids,
        line_count,
    }
}

/// Find the target line for a parsed link within the supplied note contents.
pub fn find_target_line(contents: &str, link: &Link) -> Result<Option<u32>, TargetLookupError> {
    find_target_range(contents, link).map(|range| range.map(|value| value.begin))
}

/// Find the canonical target range for a parsed link within the supplied note contents.
pub fn find_target_range(
    contents: &str,
    link: &Link,
) -> Result<Option<LineRange>, TargetLookupError> {
    if link.heading_path.is_empty() && link.block_id.is_none() {
        return Ok(None);
    }

    let scan = scan_note(contents);
    if let Some(block_id) = &link.block_id {
        return scan
            .block_ids
            .into_iter()
            .find(|block| block.id.eq_ignore_ascii_case(block_id.trim()))
            .map(|block| {
                Some(LineRange {
                    begin: block.line,
                    end: block.line,
                })
            })
            .ok_or_else(|| TargetLookupError::MissingTarget {
                reason: format!("block id '^{}' not found", block_id.trim()),
            });
    }

    let heading_sections = heading_sections(&scan.headings, scan.line_count);
    let mut current_index = None;
    let mut current_level = 0;
    let mut search_begin = 1;
    let mut search_end = scan.line_count;

    for segment in &link.heading_path {
        let mut matched = None;
        for (index, heading) in scan.headings.iter().enumerate() {
            if heading.line < search_begin || heading.line > search_end {
                continue;
            }
            if heading.level <= current_level {
                continue;
            }
            if heading.text.trim().eq_ignore_ascii_case(segment.trim()) {
                matched = Some((index, heading));
                break;
            }
        }

        let (index, heading) = matched.ok_or_else(|| TargetLookupError::MissingTarget {
            reason: format!("heading '{}' not found", segment.trim()),
        })?;

        current_index = Some(index);
        current_level = heading.level;
        search_begin = heading.line + 1;
        search_end = heading_sections[index].end;
    }

    current_index
        .map(|index| {
            Some(LineRange {
                begin: scan.headings[index].line,
                end: heading_sections[index].end,
            })
        })
        .ok_or_else(|| TargetLookupError::MalformedReference {
            reason: "link did not contain a heading or block target".to_string(),
        })
}

fn parse_heading(line: &str) -> Option<(u8, String)> {
    let content = line.trim_start();
    let level = content
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&level) {
        return None;
    }

    let after_hashes = &content[level..];
    if !after_hashes.starts_with(' ') && !after_hashes.is_empty() {
        return None;
    }

    let text = after_hashes.trim();
    Some((level as u8, text.to_string()))
}

fn parse_block_id(line: &str) -> Option<String> {
    let trimmed = line.trim_end();
    let caret = trimmed.rfind('^')?;
    if caret > 0 {
        let before = trimmed.as_bytes()[caret - 1];
        if !before.is_ascii_whitespace() {
            return None;
        }
    }

    let block_id = trimmed[caret + 1..].trim();
    if block_id.is_empty() || block_id.chars().any(char::is_whitespace) {
        return None;
    }

    Some(block_id.to_string())
}

fn heading_sections(headings: &[NoteHeading], line_count: u32) -> Vec<NoteSection> {
    let mut sections = Vec::with_capacity(headings.len());
    for (index, heading) in headings.iter().enumerate() {
        let mut end = line_count;
        for next_heading in headings.iter().skip(index + 1) {
            if next_heading.level <= heading.level {
                end = next_heading.line.saturating_sub(1);
                break;
            }
        }
        sections.push(NoteSection {
            begin: heading.line,
            end,
        });
    }
    sections
}
