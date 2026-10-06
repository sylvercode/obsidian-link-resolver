---
description: "Executable task list for the Obsidian Link Resolver MCP wrapper"
---

# Tasks: Obsidian Link Resolver MCP Wrapper

**Input**: Design documents from `/specs/002-mcp-wrapper/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/mcp.md`, and `quickstart.md`

**Tests**: Included because the feature specification and project constitution require protocol, parity, cache, diagnostics, and performance validation before implementation.

**Organization**: Tasks are grouped by user story in priority order; each story includes an independent test criterion.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: May be implemented in parallel with other marked tasks because it works in separate files and has no unfinished prerequisite.
- **[Story]**: Identifies the user story from `spec.md`.
- Every task names the repository file or files it changes.

## Phase 1: Setup

**Purpose**: Add the MCP build dependencies and dedicated executable target.

- [ ] T001 Add `rmcp` 3.5.0 with only its `server`, `macros`, `schemars`, and `transport-io` features; Tokio, `notify` 8.2.0, `structured-logger` 1.0.5 with default features disabled, and structured `log` support; declare the `obsidian-link-resolver-mcp` binary in `Cargo.toml` and refresh `Cargo.lock`.
- [ ] T002 [P] Pin the Rust toolchain to 1.98.1 and retain release-target support in `.devcontainer/devcontainer.json`; explicitly build/test the MCP binary in `.github/workflows/ci.yml` alongside formatting and Clippy checks.

---

## Phase 2: Foundational

**Purpose**: Establish prerequisites shared by the user stories.

**Checkpoint**: Complete Phase 1 before starting user-story work. No additional shared implementation blocks all stories; the server bootstrap is part of User Story 1.

---

## Phase 3: User Story 1 - Resolve links through a standard tool interface (Priority: P1) 🎯 MVP

**Goal**: Provide a local MCP stdio server with a `resolve_obsidian_link` tool that accepts a link and context path and returns the existing resolver result.

**Independent Test**: Start the binary, initialize an MCP session, list the tool, and resolve a known fixture link; verify a structured result contains the expected target path and location.

### Tests for User Story 1

> Write these protocol tests first and verify they fail before implementing the tool.

- [ ] T003 [P] [US1] Add MCP tool discovery and input-schema contract tests for required `link` and `context_path`, optional vault/emplacement arguments, and malformed arguments in `tests/contract/mcp_tool.rs`.
- [ ] T004 [P] [US1] Add child-process stdio integration tests for MCP initialize, tool listing, and a valid fixture resolution in `tests/integration/mcp_stdio.rs`.

### Implementation for User Story 1

- [ ] T005 [US1] Implement the typed `resolve_obsidian_link` request handler and return the resolver record as both structured content and compact JSON text in `src/mcp.rs`.
- [ ] T006 [US1] Register the MCP module and implement the `obsidian-link-resolver-mcp` startup path, optional `--vault` default, and stdio transport in `src/lib.rs` and `src/bin/obsidian-link-resolver-mcp.rs`.

**Checkpoint**: The MCP server can be launched by a client and resolve a fixture link without changing the existing CLI.

---

## Phase 4: User Story 2 - Preserve the current resolver contract (Priority: P2)

**Goal**: Preserve the resolver’s statuses, reasons, candidate ordering, path semantics, and emplacement fields through the MCP adapter.

**Independent Test**: For identical fixture requests, compare MCP results with the existing resolver and assert the same status, target path/range, reasons, candidates, and emplacement.

### Tests for User Story 2

> Write parity tests first and verify they fail before changing result mapping.

- [ ] T007 [P] [US2] Add parity cases for resolved, unresolved, missing sub-target, ambiguous, resolver error, same-file, heading, block, attachment, and emplacement results in `tests/integration/mcp_resolver_parity.rs`.

### Implementation for User Story 2

- [ ] T008 [US2] Preserve all `ResolutionTarget` fields and five resolver outcome categories without wrapper-only result fields in `src/mcp.rs`, using the existing resolver APIs and `specs/001-link-resolver/contracts/result.schema.json`.

**Checkpoint**: Each conformance-corpus request produces the same resolver outcome through MCP as through the existing resolver.

---

## Phase 5: User Story 4 - Reuse a cached vault index (Priority: P2)

**Goal**: Reuse a per-canonical-root index, refresh it on detected changes and at least every 60 seconds, and never serve a known-stale index after refresh failure.

**Independent Test**: Verify same-root reuse without a per-call directory walk, refresh after invalidation or a simulated missed event, root isolation, and generation consistency when invalidation occurs during a call.

### Tests for User Story 4

