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

**Purpose**: Add MCP dependencies; declare the dedicated executable target when its entry point is implemented in T007.

- [X] T001 Bootstrap the root package with `rmcp` 3.5.1 using only `server`, `macros`, `schemars`, and `transport-io` features; Tokio, `notify` 8.2.0, `structured-logger` 1.0.5 with default features disabled, and structured `log` support; refresh `Cargo.lock`. Defer the `obsidian-link-resolver-mcp` binary declaration until T007 creates its entry point. T003 subsequently migrates these declarations into their owning workspace crates and regenerates the final workspace lockfile.
- [X] T002 Verify Rust 1.98.1 is consistent across the `.devcontainer/devcontainer.json` image, `.github/workflows/ci.yml`, and `.github/workflows/release.yml`; verify the devcontainer declares every system tool and toolchain component required to build, test, and run all workspace crates and supported release targets, adding any missing requirements introduced by the MCP dependencies; align the configured Windows target with the canonical `x86_64-pc-windows-gnu` release target and install its MinGW linker in the devcontainer, retaining support for the other release targets there. Keep CI's existing formatting and Clippy checks; add MCP-specific CI build/test checks only after the binary source is created.

## Phase 2: Foundational

**Purpose**: Establish prerequisites shared by the stories.

- [ ] T003 Migrate the root Cargo package into the three-member workspace defined in `specs/002-mcp-wrapper/plan.md`: move the resolver library/FFI and its tests/build script into `crates/obsidian-link-resolver/`, the existing CLI and its tests into `crates/obsidian-link-resolver-cli/`, and the MCP server into `crates/obsidian-link-resolver-mcp/`; declare all three workspace members and `workspace.package.version` in the root `Cargo.toml`, set each member manifest to inherit `version.workspace = true`, move dependencies to their owning crate manifests, preserve existing targets, and refresh the root `Cargo.lock`.

**Checkpoint**: Complete Phase 1 and T003 before story implementation. All three crates build from the root workspace and share the root workspace version.

## Phase 3: User Story 1 - Resolve links through a standard tool interface (Priority: P1) 🎯 MVP

**Goal**: Provide a local MCP stdio server with a `resolve_obsidian_link` tool that accepts a link and context path and returns the existing resolver result.

**Independent Test**: Start the binary, initialize an MCP session, list the tool, and resolve a known fixture link; verify structured content contains the expected target path and location.

### Tests for User Story 1

> Write these protocol tests first and verify they fail before implementing the tool.

- [ ] T004 [P] [US1] Add MCP tool discovery and input-schema contract tests for required `link` and `context_path`, optional `vault_root` and `with_emplacement`, and missing required or wrong-type required/optional arguments being rejected as MCP tool/protocol errors in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_tool.rs`.
- [ ] T005 [P] [US1] Add child-process stdio integration tests for MCP initialize, tool listing, a valid fixture resolution, and missing required or wrong-type required/optional argument calls returning MCP errors without resolver result content in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_stdio.rs`.

### Implementation for User Story 1

- [ ] T006 [US1] Implement the typed `resolve_obsidian_link` request handler and return the resolver record as both structured content and compact JSON text in `crates/obsidian-link-resolver-mcp/src/mcp.rs`, documenting new Rust symbols with Rustdoc.
- [ ] T007 [US1] Implement the `obsidian-link-resolver-mcp` startup path, optional `--vault` default, module registration, and stdio transport in `crates/obsidian-link-resolver-mcp/src/main.rs`, documenting new Rust symbols with Rustdoc.

**Checkpoint**: The MCP server launches for a client and resolves a fixture link without changing the existing CLI.

## Phase 4: User Story 2 - Preserve the current resolver contract (Priority: P2)

**Goal**: Preserve resolver statuses, reasons, candidate ordering, path semantics, deterministic result fields, and emplacement through the MCP adapter.

**Independent Test**: For identical fixture requests against unchanged vault contents, compare MCP and direct resolver results and assert matching status, target path/range, reasons, candidates, emplacement, and primary record; verify published MCP contract compatibility according to FR-012 and SC-012.

### Tests for User Story 2

> Write parity and determinism tests first and verify they fail before changing result mapping.

