# Quickstart & Validation: MCP Wrapper

This guide validates the MCP server end-to-end. It references the [MCP contract](contracts/mcp.md), [data model](data-model.md), and the existing [resolver result schema](../001-link-resolver/contracts/result.schema.json).

## Prerequisites

- Rust toolchain 1.98.1+ and this repository checkout.
- A fixture or local Obsidian vault containing a `.obsidian/` directory and markdown notes.
- For interactive validation, an MCP client that supports a local stdio server.

The devcontainer and CI/release toolchains must include any dependencies added by the implementation.

## Build and Run

```bash
cargo build --release --bin obsidian-link-resolver-mcp
target/release/obsidian-link-resolver-mcp --help
```

Register the installed executable in the client's standard local stdio-server configuration. The command is `obsidian-link-resolver-mcp`; optionally pass `--vault /absolute/path/to/vault` as a default. The process uses stdin/stdout for MCP only. Do not wrap or redirect its stdout. A per-call `vault_root` overrides the launch default.

## Tool Scenarios

Use `tests/fixtures/vault/` for repeatable cases, with `context_path` set to an existing note in that vault.

| Scenario | Request | Expected result |
|---|---|---|
| Discover tool | MCP `tools/list` | One `resolve_obsidian_link` tool with required link/context fields and emplacement guidance. |
| Resolve heading | `link: "[[Project Plan#Milestones]]"` | Same `target_path`, range, and `resolved` status as `obsidian-link-resolver` for identical inputs. |
| Same-file link | `link: "[[#Overview]]"` | Resolves relative to `context_path`. |
| Emplacement | Same note link with `with_emplacement: true` | Existing heading stack, section range, and structured block fields where applicable. |
| Unresolved/ambiguous | Missing link or duplicate note name | Normal tool result with the matching status and existing reason/candidate fields, not a protocol error. |
| Attachment | Link to a fixture attachment | Resolves according to existing attachment semantics without emplacement. |
| Cache reuse | Call repeatedly against one vault | One index is reused; no full directory enumeration per call. |
| Cache refresh | Add, remove, or rename a note | A watcher invalidates before the next resolution; with the watcher event suppressed, a deterministic timer test verifies the background full scan refreshes the index within 60 seconds even when no requests arrive during the interval. |
| Multiple vaults | Resolve with two canonical roots in one process | Each root uses its own cache entry. |

## Automated Validation

Run the focused MCP tests first during implementation, then the repository gates:

```bash
cargo test mcp
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

The focused suite verifies tool discovery/input schema, stdio initialize/list/call, structured and text result parity, all five resolver statuses, cache reuse, event invalidation, request-independent periodic refresh, refresh failure behavior, and separation between vault roots. Filesystem-event correctness tests should inject invalidation deterministically; the periodic scheduler should use a controllable clock so the 60-second bound can be tested without a wall-clock delay. The cross-platform watcher smoke test checks registration without depending on event timing.

## Release Validation

The tag release workflow must run the full tests and existing warm-run benchmark gate before building/staging the MCP binary for Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64. Verify the release asset name matches the explicit MCP installer component and that the default installer behavior still installs only the existing CLI. Windows ARM64 is not in the current release matrix.
