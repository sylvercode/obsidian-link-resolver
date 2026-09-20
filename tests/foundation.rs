use obsidian_link_resolver::link::{Link, LinkStyle};
use obsidian_link_resolver::output::{ResolutionTarget, Status};
use obsidian_link_resolver::vault::{ContextFile, NoteIndexEntry, Vault, VaultSource};

#[test]
fn foundational_types_match_phase2_contracts() {
    let link = Link {
        raw: "[[Project Plan#Milestones]]".to_string(),
        style: LinkStyle::Wikilink,
        is_embed: false,
        folder_path: None,
        note_name: Some("Project Plan".to_string()),
        heading_path: vec!["Milestones".to_string()],
        block_id: None,
        alias: None,
    };

    assert_eq!(link.note_name.as_deref(), Some("Project Plan"));
    assert_eq!(link.heading_path, vec!["Milestones"]);

    let vault = Vault {
        root: "/tmp/vault".to_string(),
        source: VaultSource::Detected,
        entries: vec![NoteIndexEntry {
            rel_path: "Project Plan.md".to_string(),
            name: "Project Plan".to_string(),
            is_markdown: true,
        }],
    };
    assert_eq!(vault.entries.len(), 1);

    let context = ContextFile {
        path: "/tmp/vault/notes/a.md".to_string(),
    };
    assert_eq!(context.path, "/tmp/vault/notes/a.md");

    let result = ResolutionTarget {
        status: Status::Resolved,
        target_path: Some("Project Plan.md".to_string()),
        target_line: Some(12),
        is_embed: false,
        alias: None,
        candidates: None,
        reason: None,
        emplacement: None,
    };

    assert_eq!(result.exit_code(), 0);
    assert_eq!(result.status, Status::Resolved);
}