- [ ] T008 [P] [US2] Add contract tests that submit identical requests repeatedly against unchanged fixture contents and assert identical primary result records with no nondeterministic fields in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_determinism.rs`.
- [ ] T009 [US2] Add resolver-parity integration cases in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_resolver_parity.rs` for resolved, unresolved, missing sub-target, ambiguous, and resolver error outcomes; schema-valid link/context/vault values rejected by the resolver; path-qualified references; vault-relative context paths; same-file, heading, block, attachment, and emplacement results. For every case, compare the resolver-result object from MCP structured content (and its JSON-text representation) with the direct resolver result, asserting parity for every field and value, including status, reason, candidates, target/location, and emplacement fields when applicable, and asserting absent optional fields remain absent and no wrapper-only fields are added. Separately assert and report that cases cover all five resolver outcome categories (5/5). Keep MCP protocol errors distinct from normal resolver `error` results, which must retain their reason fields.
- [ ] T010 [P] [US2] Add MCP contract compatibility tests in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_compatibility.rs` for canonical requirement FR-012 and success criterion SC-012: preserve argument names/requiredness, result fields/meanings, and outcome semantics within a major version; allow additive optional fields and arguments; reject breaking changes unless the candidate uses a new major version with migration notes.

### Implementation for User Story 2

- [ ] T011 [US2] Define a machine-readable MCP contract artifact in `specs/002-mcp-wrapper/contracts/mcp-contract.json` and implement compatibility validation in `scripts/check-mcp-contract-compatibility.sh` as the implementation of canonical requirement FR-012; compare the candidate with the latest published contract in the same major version, establish the first published contract as the baseline, and require a new major version plus migration notes for breaking changes.
- [ ] T012 [US2] Preserve all `ResolutionTarget` fields and five resolver outcome categories without wrapper-only result fields, keeping resolver `error` outcomes as normal tool results rather than MCP errors, in `crates/obsidian-link-resolver-mcp/src/mcp.rs` using existing resolver APIs and `specs/001-link-resolver/contracts/result.schema.json`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: Each conformance-corpus request produces the same resolver outcome through MCP as through the existing resolver.

## Phase 5: User Story 4 - Reuse a cached vault index (Priority: P2)

**Goal**: Reuse a per-canonical-root index, refresh it on detected changes and at least every 60 seconds, and never serve a known-stale index after refresh failure.

**Independent Test**: Verify same-root reuse without a per-call directory walk, refresh after invalidation or a simulated missed event, root isolation, and generation consistency when invalidation occurs during a call.

### Tests for User Story 4

> Add cache behavior and benchmark coverage first; use deterministic invalidation and a controllable clock rather than wall-clock sleeps.

- [ ] T013 [P] [US4] Add cache unit tests for canonical-root reuse, dirty-state coalescing, separate roots, atomic refresh, failed-refresh behavior, in-flight generation consistency, and changed note contents becoming visible after refresh in `crates/obsidian-link-resolver-mcp/tests/unit/mcp_cache.rs`.
- [ ] T014 [P] [US4] Add integration tests for watcher invalidation, watcher unavailability, and request-independent periodic refresh within 60 seconds in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_cache_refresh.rs`.
- [ ] T015 [P] [US4] Add a warm-cache benchmark for 100 consecutive end-to-end MCP calls against the approximately 5,000-note corpus in `crates/obsidian-link-resolver-mcp/benches/mcp_warm.rs` and `crates/obsidian-link-resolver-mcp/Cargo.toml`; identify the generated corpus as `bench-vault-v1` from `tests/fixtures/bench_vault.rs`, calculate p50 excluding process startup and initial indexing, and emit a machine-readable report containing runner image, OS, architecture, fixture version, sample count, and p50; enforce p50 ≤100 ms on GitHub-hosted `ubuntu-24.04`.

### Implementation for User Story 4

- [ ] T016 [US4] Separate canonical root selection/validation from vault index enumeration while preserving `detect_root` behavior in `crates/obsidian-link-resolver/src/vault.rs`; document new or modified Rust symbols with Rustdoc.
- [ ] T017 [US4] Implement the process-scoped cache keyed by canonical root, watcher dirty hints, coalesced invalidation, atomic index replacement, and a background full rescan no later than 60 seconds in `crates/obsidian-link-resolver-mcp/src/cache.rs`; document new Rust symbols with Rustdoc.
- [ ] T018 [US4] Ensure each request resolves against the index generation selected at its start, the next request refreshes a root invalidated in flight, and refresh failures return an error rather than stale resolution in `crates/obsidian-link-resolver-mcp/src/mcp.rs`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: Repeated resolutions reuse indexes, detected changes refresh before the next call, missed events recover within 60 seconds, and failed refreshes do not produce stale successes.

