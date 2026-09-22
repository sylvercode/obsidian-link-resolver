use obsidian_link_resolver::output::Status;
use obsidian_link_resolver::resolve;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault")
}

fn resolve_case(
    link: &str,
    context_rel: &str,
    vault_rel: Option<&str>,
) -> obsidian_link_resolver::output::ResolutionTarget {
    let fixture_root = fixture_root();
    let context = fixture_root.join(context_rel);
    let vault = vault_rel.map(|rel| fixture_root.join(rel));

    resolve(
        link,
        context.to_string_lossy().as_ref(),
        vault.as_ref().map(|path| path.to_string_lossy().into_owned()).as_deref(),
        false,
    )
}

#[test]
fn resolves_documented_fixture_scenarios() {
    let resolved = resolve_case("[[Project Plan]]", "notes/a.md", None);
    assert_eq!(resolved.status, Status::Resolved);
    assert_eq!(resolved.target_path.as_deref(), Some("Project Plan.md"));
    assert_eq!(resolved.target_line, None);

    let heading = resolve_case("[[Project Plan#Milestones]]", "notes/a.md", None);
    assert_eq!(heading.status, Status::Resolved);
    assert_eq!(heading.target_path.as_deref(), Some("Project Plan.md"));
    assert_eq!(heading.target_line, Some(5));

    let block = resolve_case("[[Project Plan#^abc123]]", "notes/a.md", None);
    assert_eq!(block.status, Status::Resolved);
    assert_eq!(block.target_line, Some(9));

    let same_file = resolve_case("[[#Overview]]", "notes/a.md", None);
    assert_eq!(same_file.status, Status::Resolved);
    assert_eq!(same_file.target_path.as_deref(), Some("notes/a.md"));
    assert_eq!(same_file.target_line, Some(1));

    let unresolved = resolve_case("[[No Such Note]]", "notes/a.md", None);
    assert_eq!(unresolved.status, Status::Unresolved);

    let missing_sub_target = resolve_case("[[Project Plan#Nope]]", "notes/a.md", None);
    assert_eq!(missing_sub_target.status, Status::SubTargetNotFound);
    assert_eq!(missing_sub_target.target_path.as_deref(), Some("Project Plan.md"));

    let aliased = resolve_case("[[Project Plan|Plan]]", "notes/a.md", None);
    assert_eq!(aliased.status, Status::Resolved);
    assert_eq!(aliased.alias.as_deref(), Some("Plan"));

    let embedded = resolve_case("![[Project Plan#Milestones]]", "notes/a.md", None);
    assert_eq!(embedded.status, Status::Resolved);
    assert!(embedded.is_embed);

    let markdown = resolve_case("[t](Project%20Plan.md#Milestones)", "notes/a.md", None);
    assert_eq!(markdown.status, Status::Resolved);
    assert_eq!(markdown.target_path.as_deref(), Some("Project Plan.md"));

    let attachment = resolve_case("![[assets/diagram.png]]", "notes/a.md", None);
    assert_eq!(attachment.status, Status::Resolved);
    assert_eq!(attachment.target_path.as_deref(), Some("assets/diagram.png"));
    assert_eq!(attachment.target_line, None);

    let path_qualified = resolve_case("[[folder/sub/Note#Heading]]", "notes/a.md", None);
    assert_eq!(path_qualified.status, Status::Resolved);
    assert_eq!(path_qualified.target_path.as_deref(), Some("folder/sub/Note.md"));

    let explicit_vault = resolve_case("[[Project Plan]]", "notes/a.md", Some(""));
    assert_eq!(explicit_vault.status, Status::Resolved);
}
