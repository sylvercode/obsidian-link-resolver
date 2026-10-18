//! Determinism checks for the primary MCP result record.

use super::mcp_client::{context_file, StdioClient};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// Repeated calls against an unchanged vault return the same resolver record.
#[test]
fn repeated_calls_have_identical_primary_results() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let arguments = json!({
        "link": "[[Project Plan#Milestones]]",
        "context_path": context_file(),
        "with_emplacement": true
    });
    let first = client.call_tool(2, arguments.clone());
    let first_result = first["result"]["structuredContent"].clone();
    let first_text = result_text(&first["result"]);

    for request_id in 3..6 {
        let response = client.call_tool(request_id, arguments.clone());
        let result = &response["result"]["structuredContent"];
        assert_eq!(result, &first_result);
        assert_eq!(result_text(&response["result"]), first_text);

        let fields: BTreeSet<&str> = result
            .as_object()
            .expect("resolver result should be an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert!(fields.iter().all(|field| {
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
            .contains(field)
        }));
    }
}

/// Extract the compact JSON result from the tool's text content.
///
/// # Parameters
///
/// - `result`: MCP tool result to inspect.
///
/// # Returns
///
/// The parsed JSON record returned in text content.
fn result_text(result: &Value) -> Value {
    let text = result["content"]
        .as_array()
        .expect("tool result should include content")
        .iter()
        .find_map(|item| item["text"].as_str())
        .expect("tool result should include compact JSON text");
    serde_json::from_str(text).expect("tool text should contain valid JSON")
}
