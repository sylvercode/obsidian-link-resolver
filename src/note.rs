#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteHeading {
    pub text: String,
    pub level: u8,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSection {
    pub begin: u32,
    pub end: u32,
}

pub fn parse_note_headings(_contents: &str) -> Vec<NoteHeading> {
    Vec::new()
}

pub fn parse_note_sections(_contents: &str) -> Vec<NoteSection> {
    Vec::new()
}
