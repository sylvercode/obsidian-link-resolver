---
description: "Executable task list for the Obsidian Link Resolver MCP wrapper"
---

# Tasks: Obsidian Link Resolver MCP Wrapper

**Input**: Design documents from `/specs/002-mcp-wrapper/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/mcp.md`, and `quickstart.md`

**Tests**: Tests are required by the feature specification and project constitution. Author tests before implementation for protocol, result parity, cache behavior, diagnostics, installer/release behavior, determinism, and performance.

**Organization**: Tasks are grouped by user story in priority order. `[P]` marks tasks that can proceed independently in separate files without unfinished prerequisites.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: May be implemented in parallel with other marked tasks because it works in separate files and has no unfinished prerequisite.
- **[Story]**: Identifies the user story from `spec.md`.
- Every task names the repository file or files it changes.

## Phase 1: Setup

**Purpose**: Add MCP dependencies and the dedicated executable target.

- [ ] T001 Add `rmcp` 3.5.0 with only `server`, `macros`, `schemars`, and `transport-io` features; Tokio, `notify` 8.2.0, `structured-logger` 1.0.5 with default features disabled, and structured `log` support; declare the `obsidian-link-resolver-mcp` binary in `Cargo.toml` and refresh `Cargo.lock`.
- [ ] T002 Pin Rust to 1.98.1 consistently in `.devcontainer/devcontainer.json`, `.github/workflows/ci.yml`, and `.github/workflows/release.yml`; verify all three use the same explicit toolchain version, retain release-target support in the devcontainer, and update CI to explicitly build/test the MCP binary alongside formatting and Clippy checks.

## Phase 2: Foundational

**Purpose**: Establish prerequisites shared by the stories.

**Checkpoint**: Complete Phase 1 before story implementation. No additional shared implementation blocks the user stories; server bootstrap is part of US1.

## Phase 3: User Story 1 - Resolve links through a standard tool interface (Priority: P1) 🎯 MVP

**Goal**: Provide a local MCP stdio server with a `resolve_obsidian_link` tool that accepts a link and context path and returns the existing resolver result.

**Independent Test**: Start the binary, initialize an MCP session, list the tool, and resolve a known fixture link; verify structured content contains the expected target path and location.

### Tests for User Story 1

> Write these protocol tests first and verify they fail before implementing the tool.

- [ ] T003 [P] [US1] Add MCP tool discovery and input-schema contract tests for required `link` and `context_path`, optional vault/emplacement arguments, and missing or wrong-type required arguments being rejected as MCP tool/protocol errors in `tests/contract/mcp_tool.rs`.
- [ ] T004 [P] [US1] Add child-process stdio integration tests for MCP initialize, tool listing, a valid fixture resolution, and missing/wrong-type argument calls returning MCP errors without resolver result content in `tests/integration/mcp_stdio.rs`.

### Implementation for User Story 1

- [ ] T005 [US1] Implement the typed `resolve_obsidian_link` request handler and return the resolver record as both structured content and compact JSON text in `src/mcp.rs`, documenting new Rust symbols with Rustdoc.
- [ ] T006 [US1] Register the MCP module and implement the `obsidian-link-resolver-mcp` startup path, optional `--vault` default, and stdio transport in `src/lib.rs` and `src/bin/obsidian-link-resolver-mcp.rs`, documenting new Rust symbols with Rustdoc.

**Checkpoint**: The MCP server launches for a client and resolves a fixture link without changing the existing CLI.

## Phase 4: User Story 2 - Preserve the current resolver contract (Priority: P2)

**Goal**: Preserve resolver statuses, reasons, candidate ordering, path semantics, deterministic result fields, and emplacement through the MCP adapter.

**Independent Test**: For identical fixture requests against unchanged vault contents, compare MCP and direct resolver results and assert matching status, target path/range, reasons, candidates, emplacement, and primary record; verify published MCP contracts remain compatible within a major version.

### Tests for User Story 2

> Write parity and determinism tests first and verify they fail before changing result mapping.

