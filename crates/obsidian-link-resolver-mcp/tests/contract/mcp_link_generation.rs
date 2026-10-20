//! Contract tests for the MCP link-generation tool.

use super::mcp_client::{context_file, StdioClient};
use serde_json::{json, Value};

/// Verify that generation is discoverable with strict, explicit request fields.
#[test]
fn advertises_strict_generation_schema() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = client.request(2, "tools/list", json!({}));
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools/list should return a tool array");
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == "generate_obsidian_link")
        .expect("generation tool should be discoverable");
    let schema = &tool["inputSchema"];
    assert_eq!(schema["additionalProperties"], false);

    let properties = schema["properties"]
        .as_object()
        .expect("generation schema should define properties");
    for name in ["context_path", "target_path"] {
        assert_eq!(properties[name]["type"], "string");
        assert!(properties[name]["minLength"].as_u64().unwrap_or_default() >= 1);
    }
    let intent_schema = if let Some(reference) = properties["intent"]["$ref"].as_str() {
        let definition_name = reference.rsplit('/').next().unwrap_or_default();
        &schema["$defs"][definition_name]
    } else {
        &properties["intent"]
    };
    let intent_values: Vec<&Value> = intent_schema["oneOf"]
        .as_array()
        .expect("intent should be a strict string enum")
        .iter()
        .map(|option| &option["const"])
        .collect();
    assert!(intent_values.contains(&&json!("existing")));
    assert!(intent_values.contains(&&json!("future")));
    assert_schema_values(schema, "format", &["wikilink", "markdown"]);
    assert_schema_values(schema, "output", &["link", "embed"]);
    for name in [
        "format",
        "output",
        "same_file",
        "heading",
        "block_id",
        "label",
        "vault_root",
    ] {
        assert!(properties.contains_key(name), "missing property {name}");
    }
    assert_eq!(properties["same_file"]["type"], "boolean");

    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("generation schema should define required arguments")
        .iter()
        .map(|value| value.as_str().expect("required name should be a string"))
        .collect();
    for name in ["context_path", "target_path", "intent"] {
        assert!(required.contains(&name), "{name} should be required");
    }
    assert!(!required.contains(&"format"));
    assert!(!required.contains(&"output"));

    let defaults = client.request(
        3,
        "tools/call",
        json!({
            "name": "generate_obsidian_link",
            "arguments": {
                "context_path": context_file(),
                "target_path": "Project Plan.md",
                "intent": "existing"
            }
        }),
    );
    assert_eq!(defaults["result"]["isError"], false, "{defaults}");
    assert_eq!(
        defaults["result"]["structuredContent"]["format"],
        "wikilink"
    );
    assert_eq!(defaults["result"]["structuredContent"]["output"], "link");
    assert_eq!(
        defaults["result"]["structuredContent"]["link_text"],
        "[[Project Plan]]"
    );

    for (id, format, expected_link) in [
        (4, "wikilink", "[[Project Plan|Roadmap]]"),
        (5, "markdown", "[Roadmap](Project%20Plan.md)"),
    ] {
        let labeled = client.request(
            id,
            "tools/call",
            json!({
                "name": "generate_obsidian_link",
                "arguments": {
                    "context_path": context_file(),
                    "target_path": "Project Plan.md",
                    "intent": "existing",
                    "format": format,
                    "label": "Roadmap"
                }
            }),
        );
        assert_eq!(labeled["result"]["isError"], false, "{labeled}");
        assert_eq!(
            labeled["result"]["structuredContent"]["status"], "validated",
            "{labeled}"
        );
        assert_eq!(
            labeled["result"]["structuredContent"]["link_text"], expected_link,
            "{labeled}"
        );
    }
}

/// Verify missing, malformed, and unknown generation arguments are rejected.
#[test]
fn rejects_invalid_generation_arguments() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let invalid_arguments = [
        json!({"target_path": "Project Plan.md", "intent": "existing"}),
        json!({"context_path": context_file(), "target_path": "Project Plan.md"}),
        json!({"context_path": context_file(), "target_path": "Project Plan.md", "intent": "existing", "extra": true}),
        json!({"context_path": context_file(), "target_path": "Project Plan.md", "intent": "existing", "same_file": "true"}),
        json!({"context_path": context_file(), "target_path": "Project Plan.md", "intent": "unknown"}),
        json!({"context_path": context_file(), "target_path": "Project Plan.md", "intent": "existing", "format": "html"}),
        json!({"context_path": context_file(), "target_path": "Project Plan.md", "intent": "existing", "output": "preview"}),
        json!({"context_path": "", "target_path": "Project Plan.md", "intent": "existing"}),
        json!({"context_path": context_file(), "target_path": "Project Plan.md", "intent": "existing", "markdown_label": "Roadmap"}),
    ];

    for (index, arguments) in invalid_arguments.into_iter().enumerate() {
        let response = client.request(
            index as u64 + 2,
            "tools/call",
            json!({"name": "generate_obsidian_link", "arguments": arguments}),
        );
        assert_tool_error(&response);
    }

    let simultaneous_subtargets = client.request(
        20,
        "tools/call",
        json!({
            "name": "generate_obsidian_link",
            "arguments": {
                "context_path": context_file(),
                "target_path": "Project Plan.md",
                "intent": "existing",
                "heading": "Milestones",
                "block_id": "plan-block"
            }
        }),
    );
    assert_eq!(
        simultaneous_subtargets["result"]["structuredContent"]["status"], "rejected",
        "{simultaneous_subtargets}"
    );
}

/// Assert that a generated enum schema contains exactly the supported values.
fn assert_schema_values(root_schema: &Value, field_name: &str, expected: &[&str]) {
    let field_schema = &root_schema["properties"][field_name];
    let enum_schema = if let Some(reference) = field_schema["$ref"].as_str() {
        let definition_name = reference.rsplit('/').next().unwrap_or_default();
        &root_schema["$defs"][definition_name]
    } else {
        field_schema
    };
    let options = enum_schema["oneOf"]
        .as_array()
        .unwrap_or_else(|| panic!("enum schema should use oneOf: {enum_schema}"));
    let values: Vec<&str> = options
        .iter()
        .map(|option| {
            option["const"]
                .as_str()
                .expect("enum option should have const")
        })
        .collect();
    assert_eq!(values, expected);
}

/// Assert that invalid input returns an MCP error and no structured result.
fn assert_tool_error(response: &Value) {
    assert!(
        response.get("error").is_some() || response["result"]["isError"] == true,
        "invalid call should be an MCP error: {response}"
    );
    assert!(
        response["result"].get("structuredContent").is_none(),
        "invalid call must not contain structured content: {response}"
    );
}