## Phase 6: User Story 5 - Install the MCP server in supported clients (Priority: P2)

**Goal**: Publish MCP binaries for the five supported targets and provide separate MCP installer commands and local-stdio client setup without changing the CLI-only installer default.

**Independent Test**: On each supported installer platform, verify the existing CLI installer installs only the CLI and the separate MCP installer installs only the MCP executable without a component parameter; verify release mapping includes all five target assets and both documented clients can discover and call the tool.

### Tests for User Story 5

> Add installer and release-matrix tests before changing the installer or release workflow.

- [ ] T019 [P] [US5] Add mocked installer integration tests for Bash and PowerShell in `crates/obsidian-link-resolver-mcp/tests/integration/installers.rs`, covering the existing CLI installer and dedicated MCP installer scripts separately, latest and pinned-version URLs, supported asset names, and installed executable paths; verify neither installer accepts or requires a component-selection parameter.
- [ ] T020 [P] [US5] Add release-matrix contract tests that verify MCP artifact mapping for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`; verify each shared-version release publishes distinct library/FFI, CLI, and MCP assets from their respective crates only after the full-test, warm-benchmark, SC-014 cold-start baseline, and FR-012 contract-compatibility gates; and verify inclusion of the machine-readable MCP contract artifact in release assets in `crates/obsidian-link-resolver-mcp/tests/integration/release_assets.rs`.

### Implementation for User Story 5

- [ ] T021 [P] [US5] Add a dedicated parameterless Bash MCP installer at `scripts/install-mcp.sh`, leaving `scripts/install.sh` as the CLI-only installer and supporting MCP asset names, latest/pinned versions, and executable paths.
- [ ] T022 [P] [US5] Add a dedicated parameterless PowerShell MCP installer at `scripts/install-mcp.ps1`, leaving `scripts/install.ps1` as the CLI-only installer and supporting MCP asset names, latest/pinned versions, and executable paths.
- [ ] T023 [US5] Extend `.github/workflows/release.yml` to build and stage distinct library/FFI, CLI, and MCP assets from their respective crates under the shared `vX.Y.Z` version, including the MCP executable for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`; on GitHub-hosted `ubuntu-24.04`, run the full workspace test suite, the T015 ≤100 ms p50 benchmark gate, the T035/SC-014 cold-start baseline, and MCP contract compatibility validation under FR-012 against the latest published same-major contract before staging or publishing any component assets; retain both benchmark reports as workflow artifacts and publish the current contract artifact with the release assets.
- [ ] T024 [US5] Document latest-version install/update, pinned-version and manual-fallback MCP installer commands, plus local-stdio registration and vault configuration for GitHub Copilot in VS Code and Claude Desktop in `README.md`.
- [ ] T025 [US5] Validate tool discovery and one valid call against the latest stable GitHub Copilot in VS Code and Claude Desktop releases available when validation is performed; record each exact client version, validation date, operating system, and result in the Client Validation Record in `specs/002-mcp-wrapper/quickstart.md`. A later client release requires a new validation record to claim support for that release.

**Checkpoint**: MCP assets exist for all five release targets; the existing CLI installers remain CLI-only; dedicated parameterless MCP installer commands install only the MCP server; and both supported clients can launch and call the tool.

## Phase 7: User Story 3 - Offer discoverable, agent-friendly tool metadata (Priority: P3)

**Goal**: Make the tool's purpose, arguments, outcomes, and choice between a simple target line and structured emplacement clear to clients.

**Independent Test**: Inspect `tools/list` metadata and schema; verify the description recommends emplacement for precise section/shard reads and a simple target line for a single point target.

### Tests for User Story 3

> Add metadata assertions before changing the published tool description/schema.

