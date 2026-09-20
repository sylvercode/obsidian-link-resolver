use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStyle {
    #[serde(rename = "wikilink")]
    Wikilink,
    #[serde(rename = "markdown")]
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub raw: String,
    pub style: LinkStyle,
    pub is_embed: bool,
    pub folder_path: Option<String>,
    pub note_name: Option<String>,
    pub heading_path: Vec<String>,
    pub block_id: Option<String>,
    pub alias: Option<String>,
}

impl Link {
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
