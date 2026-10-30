//! Integration tests for startup-configured server information.

use super::mcp_client::{context_file, StdioClient};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Verify one, two, and three configured roots are reported in startup order before indexing.
#[test]
fn reports_one_two_and_three_vaults_in_startup_order_without_writes() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let roots = ["first", "second", "third"]
        .into_iter()
        .map(|name| temporary_directory.path().join(name))
        .collect::<Vec<_>>();
    for root in &roots {
        fs::create_dir_all(root.join("nested")).expect("vault directory should be created");
        fs::write(root.join("nested/Note.md"), "# Note\n").expect("vault note should be written");
    }
    let snapshots = roots
        .iter()
        .map(|root| snapshot_directory(root))
        .collect::<Vec<_>>();

    for count in 1..=roots.len() {
        let supplied = roots[..count]
            .iter()
            .map(|root| root.to_string_lossy().into_owned())
            .collect();
        let expected = roots[..count]
            .iter()
            .map(|root| canonical_string(root))
            .collect::<Vec<_>>();
        let mut client = StdioClient::with_vaults(supplied);
        assert!(client.initialize().get("result").is_some());
        client.notify_initialized();

        let response = server_info(&mut client, 2);
        assert_eq!(paths(&response), expected);
        assert_eq!(
            roots
                .iter()
                .map(|root| snapshot_directory(root))
                .collect::<Vec<_>>(),
            snapshots
        );
    }
}

/// Verify duplicate canonical roots and a symlink alias are returned only once.
#[cfg(unix)]
#[test]
fn removes_duplicate_and_symlink_alias_roots() {
    use std::os::unix::fs::symlink;

    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let vault = temporary_directory.path().join("vault");
    let alias = temporary_directory.path().join("alias");
    fs::create_dir_all(&vault).expect("vault directory should be created");
    if symlink(&vault, &alias).is_err() {
        return;
    }
    let mut client = StdioClient::with_vaults(vec![
        vault.to_string_lossy().into_owned(),
        vault.to_string_lossy().into_owned(),
        alias.to_string_lossy().into_owned(),
    ]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = server_info(&mut client, 2);
    assert_eq!(paths(&response), [canonical_string(&vault)]);
}

/// Verify a vault removed after startup remains in the configured-root response.
#[test]
fn keeps_vaults_listed_after_their_directories_are_removed() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let roots = [
        temporary_directory.path().join("first"),
        temporary_directory.path().join("second"),
    ];
    for root in &roots {
        fs::create_dir_all(root).expect("vault directory should be created");
    }
    let expected = roots
        .iter()
        .map(|root| canonical_string(root))
        .collect::<Vec<_>>();
    let mut client = StdioClient::with_vaults(
        roots
            .iter()
            .map(|root| root.to_string_lossy().into_owned())
            .collect(),
    );
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();
    fs::remove_dir_all(&roots[1]).expect("second vault should be removed");

    let response = server_info(&mut client, 2);
    assert_eq!(paths(&response), expected);
}

/// Verify canonical paths preserve spaces and non-ASCII characters.
#[test]
fn preserves_spaces_and_non_ascii_in_canonical_paths() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let vault = temporary_directory.path().join("notes espace é");
    fs::create_dir_all(&vault).expect("vault directory should be created");
    let mut client = StdioClient::with_vaults(vec![vault.to_string_lossy().into_owned()]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = server_info(&mut client, 2);
    assert_eq!(paths(&response), [canonical_string(&vault)]);
}

/// Verify duplicate vault arguments do not change existing resolve or generation results.
#[test]
fn duplicate_roots_preserve_existing_tool_results() {
    let root = PathBuf::from(context_file())
        .parent()
        .and_then(Path::parent)
        .expect("fixture context should be nested under its vault")
        .to_path_buf();
    let root_string = root.to_string_lossy().into_owned();
    let mut single = StdioClient::with_vaults(vec![root_string.clone()]);
    let mut duplicate = StdioClient::with_vaults(vec![root_string.clone(), root_string.clone()]);
    assert!(single.initialize().get("result").is_some());
    assert!(duplicate.initialize().get("result").is_some());
    single.notify_initialized();
    duplicate.notify_initialized();

    let resolve_arguments = json!({
        "link": "[[Project Plan#Milestones]]",
        "context_path": context_file()
    });
    let single_resolve = single.call_tool(2, resolve_arguments.clone());
    let duplicate_resolve = duplicate.call_tool(2, resolve_arguments);
    assert_eq!(
        single_resolve["result"]["structuredContent"],
        duplicate_resolve["result"]["structuredContent"]
    );

    let generation_params = json!({
        "name": "generate_obsidian_link",
        "arguments": {
            "context_path": context_file(),
            "target_path": "Project Plan.md",
            "intent": "existing",
            "heading": "Milestones"
        }
    });
    let single_generation = single.request(3, "tools/call", generation_params.clone());
    let duplicate_generation = duplicate.request(3, "tools/call", generation_params);
    assert_eq!(
        single_generation["result"]["structuredContent"],
        duplicate_generation["result"]["structuredContent"]
    );
}

/// Call `get_server_info` and return its structured result.
fn server_info(client: &mut StdioClient, id: u64) -> Value {
    let response = client.request(
        id,
        "tools/call",
        json!({"name": "get_server_info", "arguments": {}}),
    );
    assert_eq!(response["result"]["isError"], false, "{response}");
    response["result"]["structuredContent"].clone()
}

/// Return the configured vault paths from a server-info response.
fn paths(response: &Value) -> Vec<String> {
    response["vaults"]
        .as_array()
        .expect("vaults should be an array")
        .iter()
        .map(|vault| {
            vault["path"]
                .as_str()
                .expect("vault path should be a string")
                .to_owned()
        })
        .collect()
}

/// Return the canonical path string used by the server for a vault root.
fn canonical_string(path: &Path) -> String {
    fs::canonicalize(path)
        .expect("vault path should canonicalize")
        .to_string_lossy()
        .into_owned()
}

/// Snapshot relative paths, file sizes, and modification times beneath a vault root.
fn snapshot_directory(root: &Path) -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    let mut pending = vec![root.to_path_buf()];
    let mut snapshot = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).expect("vault directory should be readable");
        for entry in entries {
            let path = entry.expect("vault entry should be readable").path();
            let metadata = fs::metadata(&path).expect("vault metadata should be readable");
            if metadata.is_dir() {
                pending.push(path.clone());
            }
            snapshot.push((
                path.strip_prefix(root)
                    .expect("entry should remain under vault root")
                    .to_path_buf(),
                metadata.len(),
                metadata.modified().ok(),
            ));
        }
    }
    snapshot.sort_by(|left, right| left.0.cmp(&right.0));
    snapshot
}
