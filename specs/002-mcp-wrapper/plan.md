# Implementation Plan: Obsidian Link Resolver MCP Wrapper

**Branch**: `002-mcp-wrapper` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-mcp-wrapper/spec.md`

## Summary

Add a separately installable, long-running MCP server that exposes the existing resolver as one standard tool over stdio. The server delegates every request to the Rust library's existing resolution path and emits the same deterministic `ResolutionTarget` fields and five outcomes; determinism applies while the vault contents are unchanged, while observed filesystem changes may change later results. It accepts a context file, optional vault root, and optional emplacement request; tool metadata explains when richer emplacement is useful. A process-scoped vault cache reuses indexes per canonical root, invalidates on filesystem events, and runs a background full rescan at least every 60 seconds independently of incoming requests. Optional diagnostics use `structured-logger` to report lifecycle, request outcome, failures, and cache activity as structured stderr records, excluding request text, paths, note contents, serialized arguments, and unfiltered error messages. Publish a dedicated native executable alongside the current CLI/FFI assets, retain the existing CLI installer unchanged, provide separate parameterless CLI and MCP installer commands, and document standard client registration. Treat the MCP contract as semantically versioned: preserve argument names/requiredness and result fields/meanings within a major version; ship breaking changes only in a new major version with migration notes. Organize the codebase as three separate crates in a Cargo workspace (`obsidian-link-resolver` library, `obsidian-link-resolver-cli`, and `obsidian-link-resolver-mcp`) sharing a synchronized version and released under a single `vX.Y.Z` tag; every release publishes distinct library/FFI, CLI, and MCP assets at that version. The executable installers install only their respective binaries; library/FFI assets are distributed separately.

## Technical Context

**Language/Version**: Rust 1.98.1, edition 2021, pinned consistently in the devcontainer, CI workflow, and `vX.Y.Z` release workflow.

**Primary Dependencies**: `rmcp` 3.5.1 for MCP server/tool/stdio protocol support (explicitly enable only `server`, `macros`, `schemars`, and `transport-io`); Tokio runtime as required by the SDK and periodic refresh scheduler; `notify` 8.2.0 for cross-platform vault filesystem invalidation; `structured-logger` 1.0.5 with default features disabled for synchronous JSON diagnostics and `log` 0.4 with structured key-value support; existing `serde`/`serde_json`, `clap`, and resolver library. Do not enable HTTP transport.

**Storage**: Read-only local vault filesystem; in-memory cache of vault indexes scoped to the server process.

**Testing**: `cargo test --workspace` with MCP protocol contract tests, resolver parity tests, cache behavior tests, diagnostics default/redaction/channel-isolation tests, and existing unit/integration/CLI/FFI suites; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; and release-tag validation that accepts the shared `vX.Y.Z` tag, runs all workspace release gates, and publishes distinct assets for all three components. Each release runs the full workspace test suite and MCP warm-run benchmark before staging assets; the benchmark enforces a ≤100 ms p50. Record a separate cold-start benchmark p50 as a baseline. Run benchmarks on GitHub-hosted `ubuntu-24.04` and emit/upload machine-readable reports containing runner image/OS/architecture, fixture version, sample count, and p50. Run path-qualified and vault-relative path parity fixtures natively on all five supported release targets. Contract compatibility validation compares published MCP contract versions within a major version and checks that every breaking contract change has a new major version and migration notes.

**Target Platform**: MCP server binaries target `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-gnu`, matching the existing release targets. Use these Rust target triples as the canonical build, test, and asset-mapping identifiers; user-facing prose may use platform architecture names. Supported client registrations are limited to GitHub Copilot in VS Code and Claude Desktop. Windows ARM64 is not currently a supported release target.

**Project Type**: Cargo workspace with three separate crates (`obsidian-link-resolver` library/FFI, `obsidian-link-resolver-cli`, and `obsidian-link-resolver-mcp`) sharing a synchronized version. GitHub Releases remain the distribution channel; no crates.io publication is in scope.

**Performance Goals**: Repeated requests for a vault reuse its index and do not enumerate the vault per call. Filesystem events cause refresh before the next resolution; a background full rescan runs at least every 60 seconds per cached vault, whether or not requests arrive. On GitHub-hosted `ubuntu-24.04`, both pull-request CI and tag-release validation enforce a ≤100 ms end-to-end p50 across 100 consecutive warm-cache calls against the approximately 5,000-note benchmark vault. Exclude process startup and initial index construction from this warm-cache measurement. Separately measure 100 cold starts from process launch through completion of the first MCP resolution against the same corpus, including startup and initial indexing; record its p50 in a machine-readable report as a baseline, without a release-blocking threshold until a budget is established from measured results. Identify the synthetic corpus as `bench-vault-v1`, generated by `tests/fixtures/bench_vault.rs`; bump the fixture version whenever its generation rules or contents change. Emit both reports and retain them as workflow artifacts, recording the runner image/OS/architecture and fixture version alongside sample count and p50.

**Constraints**: Standard MCP stdio messages only on stdout; optional diagnostics on stderr and disabled by default. Diagnostic record fields, event names, failure categories, and redaction rules are defined by the Operational Diagnostics section in `spec.md` and `contracts/mcp.md`. Initialize the logger only when `--diagnostics` is supplied, set an explicit log level, route only the fixed MCP diagnostics target to stderr, and route other log targets to a sink. Disable `structured-logger`'s default `log-panic` feature. Identical requests against unchanged vault contents must produce identical primary result records; changed contents may produce changed results after the freshness policy reflects them. Cache keys use canonical roots; refresh errors fail closed as `error` outcomes rather than serving known-stale index data. Note bodies are read for each resolution, so heading/block edits are observed without rebuilding the name index. The devcontainer, CI workflow, and release workflow must use Rust 1.98.1 consistently; any toolchain update must change all three together. New runtime/build dependencies must be reflected in `.devcontainer/devcontainer.json` and CI/release. Workspace member crates share a synchronized semantic version matching the root workspace / `vX.Y.Z` tag. A `vX.Y.Z` release publishes distinct assets for all three crates after all gates pass. The CLI and MCP server each have a separate installer command that installs only its corresponding executable; neither requires a component-selection parameter, and library/FFI assets are distributed separately.

**Scale/Scope**: One long-lived server process, one cache entry per canonical vault root accessed during that process, and vaults of up to ~5,000 notes within the current performance target.

### Design Decisions

- Split the monolithic repository into three separate crates within a Cargo workspace under `crates/`: `obsidian-link-resolver` (library and C ABI), `obsidian-link-resolver-cli` (CLI binary), and `obsidian-link-resolver-mcp` (MCP server binary). Manage a shared semantic version across all member crates at the workspace level (`[workspace.package]`). Both CLI and MCP crates depend on the library crate.
- Use the official Rust MCP SDK (`rmcp`) and stdio transport; run the server as `obsidian-link-resolver-mcp` from its own crate so the established one-shot CLI argument and output contract remain unchanged.
- Expose one `resolve_obsidian_link` tool. Its required `link` and `context_path` fields and optional `vault_root` and `with_emplacement` fields map directly to existing resolver inputs. A launch-time `--vault` is a default only; an explicit per-call `vault_root` takes precedence, otherwise the resolver's existing auto-detection applies.
- Return all resolver statuses as normal tool results, including unresolved and ambiguous. Invalid tool arguments and protocol/lifecycle failures use MCP errors. Include the result as structured content and serialized JSON text for client compatibility; the result object remains governed by the existing result schema.
- Implement canonical compatibility requirement FR-012 by versioning the MCP contract with the shared Cargo package semantic version. Release validation compares published contract artifacts and verifies migration notes for breaking changes.
- Keep the existing `vX.Y.Z` GitHub release tag and root Cargo package version shared across the library, CLI, and MCP server. Each tagged release passes the full test suite and MCP warm-run benchmark, then publishes distinct library/FFI, CLI, and MCP assets carrying the same version. The CLI and MCP server have separate installer commands supporting latest and pinned versions; each installs only its corresponding executable and takes no component-selection parameter. Publish GitHub release assets only; crates.io publication is out of scope.
- Split vault-root selection from index enumeration as needed so each request can identify and validate its canonical root without forcing a full vault walk. Cache `Vault` snapshots by canonical root for the server process lifetime.
- Keep cache ownership explicit: `cache.rs` owns canonical-root entries, watcher state, dirty flags, index generations, refresh scheduling, and atomic replacement; `mcp.rs` selects the cache generation for each request, invokes the resolver with that generation, and maps resolver results to MCP responses.
- Use filesystem watcher events as per-root dirty hints and coalesce bursts. Each in-flight resolution uses the index generation selected when it starts; if that root is invalidated during the call, the call may complete on its selected generation, but refresh before the next resolution for that root and use the latest state available at refresh time. Run a periodic background full rescan for each cached vault at least every 60 seconds, independent of requests, to bound missed-event staleness. If watcher setup fails, retain the periodic-rescan fallback. A refresh failure returns an error, not a stale resolution.
- Add an opt-in server `--diagnostics` launch flag, disabled by default. When enabled, use `structured-logger`'s synchronous JSON writer for one JSON-lines event per log call to stderr. Initialize with an explicit level and only the MCP diagnostics target routed to stderr; route all other targets to a sink. Use only the fixed fields and allow-listed event/failure values defined in the Operational Diagnostics section of `spec.md` and `contracts/mcp.md`. Never log user-supplied link text, context/vault paths, note contents, serialized arguments, or unfiltered error strings. Use only static messages and targets. Disable the default `log-panic` feature and keep optional diagnostics out of MCP stdout and tool result objects.
- Distribute prebuilt MCP executables for the existing supported release matrix. Keep the existing CLI installer and add separate MCP installer scripts/commands so users do not need to pass a component-selection parameter.
- Keep the Rust toolchain pinned to 1.98.1 in the release workflow, matching the devcontainer and CI; treat any toolchain version change as a coordinated update across all three.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Requirement | Plan compliance |
|---|---|---|
| I. Performance & Agent Efficiency | Low latency and minimal repeated work; measure common-case latency in CI | Native binary; process-scoped per-vault cache; no full enumeration per request; warm-run p50 threshold enforced and cold-start p50 recorded as a baseline in pull-request CI and release on the same `ubuntu-24.04` runner label, with runner and fixture metadata retained in reports. PASS |
| II. CLI Interface & Consistent Output | Deterministic machine interface and stderr diagnostics | MCP stdio is the server's only stdout protocol; logs stay on stderr; results reuse deterministic resolver fields. PASS |
| III. Test-First | Tests precede implementation for public behavior/performance changes | Plan requires MCP schema/protocol, resolver-parity, and cache tests before implementation. PASS |
| IV. Integration Testing | Exercise real caller patterns and output/error behavior | Stdio initialize/list/call, client-like requests, all result outcomes, cache refresh, path-qualified/vault-relative path behavior on all five supported release targets, and release binaries receive integration coverage. PASS |
| V. Observability, Versioning & Simplicity | Small stable surface, semver, quiet defaults | One tool with stable names; explicit within-major contract guarantees and breaking-change migration notes; optional, fully redacted diagnostics on stderr only; separate crates in a workspace sharing a synchronized version released via single `vX.Y.Z` tag with distinct assets, preserving the CLI-only default. PASS |
| VI. Interoperability & Cross-Technology Integration | Preserve language-agnostic surfaces | MCP is an additional standard protocol surface; existing CLI, JSON schema, and C ABI remain unchanged. PASS |
| VII. Reproducible Development Environment | Declare new tools/dependencies in devcontainer | Add Rust dependencies and any required system/toolchain support to `.devcontainer/devcontainer.json`; keep CI aligned. PASS, required implementation task |
| VIII. CI & Release Gating | Full tests gate all release artifacts; CI and release toolchains match the devcontainer | Keep the explicit Rust 1.98.1 pin consistent across devcontainer, CI, and the release workflow. Pull-request CI and every `vX.Y.Z` release run the full workspace test suite (`cargo test --workspace`) and MCP warm-run gate on the same `ubuntu-24.04` runner label; release assets are staged only after all gates pass. Retain the runner/fixture report. PASS, required implementation task |
| IX. Native Documentation for Named Symbols | Rust symbols use Rustdoc | New/modified Rust modules, types, methods, and fields require Rustdoc in the same patch; add API/tool usage docs. PASS |

**Pre-research gate**: PASS. The required watcher dependency and protocol SDK are justified by FR-017/018 and FR-001/012; simpler per-call rebuilding violates the cache requirement, and a hand-rolled protocol would increase compatibility risk. `structured-logger` is justified by the diagnostics requirement for structured events and provides a synchronous JSON writer with explicit target routing; its panic-logging default is disabled to preserve the redaction boundary. Organizing as separate crates in a Cargo workspace with synchronized workspace-level versioning cleanly isolates crate boundaries while maintaining a single release tag and version.

**Post-design gate**: PASS. The design retains existing resolver contracts, adds one narrowly scoped adapter, documents schemas, tests refresh, parity, and diagnostics isolation, and fits the existing test-gated release model. Determinism is explicitly scoped to unchanged vault contents, consistent with cache freshness. Diagnostics are opt-in and exclude request text, paths, note contents, serialized arguments, and unfiltered errors; the logger is configured with a stderr-only target and no panic payload logging. Contract compatibility checks cover published MCP contract versions, major-version boundaries, and migration notes. The workspace member crates share a synchronized version matching `vX.Y.Z`. A `vX.Y.Z` tag runs the full workspace test suite and MCP warm-run benchmark, then publishes the distinct component assets together; a separate cold-start benchmark records startup plus initial-indexing p50 as a baseline without an ungrounded failure threshold. Both reports identify the runner image/OS/architecture and versioned fixture.

## Project Structure

### Documentation

```text
specs/002-mcp-wrapper/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── mcp.md                 # Human-readable MCP protocol contract
│   ├── mcp-contract.json      # Machine-readable versioned compatibility artifact
│   └── releases.md            # Shared release version/tag and component asset boundaries
└── tasks.md                 # Phase 2; not produced by this plan
```

### Source and Validation

```text
Cargo.toml                   # Root workspace manifest defining members and shared workspace version/dependencies
Cargo.lock                   # Workspace lockfile
.devcontainer/devcontainer.json
.github/workflows/ci.yml
.github/workflows/release.yml # Shared vX.Y.Z tag, builds all workspace crates, publishes distinct component assets
scripts/install.sh
scripts/install.ps1
scripts/install-mcp.sh
scripts/install-mcp.ps1
scripts/check-mcp-contract-compatibility.sh # FR-012 compatibility validation; run in CI and before release assets are staged
README.md
crates/
├── obsidian-link-resolver/   # Library crate: resolver API, vault index, C ABI / FFI
│   ├── Cargo.toml
│   ├── build.rs              # cbindgen header generation
│   ├── include/              # generated C header
│   ├── src/
│   │   ├── lib.rs            # library root and public API
│   │   ├── cli.rs            # shared CLI argument/format types used by CLI crate
│   │   ├── ffi.rs            # C ABI functions
│   │   ├── link.rs           # link parsing
│   │   ├── note.rs           # note parsing / emplacement
│   │   ├── output.rs         # resolution target output types
│   │   ├── resolve.rs        # resolution logic
│   │   └── vault.rs          # vault enumeration and indexing
│   └── tests/                # library unit, integration, and FFI tests
├── obsidian-link-resolver-cli/ # CLI binary crate
│   ├── Cargo.toml            # depends on path = "../obsidian-link-resolver"
│   ├── src/
│   │   └── main.rs           # CLI binary entry point (obsidian-link-resolver)
│   └── tests/                # CLI integration and contract tests
└── obsidian-link-resolver-mcp/ # MCP server binary crate
    ├── Cargo.toml            # depends on path = "../obsidian-link-resolver", rmcp, notify, structured-logger
    ├── src/
    │   ├── main.rs           # MCP binary entry point (obsidian-link-resolver-mcp)
    │   ├── mcp.rs            # MCP tool handler, request generation selection, result mapping, and diagnostics
    │   └── cache.rs          # canonical-root cache entries, generations, refresh, and watcher management
    ├── tests/                # MCP protocol contract, parity, cache, and diagnostics tests
    └── benches/
        ├── mcp_warm.rs       # 100-call warm-cache benchmark and report
        └── mcp_cold_start.rs # 100 cold-start benchmark and baseline report
tests/fixtures/bench_vault.rs # versioned synthetic ~5,000-note corpus generator
```

**Structure Decision**: Separate each component into its own crate within a Cargo workspace under `crates/`: `obsidian-link-resolver` (library and C ABI), `obsidian-link-resolver-cli` (CLI binary), and `obsidian-link-resolver-mcp` (MCP server binary). The workspace manages a shared version (`workspace.package.version`) so all crates advance in lockstep under a single `vX.Y.Z` tag. Each crate has clear boundaries and dependencies: CLI and MCP crates depend on the library crate. Tests and benchmarks are co-located with their respective crates. Release CI builds each crate's artifacts independently and publishes distinct assets under the shared tag.

## Complexity Tracking

No constitution violations. The SDK and watcher support standard MCP interoperability and the explicit cache freshness requirement; `structured-logger` provides the structured logging backend required by US6. The 60-second scan is a correctness fallback, not a per-request rebuild. Workspace crate separation provides clean modular boundaries and dependency isolation; synchronized workspace versioning keeps the single `vX.Y.Z` release workflow simple.
