//! End-to-end path-boundary tests for MCP link generation.

use super::mcp_client::StdioClient;
use serde_json::{json, Value};
use std::fs;

/// Verify that a planned source beneath an allowed root can be used safely.
#[test]
fn accepts_a_planned_source_path_inside_the_configured_vault() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let root = temporary_directory.path().join("vault");
    fs::create_dir_all(root.join(".obsidian")).expect("vault metadata should be created");
    let root_string = root.to_string_lossy().into_owned();
    let planned_source = root.join("drafts/Planned Note.md");
    let mut client = StdioClient::with_vaults(vec![root_string]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let response = call_generation(
        &mut client,
        2,
        json!({
            "context_path": planned_source.to_string_lossy(),
            "target_path": "future/Planned Target.md",
            "intent": "future"
        }),
    );

    let result = &response["result"]["structuredContent"];
    assert_eq!(result["status"], "unverified", "{response}");
    assert_eq!(result["path_allowed"], true, "{response}");
    assert_eq!(result["destination_validation"], "unverified", "{response}");
    assert!(
        !planned_source.exists(),
        "generation must not create its source"
    );
}

/// Verify lexical traversal in source or target paths is never accepted.
#[test]
fn rejects_source_and_target_traversal_outside_the_configured_vault() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let root = temporary_directory.path().join("vault");
    fs::create_dir_all(root.join(".obsidian")).expect("vault metadata should be created");
    fs::write(root.join("Context.md"), "# Context\n").expect("context should be written");
    let root_string = root.to_string_lossy().into_owned();
    let mut client = StdioClient::with_vaults(vec![root_string.clone()]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    for (id, context_path, target_path) in [
        (2, format!("{root_string}/../outside.md"), "Future.md"),
        (3, format!("{root_string}/Context.md"), "../outside.md"),
        (
            4,
            format!("{root_string}/missing/../../planned.md"),
            "Future.md",
        ),
    ] {
        let response = call_generation(
            &mut client,
            id,
            json!({
                "context_path": context_path,
                "target_path": target_path,
                "intent": "future"
            }),
        );
        assert_eq!(
            response["result"]["structuredContent"]["status"], "rejected",
            "{response}"
        );
        assert_eq!(
            response["result"]["structuredContent"]["path_allowed"], false,
            "{response}"
        );
    }
}

