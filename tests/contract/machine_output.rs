use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn ambiguous_results_are_sorted_and_validated_against_schema() {
    let temp = TempDir::new().expect("temp dir should be creatable");
    let vault_root = temp.path();
    let context_dir = vault_root.join("notes");
    std::fs::create_dir_all(&context_dir).expect("context dir should be creatable");
    std::fs::create_dir_all(vault_root.join("a")).expect("a dir should be creatable");
    std::fs::create_dir_all(vault_root.join("b")).expect("b dir should be creatable");

    let context = context_dir.join("a.md");
    std::fs::write(&context, "# Context\n").expect("context file should be writable");
    std::fs::write(vault_root.join("a/Note.md"), "# A\n").expect("first note should be writable");
    std::fs::write(vault_root.join("b/Note.md"), "# B\n").expect("second note should be writable");
    std::fs::create_dir_all(vault_root.join(".obsidian"))
        .expect(".obsidian dir should be creatable");

    let mut command = cargo_bin_cmd!("obsidian-link-resolver");
    command.args([
        "[[Note]]",
        "--context",
        context.to_string_lossy().as_ref(),
        "--vault",
        vault_root.to_string_lossy().as_ref(),
        "--format",
        "json",
    ]);

    let output = command.output().expect("resolver command should run");
    assert!(
        !output.status.success(),
        "ambiguous result should exit non-zero"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON");

    assert_eq!(parsed["status"], "ambiguous");
    assert_eq!(
        parsed["candidates"],
        Value::Array(vec!["a/Note.md".into(), "b/Note.md".into()])
    );
    assert!(parsed["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("multiple"));
}
