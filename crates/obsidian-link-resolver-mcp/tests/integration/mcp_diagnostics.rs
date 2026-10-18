//! Child-process tests for optional MCP diagnostics and protocol isolation.

use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

const ALLOWED_EVENTS: &[&str] = &[
    "server_started",
    "server_stopped",
    "request_started",
    "request_completed",
    "request_failed",
    "cache_hit",
    "cache_miss",
    "cache_refresh_started",
    "cache_refreshed",
    "cache_refresh_failed",
    "watcher_unavailable",
];

const ALLOWED_FAILURE_KINDS: &[&str] = &[
    "invalid_arguments",
    "root_selection",
    "cache_refresh",
    "watch_setup",
    "protocol",
    "transport",
    "internal",
];

const ALLOWED_DIAGNOSTIC_FIELDS: &[&str] = &[
    "timestamp",
    "level",
    "target",
    "message",
    "event",
    "request_seq",
    "status",
    "failure_kind",
    "duration_ms",
];

const REDACTED_SENTINELS: &[&str] = &[
    "request-text-sentinel",
    "context-path-sentinel",
    "vault-path-sentinel",
    "note-content-sentinel",
    "serialized-arguments-sentinel",
    "raw-error-sentinel",
];

/// Verify diagnostics are silent by default and redacted when enabled without changing stdio results.
#[test]
fn diagnostics_are_opt_in_redacted_and_isolated_from_protocol_output() {
    let disabled = run_server(false);
    let enabled = run_server(true);

    assert!(disabled.status.success());
    assert!(
        disabled.stderr.is_empty(),
        "unexpected stderr: {}",
        disabled.stderr
    );
    assert!(enabled.status.success());
    assert_eq!(disabled.responses, enabled.responses);
    assert!(enabled.trailing_stdout.is_empty());

    let diagnostics = enabled
        .stderr
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("diagnostic line should be JSON"))
        .collect::<Vec<_>>();
    assert!(!diagnostics.is_empty(), "diagnostics should be emitted");

    let mut events = Vec::new();
    for record in &diagnostics {
        let fields = record
            .as_object()
            .expect("diagnostic record should be an object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert!(
            fields
                .iter()
                .all(|field| ALLOWED_DIAGNOSTIC_FIELDS.contains(field)),
            "unexpected diagnostic fields: {fields:?}"
        );
        for required in ["timestamp", "level", "target", "message", "event"] {
            assert!(
                record.get(required).is_some(),
                "missing {required}: {record}"
            );
        }
        assert_eq!(record["target"], "obsidian_link_resolver_mcp::diagnostics");

        let event = record["event"].as_str().expect("event should be a string");
        assert!(ALLOWED_EVENTS.contains(&event), "unexpected event: {event}");
        events.push(event);

        if let Some(failure_kind) = record.get("failure_kind") {
            let failure_kind = failure_kind
                .as_str()
                .expect("failure_kind should be a string");
            assert!(
                ALLOWED_FAILURE_KINDS.contains(&failure_kind),
                "unexpected failure kind: {failure_kind}"
            );
        }
        let serialized = record.to_string();
        for sentinel in REDACTED_SENTINELS {
            assert!(
                !serialized.contains(sentinel),
                "diagnostic exposed {sentinel}: {record}"
            );
        }
    }

    for expected in [
        "server_started",
        "server_stopped",
        "request_started",
        "request_completed",
        "request_failed",
    ] {
        assert!(
            events.contains(&expected),
            "missing {expected} event: {events:?}"
        );
    }
    assert!(diagnostics.iter().any(|record| {
        record["event"] == "request_failed" && record["failure_kind"] == "root_selection"
    }));
    assert!(diagnostics
        .iter()
        .any(|record| { record["event"] == "request_completed" && record["status"] == "error" }));
    assert!(diagnostics.iter().any(|record| {
        record["event"] == "request_failed" && record["failure_kind"] == "invalid_arguments"
    }));
}