/// Verify existing source and target symlinks cannot escape an allowed root.
#[cfg(unix)]
#[test]
fn rejects_source_and_target_symlinks_that_escape_the_configured_vault() {
    use std::os::unix::fs::symlink;

    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let root = temporary_directory.path().join("vault");
    let outside = temporary_directory.path().join("outside");
    fs::create_dir_all(root.join(".obsidian")).expect("vault metadata should be created");
    fs::create_dir_all(&outside).expect("outside directory should be created");
    fs::write(outside.join("Context.md"), "# Outside\n").expect("outside source should exist");
    fs::write(outside.join("Target.md"), "# Outside target\n")
        .expect("outside target should exist");
    fs::write(
        temporary_directory.path().join("Context.md"),
        "# Outside context\n",
    )
    .expect("parent context should exist");
    fs::write(root.join("Context.md"), "# In-vault context\n")
        .expect("in-vault context should exist");
    fs::write(root.join("Target.md"), "# In-vault target\n").expect("in-vault target should exist");
    symlink(outside.join("Context.md"), root.join("Escaped Context.md"))
        .expect("source symlink should be created");
    symlink(outside.join("Target.md"), root.join("Escaped Target.md"))
        .expect("target symlink should be created");
    symlink(&outside, root.join("Escaped Directory")).expect("directory symlink should be created");
    let root_string = root.to_string_lossy().into_owned();
    let mut client = StdioClient::with_vaults(vec![root_string.clone()]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    for (id, context_path, target_path) in [
        (
            2,
            root.join("Escaped Context.md")
                .to_string_lossy()
                .into_owned(),
            "Target.md",
        ),
        (
            3,
            root.join("Context.md").to_string_lossy().into_owned(),
            "Escaped Target.md",
        ),
        (
            4,
            root.join("Escaped Directory/../Context.md")
                .to_string_lossy()
                .into_owned(),
            "Target.md",
        ),
        (
            5,
            root.join("Context.md").to_string_lossy().into_owned(),
            "Escaped Directory/../Target.md",
        ),
    ] {
        let response = call_generation(
            &mut client,
            id,
            json!({
                "context_path": context_path,
                "target_path": target_path,
                "intent": "existing"
            }),
        );
        assert_eq!(
            response["result"]["structuredContent"]["status"], "rejected",
            "{response}"
        );
        assert_eq!(
            response["result"]["structuredContent"]["path_allowed"], false,
            "{response}"
        );
    }
}

/// Verify existing note and subtarget requests validate only exact resolver matches.
#[test]
fn validates_existing_notes_headings_and_blocks_in_both_formats() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let root = temporary_directory.path().join("vault");
    fs::create_dir_all(root.join(".obsidian")).expect("vault metadata should be created");
    fs::create_dir_all(root.join("notes")).expect("notes directory should be created");
    fs::write(root.join("Context.md"), "# Context\n").expect("context should be written");
    fs::write(
        root.join("notes/Target.md"),
        "# Overview\n\nIntro.\n\n## Plan\n\nPlan details.\n\n^plan-block\n",
    )
    .expect("target note should be written");
    fs::write(root.join("notes/Exact Case.md"), "# Case\n")
        .expect("exact-case target should be written");
    fs::write(root.join("notes/Duplicate.md"), "# Upper collision\n")
        .expect("upper-case colliding target should be written");
    fs::write(root.join("notes/duplicate.md"), "# Lower collision\n")
        .expect("case-colliding target should be written");
    let context_path = root.join("Context.md").to_string_lossy().into_owned();
    let mut client = StdioClient::with_vaults(vec![root.to_string_lossy().into_owned()]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let mut request_id = 2;
    for (format, subtarget, expected_link) in [
        ("wikilink", json!({}), "[[notes/Target]]"),
        ("markdown", json!({}), "[Target](notes/Target.md)"),
        (
            "wikilink",
            json!({"heading": "Plan"}),
            "[[notes/Target#Plan]]",
        ),
        (
            "markdown",
            json!({"heading": "Plan"}),
            "[Target#Plan](notes/Target.md#Plan)",
        ),
        (
            "wikilink",
            json!({"block_id": "plan-block"}),
            "[[notes/Target#^plan-block]]",
        ),
        (
            "markdown",
            json!({"block_id": "plan-block"}),
            "[Target#^plan-block](notes/Target.md#^plan-block)",
        ),
    ] {
        let mut arguments = json!({
            "context_path": context_path,
            "target_path": "notes/Target.md",
            "intent": "existing",
            "format": format
        });
        for (key, value) in subtarget
            .as_object()
            .expect("subtarget should be an object")
        {
            arguments[key] = value.clone();
        }
        let response = call_generation(&mut client, request_id, arguments);
        request_id += 1;
        let result = &response["result"]["structuredContent"];
        assert_eq!(result["status"], "validated", "{response}");
        assert_eq!(result["destination_validation"], "validated", "{response}");
        assert_eq!(result["target_path"], "notes/Target.md", "{response}");
        assert_eq!(result["link_text"], expected_link, "{response}");
    }

    for arguments in [
        json!({
            "context_path": context_path,
            "target_path": "notes/Target.md",
            "intent": "existing",
            "heading": "Missing"
        }),
        json!({
            "context_path": context_path,
            "target_path": "notes/Target.md",
            "intent": "existing",
            "block_id": "missing-block"
        }),
    ] {
        let response = call_generation(&mut client, request_id, arguments);
        request_id += 1;
        assert_eq!(
            response["result"]["structuredContent"]["status"], "rejected",
            "{response}"
        );
        assert_ne!(
            response["result"]["structuredContent"]["destination_validation"], "validated",
            "{response}"
        );
    }

    let ambiguous = call_generation(
        &mut client,
        request_id,
        json!({
            "context_path": context_path,
            "target_path": "notes/Duplicate.md",
            "intent": "existing"
        }),
    );
    assert_eq!(
        ambiguous["result"]["structuredContent"]["status"], "rejected",
        "case-insensitive resolver collision must not be validated: {ambiguous}"
    );
    let mismatched = call_generation(
        &mut client,
        request_id + 1,
        json!({
            "context_path": context_path,
            "target_path": "notes/exact case.md",
            "intent": "existing"
        }),
    );
    assert_eq!(
        mismatched["result"]["structuredContent"]["status"], "rejected",
        "case-insensitive resolution must not mask an exact-path mismatch: {mismatched}"
    );
    assert_eq!(
        mismatched["result"]["structuredContent"]["destination_validation"], "failed",
        "{mismatched}"
    );

    let mismatched_same_file = call_generation(
        &mut client,
        request_id + 2,
        json!({
            "context_path": context_path,
            "target_path": "notes/Target.md",
            "intent": "existing",
            "same_file": true,
            "heading": "Plan"
        }),
    );
    assert_eq!(
        mismatched_same_file["result"]["structuredContent"]["status"], "rejected",
        "same-file shorthand must name the contextual note: {mismatched_same_file}"
    );
}

/// Verify future destinations remain unverified and generation does not mutate the vault.
#[test]
fn keeps_future_targets_unverified_and_does_not_write_vault_files() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let root = temporary_directory.path().join("vault");
    fs::create_dir_all(root.join(".obsidian")).expect("vault metadata should be created");
    fs::create_dir_all(root.join("notes")).expect("notes directory should be created");
    fs::write(root.join("Context.md"), "# Context\n").expect("context should be written");
    fs::write(root.join("notes/Existing.md"), "# Ready\n\nBody.\n")
        .expect("existing note should be written");
    let context_path = root.join("Context.md").to_string_lossy().into_owned();
    let mut client = StdioClient::with_vaults(vec![root.to_string_lossy().into_owned()]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();
    let before = list_files(&root);

    for (id, target_path, selector) in [
        (2, "notes/Future Note.md", json!({})),
        (3, "assets/Future Image.png", json!({})),
        (
            4,
            "notes/Existing.md",
            json!({"heading": "Planned Heading"}),
        ),
    ] {
        let mut arguments = json!({
            "context_path": context_path,
            "target_path": target_path,
            "intent": "future"
        });
        for (key, value) in selector.as_object().expect("selector should be an object") {
            arguments[key] = value.clone();
        }
        let response = call_generation(&mut client, id, arguments);
        let result = &response["result"]["structuredContent"];
        assert_eq!(result["status"], "unverified", "{response}");
        assert_eq!(result["destination_validation"], "unverified", "{response}");
        assert_eq!(result["syntax_valid"], true, "{response}");
        assert_eq!(result["path_allowed"], true, "{response}");
        assert!(result["link_text"].is_string(), "{response}");
    }

    let unsupported = call_generation(
        &mut client,
        6,
        json!({
            "context_path": context_path,
            "target_path": "assets/Future Document.txt",
            "intent": "future"
        }),
    );
    assert_eq!(
        unsupported["result"]["structuredContent"]["status"], "rejected",
        "unsupported attachment types must not be emitted: {unsupported}"
    );
    assert_eq!(
        unsupported["result"]["structuredContent"]["path_allowed"],
        true
    );

    let missing_existing = call_generation(
        &mut client,
        5,
        json!({
            "context_path": context_path,
            "target_path": "notes/Missing.md",
            "intent": "existing"
        }),
    );
    assert_eq!(
        missing_existing["result"]["structuredContent"]["status"], "rejected",
        "{missing_existing}"
    );
    assert_eq!(
        missing_existing["result"]["structuredContent"]["path_allowed"], true,
        "missing destinations inside the vault still pass path checks: {missing_existing}"
    );
    assert_eq!(
        missing_existing["result"]["structuredContent"]["destination_validation"], "failed",
        "{missing_existing}"
    );
    assert_eq!(
        list_files(&root),
        before,
        "generation must not mutate the vault"
    );
}

/// Verify attachments, embeds, same-file shorthand, and display labels round-trip exactly.
#[test]
fn generates_attachments_embeds_same_file_links_and_display_labels() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be created");
    let root = temporary_directory.path().join("vault");
    fs::create_dir_all(root.join(".obsidian")).expect("vault metadata should be created");
    fs::create_dir_all(root.join("assets")).expect("assets directory should be created");
    fs::create_dir_all(root.join("notes")).expect("notes directory should be created");
    fs::write(
        root.join("Context.md"),
        "# Context Heading\n\nContext body.\n\n^context-block\n",
    )
    .expect("context should be written");
    fs::write(
        root.join("notes/Target.md"),
        "# Plan\n\n## Plan Section\n\nPlan body.\n",
    )
    .expect("target note should be written");
    fs::write(root.join("assets/diagram.png"), "attachment").expect("attachment should be written");
    fs::write(root.join("assets/cover image.png"), "attachment")
        .expect("spaced attachment should be written");
    let context_path = root.join("Context.md").to_string_lossy().into_owned();
    let mut client = StdioClient::with_vaults(vec![root.to_string_lossy().into_owned()]);
    assert!(client.initialize().get("result").is_some());
    client.notify_initialized();

    let cases = [
        (
            json!({
                "context_path": context_path,
                "target_path": "assets/diagram.png",
                "intent": "existing"
            }),
            "[[assets/diagram.png]]",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "assets/diagram.png",
                "intent": "existing",
                "output": "embed"
            }),
            "![[assets/diagram.png]]",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "notes/Target.md",
                "intent": "existing",
                "format": "wikilink",
                "label": "Roadmap"
            }),
            "[[notes/Target|Roadmap]]",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "assets/diagram.png",
                "intent": "existing",
                "format": "wikilink",
                "output": "embed",
                "label": "Diagram"
            }),
            "![[assets/diagram.png|Diagram]]",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "assets/diagram.png",
                "intent": "existing",
                "format": "markdown",
                "output": "embed",
                "label": "Diagram"
            }),
            "![Diagram](assets/diagram.png)",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "assets/cover image.png",
                "intent": "existing",
                "format": "markdown"
            }),
            "[cover image.png](assets/cover%20image.png)",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "notes/Target.md",
                "intent": "existing",
                "format": "markdown",
                "heading": "Plan Section"
            }),
            "[Target#Plan Section](notes/Target.md#Plan%20Section)",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "notes/Target.md",
                "intent": "existing",
                "format": "markdown",
                "heading": "Plan",
                "label": "Roadmap"
            }),
            "[Roadmap](notes/Target.md#Plan)",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "Context.md",
                "intent": "existing",
                "same_file": true,
                "heading": "Context Heading"
            }),
            "[[#Context Heading]]",
        ),
        (
            json!({
                "context_path": context_path,
                "target_path": "Context.md",
                "intent": "existing",
                "same_file": true,
                "format": "markdown",
                "block_id": "context-block"
            }),
            "[Context#^context-block](#^context-block)",
        ),
    ];
    for (index, (arguments, expected_link)) in cases.into_iter().enumerate() {
        let response = call_generation(&mut client, index as u64 + 2, arguments);
        assert_eq!(
            response["result"]["structuredContent"]["status"], "validated",
            "{response}"
        );
        assert_eq!(
            response["result"]["structuredContent"]["link_text"], expected_link,
            "{response}"
        );
    }

    let invalid = call_generation(
        &mut client,
        8,
        json!({
            "context_path": context_path,
            "target_path": "notes/Target.md",
            "intent": "existing",
            "heading": "Plan",
            "block_id": "context-block"
        }),
    );
    assert_eq!(invalid["result"]["structuredContent"]["status"], "rejected");
    assert_eq!(
        invalid["result"]["structuredContent"]["syntax_valid"],
        false
    );
}

/// Call the generation tool with one JSON argument object.
fn call_generation(client: &mut StdioClient, id: u64, arguments: Value) -> Value {
    client.request(
        id,
        "tools/call",
        json!({
            "name": "generate_obsidian_link",
            "arguments": arguments
        }),
    )
}

/// Return a sorted list of regular files beneath a vault root.
fn list_files(root: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("vault directory should be readable") {
            let path = entry.expect("directory entry should be readable").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                files.push(
                    path.strip_prefix(root)
                        .expect("entry should be under vault root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    files.sort();
    files
}
