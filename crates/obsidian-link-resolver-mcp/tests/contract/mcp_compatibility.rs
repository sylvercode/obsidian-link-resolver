//! Tests for the versioned MCP contract artifact and compatibility checker.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Verify the machine-readable contract follows the workspace version and resolver schema.
#[test]
fn contract_artifact_matches_workspace_and_resolver_outcomes() {
    let root = repository_root();
    let contract: Value = serde_json::from_slice(
        &fs::read(root.join("specs/002-mcp-wrapper/contracts/mcp-contract.json"))
            .expect("MCP contract artifact should exist"),
    )
    .expect("MCP contract artifact should be JSON");
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("workspace manifest exists");
    let workspace_version = manifest
        .split("[workspace.package]")
        .nth(1)
        .and_then(|section| {
            section.lines().find_map(|line| {
                line.trim()
                    .strip_prefix("version = \"")
                    .and_then(|value| value.strip_suffix('\"'))
            })
        })
        .expect("workspace package version should be declared");
    assert_eq!(contract["contract_version"], workspace_version);

    let result_schema: Value = serde_json::from_slice(
        &fs::read(root.join("specs/001-link-resolver/contracts/result.schema.json"))
            .expect("resolver result schema should exist"),
    )
    .expect("resolver result schema should be JSON");
    let artifact_fields = contract["tool"]["result"]["fields"]
        .as_object()
        .expect("contract should list result fields");
    let schema_fields = result_schema["properties"]
        .as_object()
        .expect("resolver schema should list result properties");
    assert_eq!(artifact_fields.len(), schema_fields.len());
    for field in schema_fields.keys() {
        assert!(artifact_fields.contains_key(field), "missing field {field}");
    }

    let outcomes = contract["tool"]["result"]["outcomes"]
        .as_object()
        .expect("contract should list resolver outcomes");
    assert_eq!(
        outcomes.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "ambiguous",
            "error",
            "resolved",
            "sub_target_not_found",
            "unresolved"
        ]
    );
}

/// Check additive changes and breaking changes against a published same-major baseline.
#[test]
fn compatibility_checker_accepts_additions_and_rejects_breaking_changes() {
    let root = repository_root();
    let contract_path = root.join("specs/002-mcp-wrapper/contracts/mcp-contract.json");
    let baseline: Value = serde_json::from_slice(
        &fs::read(&contract_path).expect("MCP contract artifact should exist"),
    )
    .expect("MCP contract artifact should be JSON");

    let mut additive = baseline.clone();
    additive["tool"]["arguments"]["include_metadata"] = json!({
        "type": "boolean",
        "required": false,
        "meaning": "Optional metadata request."
    });
    additive["tool"]["result"]["fields"]["metadata"] = json!({
        "type": "object",
        "required": false,
        "meaning": "Optional metadata."
    });
    assert!(run_checker(&root, &additive, &baseline, "")
        .status
        .success());

    let mut removed_argument = baseline.clone();
    removed_argument["tool"]["arguments"]
        .as_object_mut()
        .expect("arguments should be an object")
        .remove("link");
    assert!(!run_checker(&root, &removed_argument, &baseline, "")
        .status
        .success());

    let mut changed_meaning = baseline.clone();
    changed_meaning["tool"]["result"]["fields"]["status"]["meaning"] =
        json!("A different outcome meaning.");
    assert!(!run_checker(&root, &changed_meaning, &baseline, "")
        .status
        .success());

    let mut major_break = removed_argument;
    major_break["contract_version"] = json!("3.0.0");
    let missing_migration = run_checker(&root, &major_break, &baseline, "");
    assert!(
        !missing_migration.status.success(),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&missing_migration.stdout),
        String::from_utf8_lossy(&missing_migration.stderr)
    );
    assert!(run_checker(
        &root,
        &major_break,
        &baseline,
        "## 2.x to 3.0.0\nBreaking MCP contract changes.\n"
    )
    .status
    .success());
}

