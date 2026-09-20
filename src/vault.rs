use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultSource {
    #[serde(rename = "explicit")]
    Explicit,
    #[serde(rename = "detected")]
    Detected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteIndexEntry {
    pub rel_path: String,
    pub name: String,
    pub is_markdown: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    pub root: String,
    pub source: VaultSource,
    pub entries: Vec<NoteIndexEntry>,
}
