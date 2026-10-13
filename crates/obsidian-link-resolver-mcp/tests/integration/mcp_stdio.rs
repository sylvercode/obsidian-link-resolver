//! End-to-end MCP lifecycle and resolver calls over child-process stdio.

use super::mcp_client::{context_file, fixture_root, StdioClient};
use serde_json::{json, Value};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// Initialize a server, discover its tool, and resolve a known fixture link.
#[test]
fn initializes_lists_tool_and_resolves_fixture_link() {
    let mut client = StdioClient::new();
    let initialized = client.initialize();
    assert!(initialized.get("result").is_some());
    client.notify_initialized();

    let listed = client.request(2, "tools/list", json!({}));
    assert_eq!(
        listed["result"]["tools"][0]["name"],
        "resolve_obsidian_link"
    );

    let response = client.call_tool(
        3,
        json!({
            "link": "[[Project Plan#Milestones]]",
            "context_path": context_file()
        }),
    );
    let result = &response["result"];
    assert_eq!(result["isError"], false);
    assert_eq!(result["structuredContent"]["status"], "resolved");
    assert_eq!(
        result["structuredContent"]["target_path"],
        "Project Plan.md"
    );
    assert_eq!(
        result["structuredContent"]["target_range"],
        json!({"begin": 5, "end": 11})
    );

    let text = result["content"]
        .as_array()
        .expect("tool result should include text content")
        .iter()
        .find_map(|item| item["text"].as_str())
        .expect("tool result should include serialized JSON text");
    let text_result: Value = serde_json::from_str(text).expect("tool text should be JSON");
    assert_eq!(text_result, result["structuredContent"]);
}

/// Verify malformed tool arguments remain MCP errors over stdio.
#[test]
fn returns_mcp_errors_for_missing_and_wrong_type_arguments() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let invalid_arguments = [
        json!({"context_path": context_file()}),
        json!({"link": "[[Project Plan]]", "context_path": 7}),
        json!({"link": "[[Project Plan]]", "context_path": context_file(), "vault_root": false}),
        json!({"link": "[[Project Plan]]", "context_path": context_file(), "with_emplacement": "yes"}),
    ];

    for (index, arguments) in invalid_arguments.into_iter().enumerate() {
        let response = client.call_tool(index as u64 + 2, arguments);
        let is_protocol_error = response.get("error").is_some();
        let is_tool_error = response["result"]["isError"] == true;
        assert!(
            is_protocol_error || is_tool_error,
            "invalid call should be an MCP error: {response}"
        );
        assert!(response["result"].get("structuredContent").is_none());
        assert_ne!(response["result"]["structuredContent"]["status"], "error");
    }
}

/// Verify repeated launch-time roots permit resolution in either configured vault.
#[test]
fn resolves_from_each_configured_vault() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let second_root = std::env::temp_dir().join(format!(
        "obsidian-link-resolver-mcp-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(second_root.join(".obsidian")).expect("second vault should be created");
    let context = second_root.join("Context.md");
    fs::write(&context, "# Context\n").expect("second vault context should be written");
    fs::write(second_root.join("Project Plan.md"), "# Plan\n")
        .expect("second vault target should be written");

    let mut client = StdioClient::with_vaults(vec![
        fixture_root(),
        second_root.to_string_lossy().into_owned(),
    ]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = client.call_tool(
        2,
        json!({
            "link": "[[Project Plan]]",
            "context_path": context.to_string_lossy(),
            "vault_root": second_root.to_string_lossy()
        }),
    );
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(
        response["result"]["structuredContent"]["status"],
        "resolved"
    );
    assert_eq!(
        response["result"]["structuredContent"]["target_path"],
        "Project Plan.md"
    );

    drop(client);
    fs::remove_dir_all(second_root).expect("temporary vault should be removed");
}
