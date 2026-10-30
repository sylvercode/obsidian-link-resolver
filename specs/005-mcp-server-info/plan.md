# Implementation Plan: MCP Server Info Tool

**Branch**: `005-mcp-server-info` | **Date**: 2026-10-29 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-mcp-server-info/spec.md`

## Summary

Add a read-only, argument-free `get_server_info` MCP tool that returns the running server's semantic version and the ordered, de-duplicated list of canonical vault roots configured at startup. Fix the MCP handshake so `serverInfo` reports `obsidian-link-resolver-mcp` and the crate version instead of the `rmcp` SDK identity that `Implementation::from_build_env()` currently produces. Implement entirely inside the existing MCP binary crate with no new dependencies; existing tools, schemas, and the published 002 contract remain untouched.

## Technical Context

**Language/Version**: Rust, workspace edition 2021 (CI toolchain 1.98.1)

**Primary Dependencies**: Existing `rmcp` 3.5.1 (`server`, `macros`, `schemars`, `transport-io`), `serde`, `serde_json`, `schemars`, `tokio`, `clap`; no new dependencies.

**Storage**: N/A for this feature. The tool reads only in-memory startup configuration (`ResolverServer::allowed_vaults`) and never touches the `VaultCache` or the filesystem.

**Testing**: `cargo test -p obsidian-link-resolver-mcp`; MCP stdio contract tests (`tests/contract/`) and subprocess integration tests (`tests/integration/`) via the existing `StdioClient` harness. Extend `benches/mcp_warm.rs` with a `get_server_info` sample set and a 50 ms p50 gate (SC-004).

**Target Platform**: Existing native MCP stdio server targets (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64-gnu).

**Project Type**: Rust MCP server binary crate backed by the resolver library.

**Performance Goals**: `get_server_info` p50 under 50 ms on the existing 5,000-note benchmark vault, including the very first call before any vault has been indexed. Existing resolve/generation 100 ms p50 gates unchanged.

**Constraints**: Read-only; no arguments (extra arguments ignored, never rejected); deterministic output with no timestamps; compact stable field names (`version`, `vaults[].path`); raw canonical paths with no cosmetic rewriting (Windows `\\?\` preserved); handshake `serverInfo.name` = `obsidian-link-resolver-mcp` and `serverInfo.version` = tool `version`; zero behavior change for `resolve_obsidian_link` and `generate_obsidian_link`.

**Scale/Scope**: One new tool, one small result model, one handshake change, a one-line startup de-duplication, contract + integration tests, and a benchmark extension. Live vault health, cache statistics, uptime, and build metadata beyond the version are out of scope.

## Constitution Check

*GATE: Evaluated before Phase 0 research and re-checked after Phase 1 design.*

| Principle | Requirement | Plan compliance |
|---|---|---|
| I. Performance & Agent Efficiency | Low latency and minimal per-call overhead | Tool serializes in-memory configuration only; no cache access, no filesystem walk; measured in `mcp_warm` with a 50 ms p50 gate. **Pass.** |
| II. CLI Interface & Consistent Output | Stable, compact, deterministic machine interfaces | CLI unaffected. Structured JSON result with two short stable fields, no timestamps, byte-identical across calls. **Pass.** |
| III. Test-First | Tests precede behavior changes | Contract tests (schema, description, handshake, determinism, argument tolerance) and integration tests (1/2/3 vaults, duplicates, order, no writes) are written before implementation. **Pass.** |
| IV. Integration Testing | Validate real invocation and error forms | Subprocess stdio tests launch the real binary with temporary vaults via `StdioClient::with_vaults`. **Pass.** |
| V. Observability, Versioning & Simplicity | Minimal diagnostics and small stable surface | One additive tool; version sourced from `CARGO_PKG_VERSION` (semver); no new logging. **Pass.** |
| VI. Interoperability & Cross-Technology Integration | Stable language-agnostic interface | MCP JSON schema and structured JSON result; consumable from any MCP client. **Pass.** |
| VII. Reproducible Development Environment | Declare new tools/dependencies | None added; devcontainer unchanged. **Pass.** |
| VIII. CI & Release Gating | Required checks and performance tests gate changes | Existing workspace CI runs the new tests and the extended `mcp_warm` benchmark; the 002 contract compatibility check is unaffected because the published contract file is not modified. **Pass.** |
| IX. Native Documentation | Rustdoc for every new/changed symbol | `ServerInfo`, `ConfiguredVault`, the tool method, and any helper receive `///` docs in the same change. **Pass, gated in implementation.** |

**Pre-research result**: PASS. All product clarifications are resolved in the spec. No constitution exception is requested.

## Phase 0: Research Decisions

Decisions and evidence are recorded in [research.md](research.md). Key findings: `rmcp` tools declared without a `Parameters` extractor receive the schema `{"type":"object","properties":{}}` and silently ignore supplied arguments, which satisfies FR-001 without custom dispatch; `ServerConfig::new` fills `serverInfo` from `rmcp`'s own build env (`"rmcp"`, `"3.5.1"`), so FR-008 requires `with_server_info(Implementation::new(...))`; `CallToolResult::structured` already mirrors the value into text content; canonical roots are de-duplicated at construction time with first-occurrence order, which is behavior-neutral for the existing tools.

