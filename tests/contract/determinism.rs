use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

/// Run the resolver once and return raw stdout for byte-stability checks.
///
/// # Parameters
///
/// - `link`: Link string passed to the resolver CLI.
/// - `context`: Context note path passed to `--context`.
///
/// # Returns
///
/// Raw stdout emitted by the resolver process.
fn resolve_once(link: &str, context: &str) -> String {
    let mut command = cargo_bin_cmd!("obsidian-link-resolver");
    command.args([link, "--context", context, "--format", "json"]);

    let output = command.output().expect("resolver command should run");
    assert!(output.status.success(), "resolver should succeed");
    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

/// Verifies identical inputs produce byte-identical machine JSON output.
#[test]
fn machine_mode_stdout_is_byte_stable_for_identical_inputs() {
    let fixture_root = format!("{}/tests/fixtures/vault", env!("CARGO_MANIFEST_DIR"));
    let context = format!("{}/notes/a.md", fixture_root);

    let first = resolve_once("[[Project Plan#Milestones]]", &context);
    let second = resolve_once("[[Project Plan#Milestones]]", &context);

    assert_eq!(
        first, second,
        "identical inputs must produce identical JSON"
    );
    let parsed: Value = serde_json::from_str(first.trim()).expect("stdout should be valid JSON");
    assert_eq!(parsed["status"], "resolved");
    assert_eq!(parsed["target_path"], "Project Plan.md");
}
