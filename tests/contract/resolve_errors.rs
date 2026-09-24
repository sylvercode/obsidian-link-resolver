use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn run_resolver(link: &str, context: &str) -> (std::process::Output, Value) {
    let output = cargo_bin_cmd!("obsidian-link-resolver")
        .args([link, "--context", context, "--format", "json"])
        .output()
        .expect("resolver command should run");
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8");
    let json: Value = serde_json::from_str(stdout.trim_end()).expect("stdout should be valid json");
    (output, json)
}

#[test]
fn reports_unresolved_and_sub_target_not_found_via_exit_codes() {
    let fixture_root = format!("{}/tests/fixtures/vault", env!("CARGO_MANIFEST_DIR"));
    let context = format!("{}/notes/a.md", fixture_root);

    let (output, json) = run_resolver("[[No Such Note]]", &context);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json["status"], "unresolved");
    assert!(json["reason"]
        .as_str()
        .is_some_and(|reason| !reason.is_empty()));

    let (output, json) = run_resolver("[[Project Plan#Nope]]", &context);
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(json["status"], "sub_target_not_found");
    assert_eq!(json["target_path"], "Project Plan.md");
    assert!(json["reason"]
        .as_str()
        .is_some_and(|reason| !reason.is_empty()));
}

#[test]
fn reports_error_when_vault_cannot_be_determined() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("olr-outside-vault-{unique}"));
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");
    let context = temp_dir.join("outside.md");
    fs::write(&context, "# Outside\n").expect("context file should be writable");

    let (output, json) = run_resolver("[[X]]", context.to_string_lossy().as_ref());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(json["status"], "error");
    assert!(json["reason"]
        .as_str()
        .is_some_and(|reason| !reason.is_empty()));
}

#[test]
fn ignores_unsupported_extensions_and_dot_prefixed_paths_when_enumerating_vaults() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("olr-vault-filter-{unique}"));
    fs::create_dir_all(temp_dir.join(".obsidian")).expect("vault root should be creatable");
    fs::create_dir_all(temp_dir.join(".hidden")).expect("dot-prefixed folder should be creatable");
    fs::create_dir_all(temp_dir.join("nested")).expect("nested folder should be creatable");

    fs::write(temp_dir.join("good.md"), "# Good\n").expect("valid note should be writable");
    fs::write(temp_dir.join("nested/also-good.md"), "# Also\n")
        .expect("nested valid note should be writable");
    fs::write(temp_dir.join(".hidden/ignored.md"), "# Hidden\n")
        .expect("dotfile note should be writable");
    fs::write(temp_dir.join("nested/.nested-note.md"), "# Also hidden\n")
        .expect("dotfile note should be writable");
    fs::write(temp_dir.join("nested/bad*name.md"), "# Invalid\n")
        .expect("invalid note should be writable");
    fs::write(temp_dir.join("nested/unsupported.xyz"), "# Invalid ext\n")
        .expect("unsupported extension should be writable");

    let context = temp_dir.join("good.md");
    let (output, json) = run_resolver("[[ignored]]", context.to_string_lossy().as_ref());
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json["status"], "unresolved");

    let (output, json) = run_resolver("[[good]]", context.to_string_lossy().as_ref());
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(json["status"], "resolved");

    let (output, json) = run_resolver("[[nested/also-good]]", context.to_string_lossy().as_ref());
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(json["status"], "resolved");
}
