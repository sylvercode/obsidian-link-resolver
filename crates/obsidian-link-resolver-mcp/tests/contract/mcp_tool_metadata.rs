//! Contract tests for agent-facing MCP tool metadata and result guidance.

use super::mcp_client::{context_file, StdioClient};
use serde_json::{json, Value};

/// Verify that discovered metadata explains inputs, outcomes, and target detail.
#[test]
fn describes_tool_arguments_outcomes_and_emplacement_choice() {
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
        .expect("resolver tool should be discoverable");
    let description = tool["description"]
        .as_str()
        .expect("resolver tool should have a description");

    for guidance in [
        "Obsidian link",
        "link",
        "context",
        "resolved",
        "unresolved",
        "sub_target_not_found",
        "ambiguous",
        "error",
        "simple target line",
        "single point target",
        "structured emplacement",
        "precise section or shard",
    ] {
        assert!(
            description.contains(guidance),
            "tool description should include {guidance:?}: {description}"
        );
    }

    let schema = &tool["inputSchema"];
    let properties = schema["properties"]
        .as_object()
        .expect("tool schema should define properties");
    for (name, guidance) in [
        ("link", "raw Obsidian link"),
        ("context_path", "file containing"),
        ("vault_root", "allowed vault root"),
        ("with_emplacement", "structured emplacement"),
    ] {
        let field_description = properties[name]["description"].as_str().unwrap_or_default();
        assert!(
            field_description
                .to_lowercase()
                .contains(&guidance.to_lowercase()),
            "{name} schema should explain {guidance:?}: {field_description}"
        );
    }

    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("tool schema should list required arguments")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("required argument should be a string")
        })
        .collect();
    assert_eq!(required, ["link", "context_path"]);

    let response = client.call_tool(
        3,
        json!({
            "link": "[[Project Plan#Milestones]]",
            "context_path": context_file(),
            "with_emplacement": true
        }),
    );
    assert_eq!(response["result"]["isError"], false, "{response}");
    let record = &response["result"]["structuredContent"];
    assert_eq!(record["status"], "resolved");
    assert_eq!(record["target_path"], "Project Plan.md");
    assert!(record["target_range"].is_object());
    assert!(record["emplacement"].is_object());
    assert_eq!(record["is_embed"], false);

    let fields = record
        .as_object()
        .expect("structured content should be a resolver record");
    for field in ["status", "is_embed"] {
        assert!(fields.contains_key(field), "result should include {field}");
    }
    assert!(fields.keys().all(|field| {
        [
            "status",
            "target_path",
            "target_range",
            "is_embed",
            "display_text",
            "candidates",
            "reason",
            "emplacement",
        ]
        .contains(&field.as_str())
    }));
    let text = response["result"]["content"]
        .as_array()
        .expect("tool result should include content")
        .iter()
        .find_map(|item| item["text"].as_str())
        .expect("tool result should include serialized record");
    let text_record: Value = serde_json::from_str(text).expect("tool text should be JSON");
    assert_eq!(&text_record, record);
}