#[test]
fn transport_failure_does_not_print_raw_error_when_diagnostics_are_enabled() {
    let directory = tempfile::tempdir().expect("temporary directory should be created");
    let vault = directory.path().join("vault");
    fs::create_dir_all(&vault).expect("vault should be created");

    let mut child = Command::new(env!("CARGO_BIN_EXE_obsidian-link-resolver-mcp"))
        .args([
            "--vault",
            vault.to_str().expect("vault path should be UTF-8"),
            "--diagnostics",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("MCP server should start");
    writeln!(
        child.stdin.take().expect("stdin should be piped"),
        "raw-error-sentinel"
    )
    .expect("invalid handshake data should be sent");

    let output = child
        .wait_with_output()
        .expect("server output should be collected");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(
        !output.status.success(),
        "transport failure should exit nonzero"
    );

    let diagnostics = stderr
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("diagnostic line should be JSON"))
        .collect::<Vec<_>>();
    assert!(diagnostics.iter().any(|record| {
        record["event"] == "server_stopped" && record["failure_kind"] == "transport"
    }));
    assert!(!stderr.contains("raw-error-sentinel"));
}

/// Run equivalent protocol calls and capture the server's output channels after clean shutdown.
fn run_server(diagnostics: bool) -> ProcessOutput {
    let directory = tempfile::tempdir().expect("temporary directory should be created");
    let vault = directory.path().join("vault-path-sentinel");
    fs::create_dir_all(vault.join(".obsidian")).expect("vault should be created");
    let context = vault.join("context-path-sentinel.md");
    fs::write(&context, "# Context\n").expect("context note should be written");
    fs::write(vault.join("Note.md"), "# Note\nnote-content-sentinel\n")
        .expect("target note should be written");

    let mut command = Command::new(env!("CARGO_BIN_EXE_obsidian-link-resolver-mcp"));
    command.args([
        "--vault",
        vault.to_str().expect("vault path should be UTF-8"),
    ]);
    if diagnostics {
        command.arg("--diagnostics");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("MCP server should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout should be piped"));
    let mut responses = Vec::new();

    send_message(
        &mut child,
        &mut stdin,
        &mut stdout,
        &mut responses,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "diagnostics-test", "version": "1.0"}
            }
        }),
    );
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
    )
    .expect("initialized notification should be sent");
    stdin.flush().expect("stdin should flush");

    for (id, arguments) in [
        (
            2,
            json!({
                "link": "[[Note]]",
                "context_path": context.to_string_lossy(),
                "vault_root": vault.to_string_lossy(),
                "with_emplacement": true
            }),
        ),
        (
            3,
            json!({
                "link": "[[request-text-sentinel-serialized-arguments-sentinel]]",
                "context_path": context.to_string_lossy(),
                "vault_root": vault.to_string_lossy()
            }),
        ),
        (
            4,
            json!({
                "link": "raw-error-sentinel",
                "context_path": context.to_string_lossy(),
                "vault_root": vault.to_string_lossy()
            }),
        ),
        (
            5,
            json!({
                "link": "[[Note]]",
                "context_path": context.to_string_lossy(),
                "vault_root": directory.path().join("raw-error-sentinel").to_string_lossy()
            }),
        ),
        (
            6,
            json!({
                "link": 42,
                "context_path": context.to_string_lossy(),
                "vault_root": vault.to_string_lossy(),
                "serialized-arguments-sentinel": true
            }),
        ),
    ] {
        send_message(
            &mut child,
            &mut stdin,
            &mut stdout,
            &mut responses,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {"name": "resolve_obsidian_link", "arguments": arguments}
            }),
        );
    }

    drop(stdin);
    let status = child.wait().expect("server should exit after stdin closes");
    let mut trailing_stdout = String::new();
    stdout
        .read_to_string(&mut trailing_stdout)
        .expect("remaining stdout should be readable");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr should be piped")
        .read_to_string(&mut stderr)
        .expect("stderr should be readable");

    ProcessOutput {
        status,
        responses,
        trailing_stdout,
        stderr,
    }
}

/// Write one JSON-RPC request and collect its matching response.
fn send_message(
    child: &mut std::process::Child,
    stdin: &mut std::process::ChildStdin,
    stdout: &mut BufReader<std::process::ChildStdout>,
    responses: &mut Vec<Value>,
    message: Value,
) {
    writeln!(stdin, "{message}").expect("request should be written");
    stdin.flush().expect("request should be flushed");
    let mut response = String::new();
    stdout
        .read_line(&mut response)
        .expect("response should be readable");
    if response.is_empty() {
        let status = child.try_wait().expect("child status should be readable");
        if status.is_none() {
            child.kill().expect("unresponsive child should stop");
            child.wait().expect("stopped child should be reaped");
        }
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .expect("stderr should be piped")
            .read_to_string(&mut stderr)
            .expect("stderr should be readable");
        panic!(
            "server closed stdout before responding to {message}; exit={status:?}; stderr={stderr}"
        );
    }
    responses.push(serde_json::from_str(&response).expect("response should be JSON"));
}

/// Captured child-process exit state and both output channels.
struct ProcessOutput {
    /// Process exit status.
    status: std::process::ExitStatus,
    /// MCP responses returned on stdout.
    responses: Vec<Value>,
    /// Any protocol output emitted after the expected responses.
    trailing_stdout: String,
    /// Captured diagnostic output from stderr.
    stderr: String,
}
