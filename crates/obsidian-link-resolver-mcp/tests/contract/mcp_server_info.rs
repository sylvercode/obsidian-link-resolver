//! Contract tests for the MCP server information tool and handshake.

use super::mcp_client::{fixture_root, StdioClient};
use serde_json::{json, Value};

/// Verify discovery, the empty input schema, and tolerance of supplied arguments.
#[test]
fn advertises_and_accepts_argument_free_server_info() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = client.request(2, "tools/list", json!({}));
    let tool = response["result"]["tools"]
        .as_array()
        .expect("tools/list should return a tool array")
        .iter()
        .find(|tool| tool["name"] == "get_server_info")
        .expect("server info tool should be discoverable");
    assert_eq!(
        tool["inputSchema"],
        json!({"type": "object", "properties": {}})
    );
    let description = tool["description"]
        .as_str()
        .expect("server info tool should have a description")
        .to_lowercase();
    for phrase in ["version", "configured at startup", "no arguments"] {
        assert!(
            description.contains(phrase),
            "missing {phrase:?}: {description}"
        );
    }

    let calls = [
        (3, json!({"name": "get_server_info"})),
        (4, json!({"name": "get_server_info", "arguments": {}})),
        (
            5,
            json!({
                "name": "get_server_info",
                "arguments": {"ignored": true, "extra": 1}
            }),
        ),
    ];
    let mut structured_contents = Vec::new();
    let mut serialized_text = Vec::new();
    for (id, params) in calls {
        let response = client.request(id, "tools/call", params);
        assert_eq!(response["result"]["isError"], false, "{response}");
        let structured = response["result"]["structuredContent"].clone();
        let vaults = structured["vaults"]
            .as_array()
            .expect("vaults should be an array");
        assert!(!vaults.is_empty());
        for vault in vaults {
            let fields = vault.as_object().expect("vault should be an object");
            assert_eq!(fields.len(), 1);
            assert!(fields["path"].as_str().is_some());
        }
        let content = response["result"]["content"]
            .as_array()
            .expect("tool result should contain text content");
        let text = content[0]["text"]
            .as_str()
            .expect("first content block should contain text");
        let parsed_text: Value = serde_json::from_str(text).expect("text should be JSON");
        assert_eq!(parsed_text, structured);
        structured_contents.push(serde_json::to_string(&structured).unwrap());
        serialized_text.push(text.to_owned());
    }
    assert!(structured_contents
        .windows(2)
        .all(|pair| pair[0] == pair[1]));
    assert!(serialized_text.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(
        structured_contents[0],
        serde_json::to_string(&json!({
            "version": env!("CARGO_PKG_VERSION"),
            "vaults": [{"path": std::fs::canonicalize(fixture_root()).unwrap().to_string_lossy()}]
        }))
        .unwrap()
    );
}

/// Verify the reported build version follows the crate's semantic version.
#[test]
fn reports_the_crate_version_deterministically() {
    let mut client = StdioClient::new();
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let first = server_info(&mut client, 2);
    let second = server_info(&mut client, 3);
    assert_eq!(first["version"], env!("CARGO_PKG_VERSION"));
    assert_semver(
        first["version"]
            .as_str()
            .expect("version should be a string"),
    );
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

/// Verify initialize and the info tool report one consistent server identity.
#[test]
fn reports_server_identity_in_the_handshake() {
    let mut client = StdioClient::new();
    let initialized = client.initialize();
    assert_eq!(
        initialized["result"]["serverInfo"]["name"],
        "obsidian-link-resolver-mcp"
    );
    assert_eq!(
        initialized["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        initialized["result"]["instructions"],
        "Resolve Obsidian links."
    );
    client.notify_initialized();
    let info = server_info(&mut client, 2);
    assert_eq!(
        initialized["result"]["serverInfo"]["version"],
        info["version"]
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

/// Assert that a version string has three numeric core components and valid suffix positions.
fn assert_semver(version: &str) {
    let (without_build, build) = match version.split_once('+') {
        Some((core, suffix)) => {
            assert!(!suffix.is_empty());
            (core, Some(suffix))
        }
        None => (version, None),
    };
    let (numeric, prerelease) = match without_build.split_once('-') {
        Some((core, suffix)) => {
            assert!(!suffix.is_empty());
            (core, Some(suffix))
        }
        None => (without_build, None),
    };
    assert!(build.is_none_or(|suffix| !suffix.is_empty()));
    assert!(prerelease.is_none_or(|suffix| !suffix.is_empty()));
    let components = numeric.split('.').collect::<Vec<_>>();
    assert_eq!(components.len(), 3);
    assert!(components
        .iter()
        .all(|component| component.parse::<u64>().is_ok()));
}