- [ ] T026 [P] [US3] Add tool description, argument schema, and result-shape assertions—including emplacement guidance—in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_tool_metadata.rs`.

### Implementation for User Story 3

- [ ] T027 [US3] Define stable MCP tool metadata and schema descriptions for required/optional arguments and resolver outcomes, including point-target versus emplacement guidance, in `crates/obsidian-link-resolver-mcp/src/mcp.rs`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: A client can discover the tool and choose appropriate result detail from its metadata without guesswork.

## Phase 8: User Story 6 - Inspect MCP behavior when troubleshooting (Priority: P3)

**Goal**: Add opt-in structured stderr diagnostics for lifecycle, request, cache, watcher, and failure events without changing protocol output or exposing request content.

**Independent Test**: Compare subprocess stdout and tool results with diagnostics off/on; with diagnostics enabled, validate JSON-lines events and confirm request text, paths, note contents, serialized arguments, and raw errors are absent.

### Tests for User Story 6

> Write subprocess tests first because the logger is process-global and the stdout/stderr boundary is part of the contract.

- [ ] T028 [P] [US6] Add child-process tests for diagnostics-disabled silence, enabled lifecycle/request/cache events, the exact event and failure-kind allow-lists in the Operational Diagnostics section of `spec.md` and `contracts/mcp.md`, and stdout/result parity in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_diagnostics.rs`; include unique sentinel values in request text, context/vault paths, note contents, serialized arguments, and raw errors and assert none appear in stderr.

### Implementation for User Story 6

- [ ] T029 [US6] Add `--diagnostics` handling and initialize `structured-logger` only when enabled, routing the fixed MCP diagnostics target to stderr and all other targets to a sink in `crates/obsidian-link-resolver-mcp/src/main.rs` and `crates/obsidian-link-resolver-mcp/src/mcp.rs`; document new or modified Rust symbols with Rustdoc.
- [ ] T030 [US6] Implement exactly the diagnostic fields, event names, and failure-kind allow-lists defined in the Operational Diagnostics section of `spec.md` and `contracts/mcp.md`; use fixed messages/targets, distinguish resolver `error` results from failed requests, and redact request text, paths, note contents, serialized arguments, and unfiltered error messages in `crates/obsidian-link-resolver-mcp/src/mcp.rs`; document new or modified Rust symbols with Rustdoc.

**Checkpoint**: Optional diagnostics report operational outcomes on stderr only, leave MCP results unchanged, and remain absent by default.

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Validate the complete feature and keep implementation guidance aligned with the code and release pipeline.

