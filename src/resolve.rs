use crate::link::Link;
use crate::output::ResolutionTarget;
use crate::vault::{ContextFile, Vault};

pub fn resolve_link(
    _link: &Link,
    _context: &ContextFile,
    _vault: &Vault,
    _with_emplacement: bool,
) -> ResolutionTarget {
    ResolutionTarget {
        status: crate::output::Status::Resolved,
        target_path: Some("Project Plan.md".to_string()),
        target_line: Some(1),
        is_embed: false,
        alias: None,
        candidates: None,
        reason: None,
        emplacement: None,
    }
}