#[test]
fn published_baseline_gate_selects_latest_same_major_and_cannot_be_skipped() {
    let root = repository_root();
    let baseline: Value = serde_json::from_slice(
        &fs::read(root.join("specs/002-mcp-wrapper/contracts/mcp-contract.json")).unwrap(),
    )
    .unwrap();
    let mut published = baseline.clone();
    published["contract_version"] = json!("2.10.0");
    let mut candidate = published.clone();
    candidate["contract_version"] = json!("2.11.0");
    let releases = json!([
        [
            {"tag_name": "v3.0.0", "assets": [{"name": "mcp-contract.json", "id": 30}]},
            {"tag_name": "v2.9.0", "assets": [{"name": "mcp-contract.json", "id": 29}]},
            {"tag_name": "v2.99.0", "draft": true, "assets": [{"name": "mcp-contract.json", "id": 99}]},
            {"tag_name": "v2.98.0", "prerelease": true, "assets": [{"name": "mcp-contract.json", "id": 98}]}
        ],
        [
            {"tag_name": "v2.10.0", "assets": [{"name": "mcp-contract.json", "id": 210}]},
            {"tag_name": "v2.12.0", "assets": []}
        ]
    ]);
    let mut fixture = json!({
        "mode": "published", "pages": releases, "assets": {"210": published}, "exit_code": 0
    });
    let output = run_checker_with_publication(&root, &candidate, &baseline, "", Some(&fixture));
    assert!(output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Selected published MCP contract v2.10.0")
    );

    fixture["mode"] = json!("-");
    let output = run_checker_with_publication(&root, &candidate, &baseline, "", Some(&fixture));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("only allowed when no published"));

    candidate["tool"]["arguments"]
        .as_object_mut()
        .unwrap()
        .remove("link");
    fixture["mode"] = json!("published");
    assert!(
        !run_checker_with_publication(&root, &candidate, &baseline, "", Some(&fixture))
            .status
            .success()
    );

    candidate["contract_version"] = json!("4.0.0");
    fixture["pages"] =
        json!([[{"tag_name": "v2.10.0", "assets": [{"name": "mcp-contract.json", "id": 210}]}]]);
    assert!(
        !run_checker_with_publication(&root, &candidate, &baseline, "", Some(&fixture))
            .status
            .success()
    );
    assert!(run_checker_with_publication(
        &root,
        &candidate,
        &baseline,
        "## 2.x to 4.0.0\nMigration notes.\n",
        Some(&fixture)
    )
    .status
    .success());

    fixture["assets"]["210"]["contract_version"] = json!("2.9.0");
    assert!(
        !run_checker_with_publication(&root, &candidate, &baseline, "", Some(&fixture))
            .status
            .success()
    );
    fixture["assets"] = json!({});
    assert!(
        !run_checker_with_publication(&root, &candidate, &baseline, "", Some(&fixture))
            .status
            .success()
    );
}

#[test]
fn published_baseline_gate_allows_first_contract_but_fails_closed_on_api_errors() {
    let root = repository_root();
    let contract: Value = serde_json::from_slice(
        &fs::read(root.join("specs/002-mcp-wrapper/contracts/mcp-contract.json")).unwrap(),
    )
    .unwrap();
    let mut fixture = json!({
        "mode": "-", "pages": [[{"tag_name": "v1.1.0", "assets": []}]], "assets": {}, "exit_code": 0
    });
    assert!(
        run_checker_with_publication(&root, &contract, &contract, "", Some(&fixture))
            .status
            .success()
    );
    fixture["mode"] = json!("published");
    assert!(
        run_checker_with_publication(&root, &contract, &contract, "", Some(&fixture))
            .status
            .success()
    );
    for mode in ["-", "published"] {
        fixture["mode"] = json!(mode);
        fixture["exit_code"] = json!(1);
        assert!(
            !run_checker_with_publication(&root, &contract, &contract, "", Some(&fixture))
                .status
                .success()
        );
    }
    fixture["exit_code"] = json!(0);
    for invalid_pages in [json!(null), json!({}), json!([]), json!([[{}]])] {
        fixture["pages"] = invalid_pages;
        assert!(
            !run_checker_with_publication(&root, &contract, &contract, "", Some(&fixture))
                .status
                .success()
        );
    }

    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    let gate = workflow
        .find("mcp-contract.json published MIGRATION.md")
        .unwrap();
    assert!(gate < workflow.find("name: Build release artifacts").unwrap());
    assert!(gate < workflow.find("name: Stage release assets").unwrap());
    assert!(workflow.contains("GH_TOKEN: ${{ github.token }}"));
    assert!(workflow
        .contains("cp specs/002-mcp-wrapper/contracts/mcp-contract.json dist/mcp-contract.json"));
}

