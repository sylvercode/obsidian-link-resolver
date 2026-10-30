---
description: "Task list for MCP server info tool"
---

# Tasks: MCP Server Info Tool

**Input**: Design documents from `specs/005-mcp-server-info/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/mcp-server-info.md`, `quickstart.md`

**Tests**: Required before implementation by Constitution Principles III and IV. Every task that adds or changes a Rust symbol MUST include Rustdoc (`///`) in the same change (Principle IX).

**Organization**: Tasks are grouped by user story. Test modules are registered once in a foundational phase so each story's test tasks only add cases to existing files.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Rust workspace; all work is inside the MCP binary crate `crates/obsidian-link-resolver-mcp/`. No changes to the resolver library, the CLI crate, `specs/002-mcp-wrapper/contracts/mcp-contract.json`, or `MIGRATION.md`.

---

## Phase 1: Setup

**Purpose**: Project initialization

The existing Rust workspace, MCP crate, `rmcp` 3.5.1 dependency set, and `StdioClient` test harness (`tests/support/mcp_client.rs`) already satisfy this feature's setup needs. No new dependencies or tools are required. No setup tasks.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Register the new test modules and the shared result model that every story depends on.

- [ ] T001 [P] Create `crates/obsidian-link-resolver-mcp/tests/contract/mcp_server_info.rs` with a module doc comment and register it in `crates/obsidian-link-resolver-mcp/tests/contract.rs` as `#[path = "contract/mcp_server_info.rs"] mod mcp_server_info;` (alphabetical position, after `mcp_link_generation`).
- [ ] T002 [P] Create `crates/obsidian-link-resolver-mcp/tests/integration/mcp_server_info.rs` with a module doc comment and register it in `crates/obsidian-link-resolver-mcp/tests/integration.rs` as `#[path = "integration/mcp_server_info.rs"] mod mcp_server_info;` (alphabetical position, after `mcp_resolver_parity`).
- [ ] T003 Define the documented result models in `crates/obsidian-link-resolver-mcp/src/mcp.rs`: `ServerInfo { version: String, vaults: Vec<ConfiguredVault> }` and `ConfiguredVault { path: String }`, both `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]`, field names exactly `version`, `vaults`, `path`, no other fields. Rustdoc must state: `version` is "Semantic version of the MCP server build (`MAJOR.MINOR.PATCH`, optional pre-release/build suffix). Sourced from the crate's `CARGO_PKG_VERSION`; identical to the handshake `serverInfo.version`"; `vaults` is "Ordered as supplied at startup, after canonicalization and de-duplication. Never empty"; `path` is "Canonical absolute path exactly as returned by the operating system (`fs::canonicalize`): symlinks and `.`/`..` resolved; Windows extended-length prefix `\\?\` preserved; no trailing-separator or case rewriting".

**Checkpoint**: Test modules compile (empty) and the result model exists; story work can begin.

---

## Phase 3: User Story 1 - Discover the configured vaults (Priority: P1) - MVP

**Goal**: Expose an argument-free `get_server_info` tool whose `vaults` list reports every startup-configured vault root once, in startup order, as raw canonical paths, without touching the cache or filesystem.

**Independent Test**: Start the server with one vault, call `get_server_info`, and verify the response lists exactly that vault's `fs::canonicalize` path. Repeat with two and three vaults and verify all appear in configuration order; pass a vault twice and verify it appears once.

### Tests for User Story 1