- [ ] T007 [P] [US2] Add contract tests that submit identical requests repeatedly against unchanged fixture contents and assert identical primary result records with no nondeterministic fields in `tests/contract/mcp_determinism.rs`.
- [ ] T008 [P] [US2] Add parity cases for resolved, unresolved, missing sub-target, ambiguous, and resolver error outcomes—including schema-valid link, context, and vault values rejected by the resolver—plus same-file, heading, block, attachment, and emplacement results in `tests/integration/mcp_resolver_parity.rs`; assert resolver `error` outcomes remain normal tool results with their reason fields.
- [ ] T009 [P] [US2] Add MCP contract compatibility tests in `tests/contract/mcp_compatibility.rs` covering unchanged argument names/requiredness, result fields/meanings, and outcome semantics within a major version; allow additive optional fields and arguments, and reject breaking changes unless the candidate uses a new major version with migration notes.

### Implementation for User Story 2

- [ ] T010 [US2] Define a machine-readable MCP contract artifact in `specs/002-mcp-wrapper/contracts/mcp-contract.json` and implement compatibility validation in `scripts/check-mcp-contract-compatibility.sh`; compare the candidate with the latest published contract in the same major version, establish the first published contract as the baseline, and require a new major version plus migration notes for breaking changes.
- [ ] T011 [US2] Preserve all `ResolutionTarget` fields and five resolver outcome categories without wrapper-only result fields, keeping resolver `error` outcomes as normal tool results rather than MCP errors, in `src/mcp.rs` using existing resolver APIs and `specs/001-link-resolver/contracts/result.schema.json`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: Each conformance-corpus request produces the same resolver outcome through MCP as through the existing resolver.

## Phase 5: User Story 4 - Reuse a cached vault index (Priority: P2)

**Goal**: Reuse a per-canonical-root index, refresh it on detected changes and at least every 60 seconds, and never serve a known-stale index after refresh failure.

**Independent Test**: Verify same-root reuse without a per-call directory walk, refresh after invalidation or a simulated missed event, root isolation, and generation consistency when invalidation occurs during a call.

### Tests for User Story 4

> Add cache behavior and benchmark coverage first; use deterministic invalidation and a controllable clock rather than wall-clock sleeps.

- [ ] T012 [P] [US4] Add cache unit tests for canonical-root reuse, dirty-state coalescing, separate roots, atomic refresh, failed-refresh behavior, in-flight generation consistency, and changed note contents becoming visible after refresh in `tests/unit/mcp_cache.rs`.
- [ ] T013 [P] [US4] Add integration tests for watcher invalidation, watcher unavailability, and request-independent periodic refresh within 60 seconds in `tests/integration/mcp_cache_refresh.rs`.
- [ ] T014 [P] [US4] Add a warm-cache benchmark for 100 consecutive end-to-end MCP calls against the approximately 5,000-note fixture in `benches/mcp_warm.rs` and `Cargo.toml`; calculate p50, record runner and fixture versions, and make the benchmark fail when p50 exceeds 100 ms on the designated release benchmark runner.

### Implementation for User Story 4

- [ ] T015 [US4] Separate canonical root selection/validation from vault index enumeration while preserving `detect_root` behavior in `src/vault.rs`; document new or modified Rust symbols with Rustdoc.
- [ ] T016 [US4] Implement the process-scoped cache keyed by canonical root, watcher dirty hints, coalesced invalidation, atomic index replacement, and a background full rescan no later than 60 seconds in `src/mcp.rs`; document new Rust symbols with Rustdoc.
- [ ] T017 [US4] Ensure each request resolves against the index generation selected at its start, the next request refreshes a root invalidated in flight, and refresh failures return an error rather than stale resolution in `src/mcp.rs`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: Repeated resolutions reuse indexes, detected changes refresh before the next call, missed events recover within 60 seconds, and failed refreshes do not produce stale successes.

## Phase 6: User Story 5 - Install the MCP server in supported clients (Priority: P2)

**Goal**: Publish MCP binaries for the five supported targets and provide separate MCP installer commands and local-stdio client setup without changing the CLI-only installer default.

**Independent Test**: On each supported installer platform, verify default CLI and explicit MCP selection install the correct executable; verify release mapping includes all five target assets and both documented clients can discover and call the tool.

### Tests for User Story 5

> Add installer and release-matrix tests before changing the installer or release workflow.

