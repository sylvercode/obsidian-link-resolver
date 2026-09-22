use obsidian_link_resolver::vault::{
    detect_root, enumerate_vault, resolve_name, NameResolutionError, NoteIndexEntry, Vault,
    VaultSource,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault")
}

#[test]
fn detects_vault_root_and_resolves_names_case_insensitively() {
    let fixture_root = fixture_root();
    let context = fixture_root.join("notes/a.md");

    let detected = detect_root(context.to_string_lossy().as_ref(), None).expect("vault should detect");
    assert_eq!(detected.root, fixture_root.to_string_lossy());
    assert_eq!(detected.source, VaultSource::Detected);

    let explicit = detect_root(
        context.to_string_lossy().as_ref(),
        Some(fixture_root.to_string_lossy().as_ref()),
    )
    .expect("explicit vault should work");
    assert_eq!(explicit.root, fixture_root.to_string_lossy());
    assert_eq!(explicit.source, VaultSource::Explicit);

    let entries = enumerate_vault(fixture_root.to_string_lossy().as_ref()).expect("vault should enumerate");
    assert!(entries.iter().any(|entry| entry.rel_path == "Project Plan.md"));

    let vault = Vault {
        root: fixture_root.to_string_lossy().into_owned(),
        source: VaultSource::Detected,
        entries,
    };

    let resolved = resolve_name(&vault, "project plan", None).expect("bare name should resolve");
    assert_eq!(resolved.rel_path, "Project Plan.md");

    let path_qualified = resolve_name(&vault, "folder/sub/Note", None).expect("path-qualified name should resolve");
    assert_eq!(path_qualified.rel_path, "folder/sub/Note.md");
}

#[test]
fn reports_ambiguous_and_vault_undetermined_cases() {
    let vault = Vault {
        root: "/tmp/vault".to_string(),
        source: VaultSource::Detected,
        entries: vec![
            NoteIndexEntry {
                rel_path: "b/Note.md".to_string(),
                name: "Note".to_string(),
                is_markdown: true,
            },
            NoteIndexEntry {
                rel_path: "a/Note.md".to_string(),
                name: "Note".to_string(),
                is_markdown: true,
            },
        ],
    };

    let ambiguous = resolve_name(&vault, "Note", None).expect_err("duplicate names should be ambiguous");
    match ambiguous {
        NameResolutionError::Ambiguous { candidates, .. } => {
            assert_eq!(candidates, vec!["a/Note.md", "b/Note.md"]);
        }
        other => panic!("unexpected ambiguity error: {other:?}"),
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("olr-no-vault-{unique}"));
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");
    let context = temp_dir.join("outside.md");
    fs::write(&context, "# Outside\n").expect("context file should be writable");

    assert!(detect_root(context.to_string_lossy().as_ref(), None).is_err());
}
