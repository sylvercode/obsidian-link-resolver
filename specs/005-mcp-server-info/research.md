# Research: MCP Server Info Tool

**Feature**: [MCP Server Info Tool](spec.md)  
**Date**: 2026-10-29

## Decisions

### Declare the Tool Without a Parameters Extractor

**Decision**: Register `get_server_info` with `#[tool]` as a method that takes only `&self`. Do not define a request struct.

**Rationale**: In `rmcp-macros` 3.5.1 a tool method with no `Parameters<T>` argument falls back to `rmcp::handler::server::common::schema_for_empty_input()`, which advertises `{"type":"object","properties":{}}`. Because no extractor consumes `arguments`, any supplied object is left untouched and the call succeeds. This satisfies FR-001 (ignore extra arguments, never fail) with zero custom code. The existing `call_tool` override in `mcp.rs` already routes every tool name other than `resolve_obsidian_link` to `self.tool_router`, so no dispatch change is needed.

**Alternatives considered**: An empty `#[derive(Deserialize, JsonSchema)] struct` with `deny_unknown_fields`; rejected because it would reject extra arguments, contradicting the clarified spec. An empty struct without `deny_unknown_fields`; rejected as redundant with the macro default and adds a symbol to document.

### Source the Version from Crate Metadata

**Decision**: Report `env!("CARGO_PKG_VERSION")` evaluated inside `obsidian-link-resolver-mcp`. The crate inherits `version.workspace = true` (currently `2.0.0`), so the value tracks the release tag.

**Rationale**: It is a compile-time constant, already semver-shaped, identical across calls (FR-005), and matches the version the release pipeline embeds. Tests in the same crate can compare against their own `env!("CARGO_PKG_VERSION")` without parsing Cargo metadata.

**Alternatives considered**: A `--version` CLI flag via `#[command(version)]`; not required by the spec (the independent test says "e.g.") and out of scope to keep the CLI surface unchanged. Reading `Cargo.toml` at runtime; rejected as slower and unavailable in installed binaries.

### Set the Handshake Server Identity Explicitly

**Decision**: Build `get_info` as `ServerConfig::new(capabilities).with_server_info(Implementation::new("obsidian-link-resolver-mcp", env!("CARGO_PKG_VERSION"))).with_instructions("Resolve Obsidian links.")`.

**Rationale**: `InitializeResult::new` (aliased as `ServerConfig`) fills `server_info` with `Implementation::from_build_env()`, whose `env!("CARGO_CRATE_NAME")`/`env!("CARGO_PKG_VERSION")` are expanded inside the `rmcp` crate. The handshake therefore currently advertises `"rmcp"` / `"3.5.1"`, which does not meet FR-008. `with_server_info` is the documented override. Keeping the instructions string unchanged preserves the `mcp_tool_metadata` contract test (purpose phrase present, length ≤ 40, no link syntax).

**Alternatives considered**: Leaving the default; rejected because it misidentifies the server. Adding `title`/`website_url`; rejected as beyond the spec.

### De-duplicate Canonical Roots at Construction

**Decision**: In `ResolverServer::new`, after canonicalization, drop any root equal to an earlier root while preserving first-occurrence order. `get_server_info` then reports `allowed_vaults` verbatim.

**Rationale**: The spec requires duplicates (after resolution, including symlink spellings) to appear once and in startup order. Doing this once at startup keeps a single source of truth. It is behavior-neutral for the existing tools: `select_allowed_vault` and `select_generation_root` use `starts_with`/`contains` membership and `max_by_key(components)`; duplicates are equal values, so the chosen root is identical with or without them. FR-009 is therefore preserved.

**Alternatives considered**: De-duplicating only in the info response; rejected because it leaves two views of the configuration and repeats work per call. Rejecting duplicate roots at startup; rejected because it would turn a currently accepted configuration into an error.

### Return Raw Canonical Paths as Strings

**Decision**: Convert each `PathBuf` with `to_string_lossy().into_owned()` and apply no further processing.

**Rationale**: `fs::canonicalize` already yields the absolute, symlink-resolved form the spec asks for, including the `\\?\` prefix on Windows, which the clarification says to keep. JSON strings must be UTF-8; lossy conversion is the standard Rust bridge and is lossless for every UTF-8 path. Non-UTF-8 vault paths are not supported anywhere else in the server (context paths are already strings) and are recorded as an accepted limitation.

**Alternatives considered**: Stripping `\\?\` via `dunce` or manual rewriting; rejected by the clarification and would add a dependency. Returning `Path` components as arrays; rejected as non-compact.

### Build the Response from In-Memory State on Each Call

**Decision**: Construct `ServerInfo` from `self.allowed_vaults` on every call and serialize it via `CallToolResult::structured(serde_json::to_value(info)?)`.

**Rationale**: The vector is immutable after startup and tiny (one entry per vault), so per-call construction is trivially cheap and deterministic. `CallToolResult::structured` sets `structuredContent` and mirrors `value.to_string()` into a text content block, matching the parity other tools already expose. The cache is never touched, so the first call cannot trigger indexing (edge case and SC-004).

**Alternatives considered**: Precomputing a `serde_json::Value` at startup; rejected as unnecessary state for a negligible gain.

### Measure SC-004 in the Existing Warm Benchmark

**Decision**: Extend `benches/mcp_warm.rs` to (a) time one `get_server_info` call immediately after `initialize`, before the resolve warm-up, reported as `server_info_first_call_ms`, and (b) collect 100 warm samples reported as `server_info_p50_ms` with an `assert!` gate of 50 ms.

**Rationale**: The benchmark already starts the real binary against the 5,000-note fixture and emits a machine-readable report consumed by CI artifacts, so adding two fields is the smallest change that evidences SC-004. Timing assertions in unit/integration tests would be flakier on shared runners.

**Alternatives considered**: A new benchmark binary; rejected as duplication. A timing assertion in an integration test; rejected for flakiness.

## Local Integration Findings

- `ResolverServer` (`crates/obsidian-link-resolver-mcp/src/mcp.rs`) stores `allowed_vaults: Vec<PathBuf>` canonicalized in `new()`; there is currently no de-duplication.
- `get_info` returns `ServerConfig::new(...).with_instructions("Resolve Obsidian links.")` with no explicit server identity.
- Tools are registered in a single `#[tool_router] impl ResolverServer` block; `#[tool_handler(router = self.tool_router)]` is applied to the `ServerHandler` impl.
- `tests/support/mcp_client.rs` exposes `StdioClient::with_vaults(Vec<String>)`, `initialize()`, `notify_initialized()`, and a generic `request(id, method, params)` that is sufficient to call `tools/call` with `{"name": "get_server_info", "arguments": ...}`; no harness change is required.
- No existing test asserts the number of tools in `tools/list`, so adding a tool is purely additive for the current suite.
- `scripts/check-mcp-contract-compatibility.ps1` validates `specs/002-mcp-wrapper/contracts/mcp-contract.json` against the last published release; this feature does not modify that file, so the check is unaffected and no `MIGRATION.md` note is required.
- `benches/mcp_warm.rs` already enforces 100 ms p50 gates for resolve and generation and prints a JSON report; the new fields slot into the same report.

## Remaining Risk

Windows canonical paths carry the `\\?\` prefix; the integration tests must compute expectations with `fs::canonicalize` rather than hard-coded strings so they pass on the `x86_64-pc-windows-gnu` CI target. Symlink-alias de-duplication tests should be skipped or guarded on platforms where creating symlinks requires elevated privileges.
