use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

fn run_resolver(link: &str, context: &str, extra_args: &[&str]) -> (std::process::Output, Value) {
    let mut command = cargo_bin_cmd!("obsidian-link-resolver");
    command.args([link, "--context", context, "--format", "json"]);
    command.args(extra_args);

    let output = command.output().expect("resolver command should run");
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8");
    let json: Value = serde_json::from_str(stdout.trim_end()).expect("stdout should be valid json");
    (output, json)
}

#[test]
fn resolves_primary_link_forms_to_single_line_json() {
    let fixture_root = format!("{}/tests/fixtures/vault", env!("CARGO_MANIFEST_DIR"));
    let context = format!("{}/notes/a.md", fixture_root);

    let cases = [
        ("[[Project Plan]]", "Project Plan.md", None),
        ("[[Project Plan#Milestones]]", "Project Plan.md", Some(5)),
        ("[[Project Plan#^abc123]]", "Project Plan.md", Some(9)),
        ("[[#Overview]]", "notes/a.md", Some(1)),
    ];

    for (link, target_path, target_line) in cases {
        let (output, json) = run_resolver(link, &context, &[]);
        assert!(output.status.success(), "{link} should succeed");

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let trimmed = stdout.trim_end_matches('\n');
        assert_eq!(
            trimmed.lines().count(),
            1,
            "stdout should be single-line json"
        );

        assert_eq!(json["status"], "resolved");
        assert_eq!(json["target_path"], target_path);
        assert_eq!(
            json["target_line"],
            target_line.map(Value::from).unwrap_or(Value::Null)
        );
        assert_eq!(json["is_embed"], false);
        assert_eq!(json["display_text"], Value::Null);
    }
}

#[test]
fn returns_target_range_for_heading_and_block_targets() {
    let fixture_root = format!("{}/tests/fixtures/vault", env!("CARGO_MANIFEST_DIR"));
    let context = format!("{}/notes/a.md", fixture_root);

    let (output, json) = run_resolver("[[Project Plan#Milestones]]", &context, &[]);
    assert!(output.status.success());
    assert_eq!(json["status"], "resolved");
    assert_eq!(json["target_line"], 5);
    assert_eq!(json["target_range"]["begin"], 5);
    assert_eq!(json["target_range"]["end"], 11);

    let (output, json) = run_resolver("[[Project Plan#^abc123]]", &context, &[]);
    assert!(output.status.success());
    assert_eq!(json["status"], "resolved");
    assert_eq!(json["target_line"], 9);
    assert_eq!(json["target_range"]["begin"], 9);
    assert_eq!(json["target_range"]["end"], 9);
}
