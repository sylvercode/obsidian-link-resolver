use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Resolved,
    Unresolved,
    SubTargetNotFound,
    Ambiguous,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    pub begin: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadingRef {
    pub text: String,
    pub level: u8,
    pub begin: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredEmplacement {
    pub heading_stack: Vec<HeadingRef>,
    pub section: LineRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionTarget {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_line: Option<u32>,
    pub is_embed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emplacement: Option<StructuredEmplacement>,
}

impl ResolutionTarget {
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
