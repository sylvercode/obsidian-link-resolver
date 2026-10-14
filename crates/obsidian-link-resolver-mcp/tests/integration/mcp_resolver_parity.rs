//! End-to-end comparisons between MCP results and the resolver library.

use super::mcp_client::StdioClient;
use obsidian_link_resolver::resolve;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compare MCP and direct resolver records for every outcome and representative link form.
#[test]
fn preserves_resolver_records_for_all_outcomes_and_link_forms() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let working_directory = std::env::current_dir().expect("test working directory should exist");
    let root = working_directory.join("target").join(format!(
        "obsidian-link-resolver-mcp-parity-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(root.join(".obsidian")).expect("vault metadata should be created");
    write_file(
        &root.join("notes/Context.md"),
        "# Context Heading\n\nContext body.\n",
    );
    write_file(
        &root.join("Project Plan.md"),
        "# Milestones\n\nMilestone body.\n\n^project-block\n",
    );
    write_file(
        &root.join("folder/sub/Note.md"),
        "# Qualified\n\nQualified body.\n",
    );
    write_file(&root.join("left/Duplicate.md"), "# Left\n");
    write_file(&root.join("right/Duplicate.md"), "# Right\n");
    write_file(&root.join("assets/diagram.png"), "fixture attachment");

    let context_absolute = root.join("notes/Context.md");
    let context_relative = root
        .strip_prefix(&working_directory)
        .expect("fixture vault should be inside the working directory")
        .join("notes/Context.md")
        .to_string_lossy()
        .into_owned();
    let root_string = root.to_string_lossy().into_owned();
    let cases = [
        (
            "[[Project Plan#Milestones]]",
            context_absolute.to_string_lossy().into_owned(),
            true,
        ),
        ("[[Project Plan#Milestones]]", context_relative, true),
        (
            "[[Missing Note]]",
            context_absolute.to_string_lossy().into_owned(),
            false,
        ),
        (
            "[[Project Plan#Missing]]",
            context_absolute.to_string_lossy().into_owned(),
            false,
        ),
        (
            "[[Duplicate]]",
            context_absolute.to_string_lossy().into_owned(),
            false,
        ),
        (
            "not a link",
            context_absolute.to_string_lossy().into_owned(),
            false,
        ),
        (
            "[[folder/sub/Note]]",
            context_absolute.to_string_lossy().into_owned(),
            false,
        ),
        (
            "[[#Context Heading]]",
            context_absolute.to_string_lossy().into_owned(),
            true,
        ),
        (
            "[[Project Plan#^project-block]]",
            context_absolute.to_string_lossy().into_owned(),
            true,
        ),
        (
            "[[diagram.png]]",
            context_absolute.to_string_lossy().into_owned(),
            true,
        ),
    ];

    let mut client = StdioClient::with_vaults(vec![root_string.clone()]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let mut observed_statuses = BTreeSet::new();
    for (index, (link, context_path, with_emplacement)) in cases.iter().enumerate() {
        let arguments = json!({
            "link": link,
            "context_path": context_path,
            "vault_root": root_string,
            "with_emplacement": with_emplacement
        });
        let response = client.call_tool(index as u64 + 2, arguments);
        assert_eq!(response["result"]["isError"], false, "{response}");
        let structured = &response["result"]["structuredContent"];
        let direct = resolve(link, context_path, Some(&root_string), *with_emplacement);
        let direct = serde_json::to_value(direct).expect("resolver result should serialize");
        assert_eq!(structured, &direct, "MCP parity failed for {link}");

        let text = response["result"]["content"]
            .as_array()
            .expect("tool result should include content")
            .iter()
            .find_map(|item| item["text"].as_str())
            .expect("tool result should include serialized result text");
        let text_result: Value = serde_json::from_str(text).expect("tool text should be JSON");
        assert_eq!(
            &text_result, structured,
            "MCP text parity failed for {link}"
        );
        observed_statuses.insert(
            structured["status"]
                .as_str()
                .expect("resolver status should be a string")
                .to_owned(),
        );
    }

    assert_eq!(
        observed_statuses,
        [
            "resolved",
            "unresolved",
            "sub_target_not_found",
            "ambiguous",
            "error"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );

    drop(client);
    fs::remove_dir_all(root).expect("temporary vault should be removed");
}

/// Write a fixture file and create its parent directories as needed.
///
/// # Parameters
///
/// - `path`: File location inside the temporary vault.
/// - `contents`: File contents to write.
fn write_file(path: &std::path::Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture file has a parent"))
        .expect("fixture directories should be created");
    fs::write(path, contents).expect("fixture file should be written");
}