/// Invoke the compatibility checker with explicit candidate, baseline, and migration paths.
///
/// # Parameters
///
/// - `root`: Repository root containing the checker script.
/// - `candidate`: Candidate contract content.
/// - `baseline`: Published contract content used as the comparison baseline.
/// - `migration_notes`: Migration-note content for major-version changes.
///
/// # Returns
///
/// The completed compatibility-check process output.
fn run_checker(
    root: &Path,
    candidate: &Value,
    baseline: &Value,
    migration_notes: &str,
) -> std::process::Output {
    run_checker_with_publication(root, candidate, baseline, migration_notes, None)
}

fn run_checker_with_publication(
    root: &Path,
    candidate: &Value,
    baseline: &Value,
    migration_notes: &str,
    publication: Option<&Value>,
) -> std::process::Output {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let temporary = std::env::temp_dir().join(format!(
        "mcp-contract-check-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&temporary).expect("temporary contract directory should be created");
    let candidate_path = temporary.join("candidate.json");
    let baseline_path = temporary.join("baseline.json");
    let migration_path = temporary.join("MIGRATION.md");
    fs::write(&candidate_path, serde_json::to_vec(candidate).unwrap())
        .expect("candidate contract should be written");
    fs::write(&baseline_path, serde_json::to_vec(baseline).unwrap())
        .expect("baseline contract should be written");
    fs::write(&migration_path, migration_notes).expect("migration notes should be written");

    let mut command = Command::new("pwsh");
    command.arg("-NoProfile");
    if let Some(fixture) = publication {
        let fixture_path = temporary.join("publication.json");
        fs::write(&fixture_path, serde_json::to_vec(fixture).unwrap()).unwrap();
        command.args(["-Command", r#"
            function global:gh {
                $Fixture = Get-Content -Raw $env:TEST_PUBLICATION | ConvertFrom-Json -AsHashtable
                $global:LASTEXITCODE = $Fixture['exit_code']
                if ($global:LASTEXITCODE -ne 0) { return }
                $Endpoint = $args[-1]
                if ($Endpoint -ceq 'repos/example/project/releases?per_page=100') {
                    if ($args -notcontains '--paginate' -or $args -notcontains '--slurp') { throw 'pagination required' }
                    ConvertTo-Json -InputObject $Fixture['pages'] -Depth 100 -Compress
                } elseif ($Endpoint -match '^repos/example/project/releases/assets/(\d+)$') {
                    if ($args -notcontains 'Accept: application/octet-stream') { throw 'asset download required' }
                    $Asset = $Fixture['assets'][$Matches[1]]
                    if ($null -eq $Asset) { $global:LASTEXITCODE = 1; return }
                    ConvertTo-Json -InputObject $Asset -Depth 100 -Compress
                } else { throw "unexpected gh call: $args" }
            }
            & $env:TEST_CHECKER $env:TEST_CANDIDATE $env:TEST_MODE $env:TEST_MIGRATION -Repository example/project
        "#]);
        command
            .env("TEST_PUBLICATION", fixture_path)
            .env(
                "TEST_CHECKER",
                root.join("scripts/check-mcp-contract-compatibility.ps1"),
            )
            .env("TEST_CANDIDATE", &candidate_path)
            .env("TEST_MODE", fixture["mode"].as_str().unwrap())
            .env("TEST_MIGRATION", &migration_path);
    } else {
        command
            .arg("-File")
            .arg(root.join("scripts/check-mcp-contract-compatibility.ps1"))
            .arg(&candidate_path)
            .arg(&baseline_path)
            .arg(&migration_path);
    }
    let output = command.output().expect("compatibility checker should run");
    fs::remove_dir_all(temporary).expect("temporary contract directory should be removed");
    output
}

/// Return the workspace root based on the MCP crate manifest directory.
///
/// # Returns
///
/// The absolute repository root path.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