- [ ] T018 [P] [US5] Add mocked installer integration tests for Bash and PowerShell, covering CLI-only defaults, explicit MCP selection, supported asset names, and installed executable paths in `tests/integration/installers.rs`; run each script test on its supported CI runner in `.github/workflows/ci.yml`.
- [ ] T019 [P] [US5] Add release-matrix contract tests that verify Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64 MCP artifact mapping, publication only after full-test/benchmark/contract-compatibility gates, and inclusion of the machine-readable MCP contract artifact in release assets in `tests/integration/release_assets.rs`.

### Implementation for User Story 5

- [ ] T020 [P] [US5] Add explicit `--component mcp` selection to `scripts/install.sh`, preserving the default CLI component and supporting MCP asset names and executable paths.
- [ ] T021 [P] [US5] Add explicit `-Component mcp` selection to `scripts/install.ps1`, preserving the default CLI component and supporting MCP asset names and executable paths.
- [ ] T022 [US5] Extend `.github/workflows/release.yml` to build and stage the MCP executable for Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64; run the full test suite, the T014 benchmark threshold gate, and MCP contract compatibility validation against the latest published same-major contract before staging or publishing artifacts, then publish the current contract artifact with the release assets.
- [ ] T023 [US5] Document latest-version install/update, pinned-version and manual-fallback MCP installer commands, plus local-stdio registration and vault configuration for GitHub Copilot in VS Code and Claude Desktop in `README.md`.
- [ ] T024 [US5] Validate tool discovery and one valid call in the latest stable GitHub Copilot in VS Code and Claude Desktop configurations, recording exact client versions and results in `specs/002-mcp-wrapper/quickstart.md`.

**Checkpoint**: MCP assets exist for all five release targets, the existing installer default remains CLI-only, and both supported clients can launch and call the tool.

## Phase 7: User Story 3 - Offer discoverable, agent-friendly tool metadata (Priority: P3)

**Goal**: Make the tool's purpose, arguments, outcomes, and choice between a simple target line and structured emplacement clear to clients.

**Independent Test**: Inspect `tools/list` metadata and schema; verify the description recommends emplacement for precise section/shard reads and a simple target line for a single point target.

### Tests for User Story 3

> Add metadata assertions before changing the published tool description/schema.

- [ ] T025 [P] [US3] Add tool description, argument schema, and result-shape assertions—including emplacement guidance—in `tests/contract/mcp_tool_metadata.rs`.

### Implementation for User Story 3

- [ ] T026 [US3] Define stable MCP tool metadata and schema descriptions for required/optional arguments and resolver outcomes, including point-target versus emplacement guidance, in `src/mcp.rs`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: A client can discover the tool and choose appropriate result detail from its metadata without guesswork.

## Phase 8: User Story 6 - Inspect MCP behavior when troubleshooting (Priority: P3)

**Goal**: Add opt-in structured stderr diagnostics for lifecycle, request, cache, watcher, and failure events without changing protocol output or exposing request content.

**Independent Test**: Compare subprocess stdout and tool results with diagnostics off/on; with diagnostics enabled, validate JSON-lines events and confirm request text, paths, note contents, serialized arguments, and raw errors are absent.

### Tests for User Story 6

> Write subprocess tests first because the logger is process-global and the stdout/stderr boundary is part of the contract.

- [ ] T027 [P] [US6] Add child-process tests for diagnostics-disabled silence, enabled lifecycle/request/cache events, fixed event/failure categories, and stdout/result parity in `tests/integration/mcp_diagnostics.rs`; include unique sentinel values in request text, context/vault paths, note contents, serialized arguments, and raw errors and assert none appear in stderr.

### Implementation for User Story 6

- [ ] T028 [US6] Add `--diagnostics` handling and initialize `structured-logger` only when enabled, routing the fixed MCP diagnostics target to stderr and all other targets to a sink in `src/bin/obsidian-link-resolver-mcp.rs` and `src/mcp.rs`; document new or modified Rust symbols with Rustdoc.
- [ ] T029 [US6] Emit only fixed event names, static messages/targets, allow-listed statuses/failure categories, process-local request sequence numbers, and durations; distinguish resolver `error` results from failed requests and redact request text, paths, note contents, serialized arguments, and unfiltered error messages in `src/mcp.rs`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: Optional diagnostics report operational outcomes on stderr only, leave MCP results unchanged, and remain absent by default.

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Validate the complete feature and keep implementation guidance aligned with the code and release pipeline.

