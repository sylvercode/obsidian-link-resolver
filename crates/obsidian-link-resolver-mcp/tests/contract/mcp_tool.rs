//! Assertions for the MCP tool name, schema, and invalid-input behavior.

use super::mcp_client::StdioClient;
use serde_json::{json, Value};

/// Verify required and optional arguments in the discovered tool schema.
#[test]
fn advertises_tool_schema_with_required_and_optional_arguments() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = client.request(2, "tools/list", json!({}));
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools/list should return a tool array");
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == "resolve_obsidian_link")
        .expect("resolver tool should remain discoverable");

    let schema = &tool["inputSchema"];
    assert_eq!(schema["additionalProperties"], false);
    let properties = schema["properties"]
        .as_object()
        .expect("tool schema should define properties");
    assert!(properties.contains_key("link"));
    assert!(properties.contains_key("context_path"));
    assert!(properties.contains_key("vault_root"));
    assert!(properties.contains_key("with_emplacement"));
    assert_eq!(properties["link"]["type"], "string");
    assert_eq!(properties["context_path"]["type"], "string");
    assert_eq!(properties["vault_root"]["type"], "string");
    assert_eq!(properties["with_emplacement"]["type"], "boolean");
    assert_eq!(properties["link"]["minLength"], 1);
    assert_eq!(properties["context_path"]["minLength"], 1);
    assert_eq!(properties["vault_root"]["minLength"], 1);

    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("tool schema should define required arguments")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("required argument should be a string")
        })
        .collect();
    assert_eq!(required, ["link", "context_path"]);
}

/// Verify malformed required and optional arguments produce MCP errors, not results.
#[test]
fn rejects_missing_or_wrong_type_arguments_without_resolver_result() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let invalid_arguments = [
        json!({"context_path": super::mcp_client::context_file()}),
        json!({"link": "[[Project Plan]]"}),
        json!({"link": 1, "context_path": super::mcp_client::context_file()}),
        json!({"link": "[[Project Plan]]", "context_path": 1}),
        json!({"link": "[[Project Plan]]", "context_path": super::mcp_client::context_file(), "vault_root": 1}),
        json!({"link": "[[Project Plan]]", "context_path": super::mcp_client::context_file(), "vault_root": null}),
        json!({"link": "[[Project Plan]]", "context_path": super::mcp_client::context_file(), "vault_root": ""}),
        json!({"link": "[[Project Plan]]", "context_path": super::mcp_client::context_file(), "with_emplacement": "true"}),
        json!({"link": "[[Project Plan]]", "context_path": super::mcp_client::context_file(), "with_emplacement": null}),
        json!({"link": "", "context_path": super::mcp_client::context_file()}),
        json!({"link": "[[Project Plan]]", "context_path": ""}),
        json!({"link": "[[Project Plan]]", "context_path": super::mcp_client::context_file(), "extra": true}),
    ];

    for (index, arguments) in invalid_arguments.into_iter().enumerate() {
        let response = client.call_tool(index as u64 + 2, arguments);
        assert_tool_error(&response);
    }
}

/// Ensure a request cannot widen the launch-time vault boundary to an ancestor.
#[test]
fn rejects_unconfigured_ancestor_vault_root() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = client.call_tool(
        2,
        json!({
            "link": "[[Project Plan]]",
            "context_path": super::mcp_client::context_file(),
            "vault_root": "/"
        }),
    );
    assert_tool_error(&response);
}

/// Assert that a tool response is an MCP-level error without resolver content.
///
/// # Parameters
///
/// - `response`: JSON-RPC response to inspect.
fn assert_tool_error(response: &Value) {
    let is_protocol_error = response.get("error").is_some();
    let is_tool_error = response["result"]["isError"] == true;
    assert!(
        is_protocol_error || is_tool_error,
        "invalid call should be an MCP error: {response}"
    );
    assert!(
        response["result"].get("structuredContent").is_none(),
        "invalid call must not contain a resolver record: {response}"
    );
    assert_ne!(response["result"]["structuredContent"]["status"], "error");
}