- [ ] T004 [P] [US1] Add contract tests in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_server_info.rs`: `tools/list` contains a tool named `get_server_info` whose `inputSchema` equals `{"type":"object","properties":{}}` (no `required`, no `additionalProperties`), and whose `description` contains (case-insensitive) the phrases "version", "configured at startup", and "no arguments"; `tools/call` with `arguments` omitted, with `arguments: {}`, and with `arguments: {"ignored": true, "extra": 1}` all return `isError: false` and byte-identical `structuredContent`; `structuredContent.vaults` is a non-empty array of objects each having exactly one key `path` of type string; the single `content[0].text` block parses to JSON equal to `structuredContent`.
- [ ] T005 [P] [US1] Add integration tests in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_server_info.rs` using `StdioClient::with_vaults` and `tempfile` directories: one, two, and three vaults appear in `vaults` in startup order with `path` equal to `fs::canonicalize(dir).to_string_lossy()` (never hard-coded strings, so Windows `\\?\` passes); the same directory passed twice appears once; a symlink alias to an already-listed vault appears once (guarded with `#[cfg(unix)]` or skipped when symlink creation fails); a recursive snapshot (paths + sizes + mtimes) of every vault directory is identical before and after the call; the tool answers successfully on a fresh server before any `resolve_obsidian_link` call has been made; with the same vault passed twice (`--vault A --vault A`), `resolve_obsidian_link` and `generate_obsidian_link` return `structuredContent` identical to a server started with a single `--vault A` (FR-009 regression for the startup de-duplication in T006).

### Implementation for User Story 1

- [ ] T006 [US1] De-duplicate canonical roots in `ResolverServer::new` in `crates/obsidian-link-resolver-mcp/src/mcp.rs`: after canonicalization and the directory check, keep an entry only if no earlier kept entry is equal to it (first-occurrence order preserved). Update the `new` Rustdoc to document the de-duplication rule. Do not change `select_allowed_vault` or `select_generation_root`.
- [ ] T007 [US1] Add the tool in the existing `#[tool_router] impl ResolverServer` block in `crates/obsidian-link-resolver-mcp/src/mcp.rs`: `#[tool(name = "get_server_info", description = "Return the running server version and the vault roots configured at startup as canonical paths. Takes no arguments. Call it first to learn which vaults the link tools can access.")] async fn get_server_info(&self) -> Result<CallToolResult, String>` with no `Parameters` argument. Build `ServerInfo` from `self.allowed_vaults` via `Path::to_string_lossy().into_owned()` (no normalization), set `version` to `env!("CARGO_PKG_VERSION")`, and return `CallToolResult::structured(serde_json::to_value(info)?)`. Must not read `self.cache` or the filesystem. Rustdoc the method (purpose, no-input contract, read-only side effects, return shape).
- [ ] T008 [US1] Run `cargo test -p obsidian-link-resolver-mcp --test contract mcp_server_info` and `cargo test -p obsidian-link-resolver-mcp --test integration mcp_server_info`; confirm T004/T005 pass (including the duplicate-vault parity case) and the existing `mcp_tool`, `mcp_tool_metadata`, `mcp_link_generation`, `mcp_determinism`, and `mcp_compatibility` suites still pass unchanged (FR-009, SC-006).

**Checkpoint**: `get_server_info` is discoverable, tolerant of arguments, deterministic, and reports configured vaults correctly for 1/2/3 vaults and duplicates.

---

## Phase 4: User Story 2 - Discover the running server version (Priority: P2)

**Goal**: The `version` field of `get_server_info` equals the MCP crate's declared build version as a semantic version string.

**Independent Test**: Call `get_server_info` and verify `structuredContent.version` equals the test crate's `env!("CARGO_PKG_VERSION")` and matches the semver grammar.

### Tests for User Story 2

- [ ] T009 [US2] Add contract tests in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_server_info.rs`: `structuredContent.version` is a string equal to `env!("CARGO_PKG_VERSION")`; it matches `MAJOR.MINOR.PATCH` with optional `-prerelease` / `+build` suffix (validate by splitting on `+`, then `-`, then asserting three dot-separated numeric components); two consecutive calls on the same process return byte-identical `structuredContent` (SC-003).

### Implementation for User Story 2

- [ ] T010 [US2] Verify `get_server_info` in `crates/obsidian-link-resolver-mcp/src/mcp.rs` sources `version` from `env!("CARGO_PKG_VERSION")` evaluated in the MCP crate (workspace-inherited); add a `const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");` with Rustdoc and use it in the tool so US3 can share the same constant. Run `cargo test -p obsidian-link-resolver-mcp --test contract mcp_server_info` and confirm T009 passes.

**Checkpoint**: Reported version equals the build's declared version and is stable across calls.

---

## Phase 5: User Story 3 - Version visible in the connection handshake (Priority: P3)

**Goal**: The MCP `initialize` result's `serverInfo` reports `name: "obsidian-link-resolver-mcp"` and the same version string returned by `get_server_info`, instead of the `rmcp` SDK identity.

**Independent Test**: Connect a client, read `initialize` result `serverInfo`, and verify `name == "obsidian-link-resolver-mcp"` and `version == structuredContent.version` from a subsequent `get_server_info` call.

### Tests for User Story 3

- [ ] T011 [US3] Add contract tests in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_server_info.rs`: capture the `initialize` response from `StdioClient::initialize()` (extend the harness return value in `crates/obsidian-link-resolver-mcp/tests/support/mcp_client.rs` only if it currently discards the result); assert `result.serverInfo.name == "obsidian-link-resolver-mcp"`, `result.serverInfo.version == env!("CARGO_PKG_VERSION")`, `result.serverInfo.version` equals `structuredContent.version` from `get_server_info`, and `result.instructions == "Resolve Obsidian links."` (unchanged).

### Implementation for User Story 3

- [ ] T012 [US3] Change `get_info` in `crates/obsidian-link-resolver-mcp/src/mcp.rs` to `ServerConfig::new(capabilities).with_server_info(Implementation::new("obsidian-link-resolver-mcp", SERVER_VERSION)).with_instructions("Resolve Obsidian links.")`; import `rmcp::model::Implementation`; keep the instructions string unchanged so `mcp_tool_metadata` assertions still hold. Add a documented `const SERVER_NAME: &str = "obsidian-link-resolver-mcp";` and update the `get_info` Rustdoc. Run `cargo test -p obsidian-link-resolver-mcp --test contract` and confirm T011 and `mcp_tool_metadata` pass.

**Checkpoint**: Client inspectors show the correct server name and version without calling a tool.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Evidence the performance budget and confirm the full workspace gates.

- [ ] T013 Extend `crates/obsidian-link-resolver-mcp/benches/mcp_warm.rs`: immediately after `initialize`/`notifications/initialized` and before the resolve warm-up, time one `tools/call` of `get_server_info` and report it as `server_info_first_call_ms`; after the existing warm-up, collect 100 `get_server_info` samples and report the median as `server_info_p50_ms` in the printed JSON report; add `assert!(server_info_first_call <= Duration::from_millis(50), "server_info first call exceeded 50 ms: {report}")` and `assert!(server_info_median <= Duration::from_millis(50), "server_info p50 exceeded 50 ms: {report}")` as the SC-004 gates (the first-call gate covers the "before any vault has been indexed" clause). Keep the existing `p50_ms` and `generation_p50_ms` 100 ms gates unchanged. Rustdoc any new helper.
- [ ] T014 Run `cargo bench -p obsidian-link-resolver-mcp --bench mcp_warm` and confirm the report contains `server_info_first_call_ms` and `server_info_p50_ms`, both under 50 ms.
- [ ] T015 Run the full gates from `specs/005-mcp-server-info/quickstart.md`: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`; then run `cargo clippy -p obsidian-link-resolver-mcp --all-targets -- -W clippy::missing_docs_in_private_items 2>&1 | grep -E "mcp\.rs|mcp_server_info\.rs|mcp_warm\.rs"` and confirm no hits for `ServerInfo`, `ConfiguredVault`, `SERVER_NAME`, `SERVER_VERSION`, `get_server_info`, or any new test/bench helper (the crate is a binary with no `missing_docs` lint configured, so `cargo doc` cannot detect undocumented items).
- [ ] T016 Perform the MCP smoke call from `specs/005-mcp-server-info/quickstart.md` against `target/debug/obsidian-link-resolver-mcp` started with two `--vault` directories; verify `structuredContent` shape `{"version":"<crate version>","vaults":[{"path":"..."},{"path":"..."}]}` and that `initialize` `serverInfo` reports `obsidian-link-resolver-mcp` with the same version. Confirm `specs/002-mcp-wrapper/contracts/mcp-contract.json` and `MIGRATION.md` are unmodified (`git status --porcelain -- specs/002-mcp-wrapper MIGRATION.md` is empty).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No tasks.
- **Foundational (Phase 2)**: T001 and T002 run in parallel (different test files); T003 is independent of both. All three must complete before story phases.
- **User Story 1 (Phase 3)**: T004 and T005 (tests) in parallel, then T006 → T007 (both edit `src/mcp.rs`, sequential), then T008.
- **User Story 2 (Phase 4)**: Depends on T007 (tool exists). T009 then T010.
- **User Story 3 (Phase 5)**: Depends on T010 (shared `SERVER_VERSION` constant). T011 then T012.
- **Polish (Phase 6)**: Depends on all stories. T013 → T014; T015 and T016 after T014.

### User Story Dependencies

- **US1 (P1)**: Depends only on Phase 2; delivers the MVP tool with vault discovery.
- **US2 (P2)**: Extends US1's tool with version assertions; implementation is a verification plus constant extraction.
- **US3 (P3)**: Reuses the US2 constant for the handshake; independent of the vault list.

### Parallel Opportunities

- Phase 2: T001 ‖ T002 ‖ T003.
- Phase 3: T004 ‖ T005 (contract vs. integration files).
- Phase 4 and 5 test tasks (T009, T011) can be authored alongside T004 in the same contract file by one author, but are sequenced here to keep each story independently verifiable.

---

## Implementation Strategy

1. **MVP (US1)**: Complete Phases 2–3. This delivers the tool, argument tolerance, determinism, de-duplication, and vault discovery — the core value of the feature.
2. **Increment 2 (US2)**: Phase 4 locks the version contract and extracts the shared constant.
3. **Increment 3 (US3)**: Phase 5 fixes the handshake identity.
4. **Polish**: Phase 6 adds the benchmark gate and runs full workspace verification.

Each increment leaves the existing `resolve_obsidian_link` and `generate_obsidian_link` tools untouched (FR-009) and is independently shippable.