- [ ] T030 Update `specs/002-mcp-wrapper/quickstart.md` with implemented binary/installer commands, release benchmark invocation, contract compatibility validation and published baseline behavior, and validated end-to-end scenarios while preserving the normative conformance corpus.
- [ ] T031 Audit every new or modified Rust module, type, enum and variant, function, method, member, parameter, return value, and constant in `src/lib.rs`, `src/vault.rs`, `src/mcp.rs`, and `src/bin/obsidian-link-resolver-mcp.rs`; add native Rustdoc explaining purpose, constraints, semantics, and side effects, then verify documentation builds with `cargo doc --no-deps`.
- [ ] T032 Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, focused MCP tests including determinism with unchanged vault contents and contract compatibility, installer/release-matrix tests, `cargo test`, and the MCP warm-run benchmark; verify the designated release benchmark runner enforces the ≤100 ms p50 threshold.

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 establishes the MCP target and dependencies; T002 configures the development environment and CI coverage after the binary is declared.
- **Foundational (Phase 2)**: No separate implementation tasks; MCP server bootstrap is delivered in US1.
- **User Stories (Phases 3-8)**: US1 is the prerequisite for every other story. US2 and US4 build on the working adapter; US5 requires the MCP executable. US3 and US6 also require US1.
- **Polish (Phase 9)**: Depends on completion of the selected user stories.

### User Story Dependencies

- **US1 (P1)**: Starts after Setup; no other story dependency.
- **US2 (P2)**: Depends on US1 because parity, determinism, and published-contract compatibility require a callable MCP tool and a defined MCP contract artifact.
- **US4 (P2)**: Depends on US1 and US2 so the cache is verified against the preserved resolver contract.
- **US5 (P2)**: Depends on US1 for a buildable MCP executable. Installer/release/documentation work can proceed separately from cache work once asset naming is established.
- **US3 (P3)**: Depends on US1; metadata implementation shares `src/mcp.rs` with US2, US4, and US6.
- **US6 (P3)**: Depends on US1; diagnostics implementation shares server lifecycle code in `src/mcp.rs` with US3 and cache work.

### Parallel Opportunities

- T003 and T004 can be authored in parallel; both precede T005-T006.
- T007-T009 can be authored in parallel; T010-T011 follow their tests, with T011 also depending on the US1 adapter.
- T012-T014 can be authored in parallel; T015-T017 depend on the tests and should be coordinated because they modify the resolver/cache boundary.
- T018-T019 can be authored in parallel before T020-T022; T020 and T021 can then be implemented in parallel because they modify separate installer files.
- T025 and T027 can be authored separately after US1; coordinate implementation changes to `src/mcp.rs` across US2, US3, US4, and US6.
- T031 is the final explicit constitution audit for native Rustdoc; Rustdoc should also be written alongside every implementation task that adds or modifies symbols.

### Parallel Execution Examples

**User Story 1**

```text
In parallel: T003 (tool contract tests) and T004 (stdio integration tests)
Then: T005 (tool handler) and T006 (server startup/stdio transport)
```

**User Story 4**

```text
In parallel: T012 (cache unit tests), T013 (refresh integration tests), T014 (warm-run benchmark)
Then: T015 (root/index boundary) → T016-T017 (cache lifecycle and request consistency)
```

**User Story 5**

```text
In parallel: T018 (installer tests) and T019 (release-matrix tests)
Then: T020/T021 (installer implementations); T022 (release matrix, test, benchmark, and compatibility gates); T023-T024 (docs and client validation)
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Setup (Phase 1).
2. Complete US1 protocol tests and implementation (Phase 3).
3. Stop and validate tool discovery, one valid stdio call, and the independent US1 test criterion.
4. Demo the standalone MCP binary without changing the existing CLI.

### Incremental Delivery

1. Deliver US1 as the smallest usable MCP resolver.
2. Add US2 result parity/determinism, then US4 cache reuse and refresh.
3. Add US5 installation/release support and validate both supported clients.
4. Add US3 discoverability improvements and US6 opt-in diagnostics.
5. Run complete quickstart, CI, release-target, documentation, and benchmark validation.