> Write cache behavior and benchmark coverage first; use deterministic invalidation and a controllable clock rather than wall-clock sleeps.

- [ ] T009 [P] [US4] Add cache unit tests for canonical-root reuse, dirty-state coalescing, separate roots, atomic refresh, failed-refresh behavior, and in-flight generation consistency in `tests/unit/mcp_cache.rs`.
- [ ] T010 [P] [US4] Add integration tests for watcher invalidation, watcher unavailability, and request-independent periodic refresh within 60 seconds in `tests/integration/mcp_cache_refresh.rs`.
- [ ] T011 [P] [US4] Add a warm-cache benchmark for 100 consecutive end-to-end MCP calls against the approximately 5,000-note fixture, reporting p50 and recording runner/fixture versions in `benches/mcp_warm.rs` and `Cargo.toml`.

### Implementation for User Story 4

- [ ] T012 [US4] Separate canonical root selection/validation from vault index enumeration while preserving `detect_root` behavior in `src/vault.rs`.
- [ ] T013 [US4] Implement the process-scoped cache keyed by canonical root, watcher dirty hints, coalesced invalidation, atomic index replacement, and a background full rescan no later than 60 seconds in `src/mcp.rs`.
- [ ] T014 [US4] Ensure each request resolves against the index generation selected at its start, the next request refreshes a root invalidated in flight, and refresh failures return an error rather than a stale resolution in `src/mcp.rs`.

**Checkpoint**: Repeated resolutions reuse indexes, detected changes refresh before the next call, missed events recover within 60 seconds, and failed refreshes do not produce stale successes.

---

## Phase 6: User Story 5 - Install the MCP server in supported clients (Priority: P2)

**Goal**: Publish MCP binaries for the five supported targets and provide separate MCP installer commands and local-stdio client setup without changing the CLI-only installer default.

**Independent Test**: On each supported installer platform, select the MCP component and verify the correct asset/executable is installed; follow the documented VS Code and Claude Desktop configurations to discover the tool and make a valid call.

### Implementation for User Story 5

- [ ] T015 [P] [US5] Add explicit `--component mcp` selection to `scripts/install.sh`, preserving the default CLI component and supporting MCP asset names and executable paths.
- [ ] T016 [P] [US5] Add explicit `-Component mcp` selection to `scripts/install.ps1`, preserving the default CLI component and supporting MCP asset names and executable paths.
- [ ] T017 [US5] Extend `.github/workflows/release.yml` to build and stage the MCP executable for Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64, retaining full-test and benchmark gates before publication.
- [ ] T018 [US5] Document latest-version install/update, pinned-version and manual-fallback MCP installer commands, plus local-stdio registration and vault configuration for GitHub Copilot in VS Code and Claude Desktop in `README.md`.
- [ ] T019 [US5] Validate tool discovery and one valid call in the latest stable GitHub Copilot in VS Code and Claude Desktop configurations, and record the exact client versions and results in `specs/002-mcp-wrapper/quickstart.md`.

**Checkpoint**: MCP assets exist for all five release targets, the existing installer default remains CLI-only, and both supported clients can launch and call the tool.

---

## Phase 7: User Story 3 - Offer discoverable, agent-friendly tool metadata (Priority: P3)

**Goal**: Make the tool’s purpose, arguments, outcomes, and choice between a simple target line and structured emplacement clear to clients.

**Independent Test**: Inspect `tools/list` metadata and schema; verify the description recommends emplacement for precise section/shard reads and a simple target line for a single point target.

### Tests for User Story 3

> Add metadata assertions before changing the published tool description/schema.

- [ ] T020 [P] [US3] Add tool description, argument schema, and result-shape assertions—including emplacement guidance—in `tests/contract/mcp_tool_metadata.rs`.

### Implementation for User Story 3

- [ ] T021 [US3] Define stable MCP tool metadata and schema descriptions for required/optional arguments and resolver outcomes, including the point-target versus emplacement guidance, in `src/mcp.rs`.

**Checkpoint**: A client can discover the tool and choose the appropriate result detail from its metadata without guesswork.

---

## Phase 8: User Story 6 - Inspect MCP behavior when troubleshooting (Priority: P3)

**Goal**: Add opt-in structured stderr diagnostics for lifecycle, request, cache, watcher, and failure events without changing protocol output or exposing request content.

**Independent Test**: Compare subprocess stdout and tool results with diagnostics off/on; with diagnostics enabled, validate JSON-lines events and confirm request text, paths, note contents, serialized arguments, and raw errors are absent.

### Tests for User Story 6

> Write subprocess tests first because the logger is process-global and the stdout/stderr boundary is part of the contract.

