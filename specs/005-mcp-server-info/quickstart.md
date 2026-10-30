# Quickstart: MCP Server Info Tool

This guide describes the focused validation to run after implementation. The tool is read-only and never creates, modifies, or indexes vault content.

## Prerequisites

- Rust toolchain used by the workspace (see `.github/workflows/ci.yml`).
- One or more local directories to use as vaults (the repository fixtures or `tempfile` directories).
- The MCP server started with each vault explicitly allowed using `--vault <DIR>`.

## Contract and Integration Tests

Run the MCP crate tests:

```sh
cargo test -p obsidian-link-resolver-mcp
```

Expected coverage (see [contracts/mcp-server-info.md](contracts/mcp-server-info.md) and [data-model.md](data-model.md)):

- `tests/contract/mcp_server_info.rs`: `get_server_info` is listed with the empty-object schema and a description that mentions the version, startup-configured vaults, and that no arguments are needed; `initialize` reports `serverInfo.name == "obsidian-link-resolver-mcp"` and a version equal to the tool's `version` and to `CARGO_PKG_VERSION`; calls with omitted, empty, and extra arguments all succeed with identical structured content; the text content block equals the structured content.
- `tests/integration/mcp_server_info.rs`: one, two, and three vaults appear in startup order with `fs::canonicalize` paths; a vault passed twice (and via a symlink alias where symlinks are available) appears once; a directory snapshot of each vault is unchanged after the call; the tool answers before any `resolve_obsidian_link` call.

Run the warm benchmark to guard the 50 ms p50 budget and the first-call measurement:

```sh
cargo bench -p obsidian-link-resolver-mcp --bench mcp_warm
```

The printed JSON report must include `server_info_first_call_ms` and `server_info_p50_ms`, and the benchmark fails if `server_info_p50_ms` exceeds 50 ms. Existing `p50_ms` and `generation_p50_ms` gates remain at 100 ms.

Confirm the existing suites and lints still pass unchanged:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## MCP Smoke Call

Start the server:

```sh
obsidian-link-resolver-mcp --vault /path/to/vault-a --vault /path/to/vault-b
```

Send `initialize`, `notifications/initialized`, then:

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_server_info","arguments":{}}}
```

Expected: `result.isError` is `false` and `result.structuredContent` is

```json
{"version":"<crate version>","vaults":[{"path":"<canonical vault-a>"},{"path":"<canonical vault-b>"}]}
```

Re-sending the request (with or without `"arguments":{"ignored":true}`) returns identical `structuredContent`. The `initialize` response's `result.serverInfo` shows `"name":"obsidian-link-resolver-mcp"` and the same version string.