- [ ] T031 Update `specs/002-mcp-wrapper/quickstart.md` with implemented binary/installer commands, release benchmark invocation, FR-012 contract compatibility validation and published baseline behavior, Client Validation Record results, and validated end-to-end scenarios while preserving the normative conformance corpus.
- [ ] T032 Audit every new or modified Rust module, type, enum and variant, function, method, member, parameter, return value, and constant in `crates/obsidian-link-resolver/src/`, `crates/obsidian-link-resolver-cli/src/`, and `crates/obsidian-link-resolver-mcp/src/`; add native Rustdoc explaining purpose, constraints, semantics, and side effects, then verify documentation builds with `cargo doc --workspace --no-deps`.
- [ ] T035 Implement SC-014 by adding a cold-start benchmark for 100 fresh server processes, timing process launch through completion of the first MCP resolution against the approximately 5,000-note `bench-vault-v1` corpus, including startup and initial index construction, in `crates/obsidian-link-resolver-mcp/benches/mcp_cold_start.rs` and `crates/obsidian-link-resolver-mcp/Cargo.toml`. Depend on T007 for the executable and T015 for the shared benchmark fixture/report metadata. Emit a machine-readable report with runner image, OS, architecture, fixture version, sample count, and p50; record this as a CI/release baseline without a failure threshold until a latency budget is established. T033 and T023 must wait for this task before integrating the cold-start report into workflows.
- [ ] T033 After T007 creates the MCP binary, T009 adds path-parity tests, T015 defines the warm benchmark target/report format, T019 adds installer tests, and T035 defines the cold-start benchmark/report format, update `.github/workflows/ci.yml` to build/test the MCP binary, run MCP resolver-parity and installer tests on native GitHub-hosted runners for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`, run `scripts/check-mcp-contract-compatibility.sh`, and run both benchmarks on GitHub-hosted `ubuntu-24.04`; fail the warm benchmark when p50 exceeds 100 ms and upload both machine-readable reports as workflow artifacts.
- [ ] T034 After T033 and T035 are complete, run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, focused MCP tests including determinism with unchanged vault contents and FR-012 contract compatibility, installer/release-matrix tests, `cargo test --workspace`, and both MCP benchmarks; verify CI path tests cover `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`; verify both CI and release use GitHub-hosted `ubuntu-24.04`, enforce the warm p50 ≤100 ms threshold, and retain warm and cold reports with runner and `bench-vault-v1` metadata. Include both measured p50 values, sample counts, runner, fixture version, and report links in the PR benchmark summary.

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 establishes the MCP target and dependencies; T002 aligns the development environment and workflow toolchains.
- **Foundational (Phase 2)**: T003 migrates the repository to the planned three-crate workspace and establishes shared package versioning before story implementation.
- **User Stories (Phases 3-8)**: All stories depend on Phase 2's workspace migration. US1 is the prerequisite for every other story. US2 and US4 build on the working adapter; US5 requires the MCP executable. US3 and US6 also require US1.
- **Polish (Phase 9)**: Depends on completion of the selected user stories. T033 depends on T007, T009, T015, T019, and T035 so CI runs only after the binary, path-parity tests, both benchmark targets/reports, and installer tests exist. T023 also depends on T035 before adding the cold-start baseline to release validation. T034 runs after T033 and T035 and verifies the completed benchmarks and both workflows.

### User Story Dependencies

- **US1 (P1)**: Starts after Setup and foundational task T003; no other story dependency.
- **US2 (P2)**: Depends on US1 because parity, determinism, and published-contract compatibility require a callable MCP tool and a defined MCP contract artifact.
- **US4 (P2)**: Depends on US1 and US2 so the cache is verified against the preserved resolver contract.
- **US5 (P2)**: Depends on US1 for a buildable MCP executable. Installer/release/documentation work can proceed separately from cache work once asset naming is established.
- **US3 (P3)**: Depends on US1; metadata implementation shares `crates/obsidian-link-resolver-mcp/src/mcp.rs` with US2, US4, and US6.
- **US6 (P3)**: Depends on US1; diagnostics implementation shares server lifecycle code in `crates/obsidian-link-resolver-mcp/src/mcp.rs` with US3 and cache work.

### Parallel Opportunities

- T004 and T005 can be authored in parallel after T003; both precede T006-T007.
- T008, T009, and T010 can be authored in parallel; T011-T012 follow their tests, with T012 also depending on the US1 adapter.
- T013-T015 can be authored in parallel; T016-T018 depend on the tests and should be coordinated because they modify the resolver/cache boundary.
- T019-T020 can be authored in parallel before T021-T023; T021 and T022 can then be implemented in parallel because they create separate Bash and PowerShell MCP installer scripts.
- T023 and T033 update separate workflow files and may proceed in parallel after their prerequisites; T034 validates both completed workflows.
- T035 follows T007 and T015; T023 and T033 integrate both benchmarks after the cold-start target and report format are defined; T034 runs only after T033 and T035.
- T026 and T028 can be authored separately after US1; coordinate implementation changes to `crates/obsidian-link-resolver-mcp/src/mcp.rs` across US2, US3, US4, and US6.
- T032 is the final explicit constitution audit for native Rustdoc; Rustdoc should also be written alongside every implementation task that adds or modifies symbols.

### Parallel Execution Examples

**User Story 1**

```text
After T003, in parallel: T004 (tool contract tests) and T005 (stdio integration tests)
Then: T006 (tool handler) and T007 (server startup/stdio transport)
```

**User Story 4**

```text
In parallel: T013 (cache unit tests), T014 (refresh integration tests), T015 (warm-run benchmark)
Then: T016 (root/index boundary) → T017-T018 (cache lifecycle and request consistency)
```

**User Story 5**

```text
In parallel: T019 (installer tests) and T020 (release-matrix tests)
Then: T021/T022 (separate parameterless MCP installers); T023 (release matrix, test, benchmark, and compatibility gates); T024-T025 (docs and client validation)
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Setup (Phase 1) and the workspace migration (Phase 2).
2. Complete US1 protocol tests and implementation (Phase 3).
3. Stop and validate tool discovery, one valid stdio call, and the independent US1 test criterion.
4. Demo the standalone MCP binary without changing the existing CLI.

### Incremental Delivery

1. Deliver US1 as the smallest usable MCP resolver.
2. Add US2 result parity/determinism, then US4 cache reuse and refresh.
3. Add US5 installation/release support and validate both supported clients.
4. Add US3 discoverability improvements and US6 opt-in diagnostics.
5. Run complete quickstart, CI, release-target, documentation, and benchmark validation; verify the CI and release benchmark gates use the same runner label and report the same fixture version.