## Phase 1: Design

- [Data model](data-model.md) defines `ServerInfo` and `ConfiguredVault`, their validation rules, and the startup de-duplication rule.
- [MCP contract](contracts/mcp-server-info.md) defines the tool schema, structured result, handshake fields, and compatibility guarantees.
- [Quickstart](quickstart.md) defines the contract/integration validation and benchmark invocation.

### Design Decisions

1. Register `get_server_info` in the existing `#[tool_router] impl ResolverServer` block as `async fn get_server_info(&self) -> Result<CallToolResult, String>` with no `Parameters` argument. The existing `call_tool` override already forwards every non-`resolve_obsidian_link` name to `tool_router`, so no dispatch change is needed.
2. Model the result as `ServerInfo { version: String, vaults: Vec<ConfiguredVault> }` and `ConfiguredVault { path: String }`, both `Serialize`, serialized through `CallToolResult::structured`. The version is a shared `SERVER_VERSION` constant equal to `env!("CARGO_PKG_VERSION")` of the MCP crate (workspace-inherited, currently `2.0.0`); the handshake name is a shared `SERVER_NAME` constant.
3. Build the result from `self.allowed_vaults` on each call (`Path::to_string_lossy().into_owned()`, no normalization). The cache is not consulted, so the first call never triggers indexing.
4. In `ResolverServer::new`, de-duplicate canonical roots while preserving first-occurrence order, so `allowed_vaults` is already the exact list the tool reports. This is behavior-neutral for `select_allowed_vault` / `select_generation_root` (membership and deepest-prefix selection return the same path with or without duplicates).
5. Change `get_info` to `ServerConfig::new(...).with_server_info(Implementation::new("obsidian-link-resolver-mcp", env!("CARGO_PKG_VERSION"))).with_instructions("Resolve Obsidian links.")`; the instructions string stays unchanged so `mcp_tool_metadata` contract assertions still hold.
6. Tool description states what is returned and that no arguments are required, e.g. "Return the running server version and the vault roots configured at startup as canonical paths. Takes no arguments. Call it first to learn which vaults the link tools can access."
7. Extend `benches/mcp_warm.rs` with a `get_server_info` sample set reported as `server_info_p50_ms` (gate: 50 ms) and a `server_info_first_call_ms` measurement taken before the resolve warm-up. The first call is a single sample on a cold stdio pipe, so it is gated at 100 ms (the existing gate magnitude) to absorb CI jitter while still proving no vault indexing occurred (indexing the 5,000-note vault takes far longer). The warm p50 at 50 ms is the SC-004 budget.
8. No change to `specs/002-mcp-wrapper/contracts/mcp-contract.json`, `MIGRATION.md`, the CLI crate, or the resolver library.

### Intended Source and Test Structure

```text
crates/obsidian-link-resolver-mcp/
├── src/
│   └── mcp.rs                          # ServerInfo/ConfiguredVault models, get_server_info tool,
│                                       # startup de-dup in ResolverServer::new, get_info server identity
├── tests/
│   ├── contract.rs                     # Register mcp_server_info module
│   ├── contract/
│   │   └── mcp_server_info.rs          # tools/list schema + description, handshake serverInfo,
│   │                                   # argument tolerance, determinism, text/structured parity
│   ├── integration.rs                  # Register mcp_server_info module
│   ├── integration/
│   │   └── mcp_server_info.rs          # 1/2/3 vaults in order, duplicates + symlink alias once,
│   │                                   # no vault writes, response before any resolve
│   └── support/
│       └── mcp_client.rs               # Existing harness; `request` is sufficient (no change required)
└── benches/
    └── mcp_warm.rs                     # server_info samples + 50 ms p50 gate
```

Feature artifacts live beside this plan: `research.md`, `data-model.md`, `contracts/mcp-server-info.md`, and `quickstart.md`. `tasks.md` remains the output of the separate task-generation phase.

## Post-Design Constitution Check

| Principle | Result |
|---|---|
| I. Performance & Agent Efficiency | **Pass**: in-memory serialization only; 50 ms p50 gate added to the existing benchmark. |
| II. CLI Interface & Consistent Output | **Pass**: CLI unaffected; result has two stable fields and no non-deterministic content. |
| III. Test-First | **Pass**: contract and integration scenarios are enumerated before implementation. |
| IV. Integration Testing | **Pass**: real-binary stdio tests over temporary vault layouts, including symlink aliases. |
| V. Observability, Versioning & Simplicity | **Pass**: additive tool; semver from crate metadata; no new flags or logging. |
| VI. Interoperability & Cross-Technology Integration | **Pass**: language-neutral MCP schema and JSON result. |
| VII. Reproducible Development Environment | **Pass**: no new dependency or tool. |
| VIII. CI & Release Gating | **Pass**: existing CI, contract check, and benchmark remain authoritative. |
| IX. Native Documentation | **Pass with implementation gate**: all added Rust symbols must carry Rustdoc. |

No violations or unjustified complexity. The implementation must not change the existing tool contracts, write to vault files, or touch the vault cache from the new tool.

## Complexity Tracking

No constitution violations require justification.