- [ ] T022 [P] [US6] Add child-process tests for diagnostics-disabled silence, enabled lifecycle/request/cache events, fixed event/failure categories, redaction, and stdout/result parity in `tests/integration/mcp_diagnostics.rs`.

### Implementation for User Story 6

- [ ] T023 [US6] Add `--diagnostics` handling and initialize `structured-logger` only when enabled, routing the fixed MCP diagnostics target to stderr and all other targets to a sink in `src/bin/obsidian-link-resolver-mcp.rs` and `src/mcp.rs`.
- [ ] T024 [US6] Emit only fixed event names, static messages/targets, allow-listed statuses/failure categories, process-local request sequence numbers, and durations; distinguish resolver `error` results from failed requests and redact all request/path/error content in `src/mcp.rs`.

**Checkpoint**: Optional diagnostics report operational outcomes on stderr only and leave MCP results unchanged; diagnostics remain absent by default.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Validate the complete feature and keep implementation guidance aligned with the code and release pipeline.

- [ ] T025 Update `specs/002-mcp-wrapper/quickstart.md` with the implemented binary/installer commands, release benchmark invocation, and validated end-to-end scenarios while preserving the normative conformance corpus.
- [ ] T026 Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, focused MCP tests, `cargo test`, and the MCP warm-run benchmark defined in `specs/002-mcp-wrapper/quickstart.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 and T002 establish the executable target, required dependencies, and CI/devcontainer coverage.
- **Foundational (Phase 2)**: No separate implementation tasks; the MCP server bootstrap is delivered as part of US1.
- **User Stories (Phases 3-8)**: US1 is the prerequisite for every other story. US2 and US4 build on the working adapter; US5 requires the binary target. US3 and US6 also require US1.
- **Polish (Phase 9)**: Depends on the selected user stories being implemented.

### User Story Dependencies

- **US1 (P1)**: Starts after Setup; no other story dependency.
- **US2 (P2)**: Depends on US1 because parity requires a callable MCP tool.
- **US4 (P2)**: Depends on US1 and US2 so the cache can be verified against the preserved resolver contract.
- **US5 (P2)**: Depends on US1 for a buildable MCP executable. Installer, release, and documentation work uses separate files and can proceed alongside US2/US4 after US1.
- **US3 (P3)**: Depends on US1; metadata implementation shares `src/mcp.rs` with US2, US4, and US6.
- **US6 (P3)**: Depends on US1; diagnostics implementation shares server lifecycle code in `src/mcp.rs` with US3 and cache work.

### Parallel Opportunities

- T001 and T002 establish separate manifest/environment and CI files; complete both before building.
- After US1, US5 installer/release/documentation tasks (T015-T019) can proceed separately from resolver/cache work.
- Within US1, T003 and T004 can be authored in parallel; both precede T005-T006.
- Within US2, T007 is the test-first prerequisite for T008.
- Within US4, T009-T011 can be authored in parallel; T012-T014 depend on those tests and should be coordinated because they modify the resolver/cache boundary.
- T015 and T016 can be implemented in parallel because they modify separate installer files.
- Within US3 and US6, the contract/integration test task precedes implementation; do not parallelize changes to `src/mcp.rs` across stories.

### Parallel Execution Examples

**User Story 1**

```text
In parallel: T003 (tool contract tests) and T004 (stdio integration tests)
Then: T005 (tool handler) and T006 (server startup/stdio transport)
```

**User Story 2**

```text
T007 (resolver parity tests) → T008 (preserve result contract)
```

**User Story 4**

```text
In parallel: T009 (cache unit tests), T010 (refresh integration tests), T011 (warm-run benchmark)
Then: T012 (root/index boundary) → T013-T014 (cache lifecycle and request consistency)
```

**User Story 5**

```text
In parallel: T015 (Bash installer) and T016 (PowerShell installer)
Then: T017 (release matrix); complete T018-T019 for client setup and validation
```

**User Story 3**

```text
T020 (metadata contract tests) → T021 (tool metadata/schema)
```

**User Story 6**

```text
T022 (diagnostics subprocess tests) → T023-T024 (logger configuration and safe event emission)
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Setup (Phase 1).
2. Complete US1 protocol tests and implementation (Phase 3).
3. **Stop and validate** tool discovery, one valid stdio call, and the independent US1 test criterion.
4. Demo the standalone MCP binary without changing the existing CLI.

### Incremental Delivery

1. Deliver US1 as the smallest usable MCP resolver.
2. Add US2 result parity, then US4 cache reuse and refresh.
3. Add US5 installation/release support so users can install the validated server.
4. Add US3 discoverability improvements and US6 opt-in diagnostics.
5. Run the complete quickstart, CI, release-target, and benchmark validation.
