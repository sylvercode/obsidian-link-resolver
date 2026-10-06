# Implementation Plan: Obsidian Link Resolver MCP Wrapper

**Branch**: `002-mcp-wrapper` | **Date**: 2026-10-04 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-mcp-wrapper/spec.md`

## Summary

Add a separately installable, long-running MCP server that exposes the existing resolver as one standard tool over stdio. The server delegates every request to the Rust library's existing resolution path and emits the same deterministic `ResolutionTarget` fields and five outcomes. It accepts a context file, optional vault root, and optional emplacement request; tool metadata explains when richer emplacement is useful. A process-scoped vault cache reuses indexes per canonical root, invalidates on filesystem events, and runs a background full rescan at least every 60 seconds independently of incoming requests. Optional diagnostics use `structured-logger` to report lifecycle, request outcome, failures, and cache activity as structured stderr records, without logging request contents or changing MCP output. Publish a dedicated native executable alongside the current CLI/FFI assets, extend installers without changing their default CLI behavior, and document standard client registration.

## Technical Context

**Language/Version**: Rust 1.98.1, edition 2021, pinned consistently in the devcontainer, CI workflow, and release workflow.

**Primary Dependencies**: `rmcp` 3.5.0 for MCP server/tool/stdio protocol support (minimal features: `server`, `macros`, `schemars`, `transport-io`); Tokio runtime as required by the SDK and periodic refresh scheduler; `notify` 8.2.0 for cross-platform vault filesystem invalidation; `structured-logger` 1.0.5 with default features disabled for synchronous JSON diagnostics and `log` 0.4 with structured key-value support; existing `serde`/`serde_json`, `clap`, and resolver library. Do not enable HTTP transport.

**Storage**: Read-only local vault filesystem; in-memory cache of vault indexes scoped to the server process.

**Testing**: `cargo test` with MCP protocol contract tests, resolver parity tests, cache behavior tests, diagnostics default/redaction/channel-isolation tests, and existing unit/integration/CLI/FFI suites; `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`; release benchmark gate remains ≤100 ms warm-run p50.

**Target Platform**: MCP server binaries target Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64, matching the existing release targets. Supported client registrations are limited to GitHub Copilot in VS Code and Claude Desktop. Windows ARM64 is not currently a supported release target.

**Project Type**: Single Rust crate with the existing CLI, library, and C ABI plus a dedicated MCP server binary.

**Performance Goals**: Repeated requests for a vault reuse its index and do not enumerate the vault per call. Filesystem events cause refresh before the next resolution; a background full rescan runs at least every 60 seconds per cached vault, whether or not requests arrive. On the designated release benchmark runner, preserve a ≤100 ms end-to-end p50 across 100 consecutive warm-cache calls against the approximately 5,000-note benchmark vault; exclude process startup and initial index construction and record the runner and fixture version.

**Constraints**: Standard MCP stdio messages only on stdout; optional diagnostics on stderr and disabled by default. Initialize the logger only when `--diagnostics` is supplied, set an explicit log level, route only the fixed MCP diagnostics target to stderr, and route other log targets to a sink. Disable `structured-logger`'s default `log-panic` feature. Diagnostic fields and messages must be fixed/allow-listed and must not contain link text, context/vault paths, note contents, serialized arguments, or raw error messages. Cache keys use canonical roots; refresh errors fail closed as `error` outcomes rather than serving known-stale index data. Note bodies are read for each resolution, so heading/block edits are observed without rebuilding the name index. The devcontainer, CI workflow, and release workflow must use Rust 1.98.1 consistently; any toolchain update must change all three together. New runtime/build dependencies must be reflected in `.devcontainer/devcontainer.json` and CI/release.

**Scale/Scope**: One long-lived server process, one cache entry per canonical vault root accessed during that process, and vaults of up to ~5,000 notes within the current performance target.

### Design Decisions

- Use the official Rust MCP SDK (`rmcp`) and stdio transport; run the server as a separate `obsidian-link-resolver-mcp` binary so the established one-shot CLI argument and output contract remain unchanged.
- Expose one `resolve_obsidian_link` tool. Its required `link` and `context_path` fields and optional `vault_root` and `with_emplacement` fields map directly to existing resolver inputs. A launch-time `--vault` is a default only; an explicit per-call `vault_root` takes precedence, otherwise the resolver's existing auto-detection applies.
- Return all resolver statuses as normal tool results, including unresolved and ambiguous. Invalid tool arguments and protocol/lifecycle failures use MCP errors. Include the result as structured content and serialized JSON text for client compatibility; the result object remains governed by the existing result schema.
- Split vault-root selection from index enumeration as needed so each request can identify and validate its canonical root without forcing a full vault walk. Cache `Vault` snapshots by canonical root for the server process lifetime.
- Use filesystem watcher events as per-root dirty hints and coalesce bursts. Each in-flight resolution uses the index generation selected when it starts; if that root is invalidated during the call, the call may complete on its selected generation, but refresh before the next resolution for that root and use the latest state available at refresh time. Run a periodic background full rescan for each cached vault at least every 60 seconds, independent of requests, to bound missed-event staleness. If watcher setup fails, retain the periodic-rescan fallback. A refresh failure returns an error, not a stale resolution.
- Add an opt-in server `--diagnostics` launch flag, disabled by default. When enabled, use `structured-logger`'s synchronous JSON writer for one JSON-lines event per log call to stderr. Initialize with an explicit level and only the MCP diagnostics target routed to stderr; route all other targets to a sink. Events cover server lifecycle, request start/completion/outcome, categorized request/server failures, and cache hit/miss/refresh/watcher outcomes. Structured key-values may include a process-local request sequence, resolver status/error category, and elapsed duration, but never user-supplied link text, context/vault paths, note contents, or unfiltered error strings. Use only static messages and targets. Disable the default `log-panic` feature and keep optional diagnostics out of MCP stdout and tool result objects.
- Distribute prebuilt MCP executables for the existing supported release matrix. Add an explicit installer component selection while keeping the current default as CLI-only.
- Keep the Rust toolchain pinned to 1.98.1 in the release workflow, matching the devcontainer and CI; treat any toolchain version change as a coordinated update across all three.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Requirement | Plan compliance |
|---|---|---|
| I. Performance & Agent Efficiency | Low latency and minimal repeated work | Native binary; process-scoped per-vault cache; no full enumeration per request; existing warm-run gate retained. PASS |
| II. CLI Interface & Consistent Output | Deterministic machine interface and stderr diagnostics | MCP stdio is the server's only stdout protocol; logs stay on stderr; results reuse deterministic resolver fields. PASS |
| III. Test-First | Tests precede implementation for public behavior/performance changes | Plan requires MCP schema/protocol, resolver-parity, and cache tests before implementation. PASS |
| IV. Integration Testing | Exercise real caller patterns and output/error behavior | Stdio initialize/list/call, client-like requests, all result outcomes, cache refresh, and release binaries receive integration coverage. PASS |
| V. Observability, Versioning & Simplicity | Small stable surface, semver, quiet defaults | One tool with stable names; optional, redacted diagnostics on stderr only; separate binary avoids changing CLI defaults. PASS |
| VI. Interoperability & Cross-Technology Integration | Preserve language-agnostic surfaces | MCP is an additional standard protocol surface; existing CLI, JSON schema, and C ABI remain unchanged. PASS |
| VII. Reproducible Development Environment | Declare new tools/dependencies in devcontainer | Add Rust dependencies and any required system/toolchain support to `.devcontainer/devcontainer.json`; keep CI aligned. PASS, required implementation task |
| VIII. CI & Release Gating | Full tests gate all release artifacts; CI and release toolchains match the devcontainer | Keep the explicit Rust 1.98.1 pin consistent across devcontainer, CI, and release workflows; run tests and benchmark before staging assets. PASS, required implementation task |
| IX. Native Documentation for Named Symbols | Rust symbols use Rustdoc | New/modified Rust modules, types, methods, and fields require Rustdoc in the same patch; add API/tool usage docs. PASS |

**Pre-research gate**: PASS. The required watcher dependency and protocol SDK are justified by FR-017/018 and FR-001/012; simpler per-call rebuilding violates the cache requirement, and a hand-rolled protocol would increase compatibility risk. `structured-logger` is justified by the diagnostics requirement for structured events and provides a synchronous JSON writer with explicit target routing; its panic-logging default is disabled to preserve the redaction boundary.

**Post-design gate**: PASS. The design retains existing resolver contracts, adds one narrowly scoped adapter, documents schemas, tests refresh, parity, and diagnostics isolation, and fits the existing test-gated release model. Diagnostics are opt-in and redacted; the logger is configured with a stderr-only target and no panic payload logging. The new crate is assigned to dependency, devcontainer, and CI/release work.

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
│   └── mcp.md
└── tasks.md                 # Phase 2; not produced by this plan
```

### Source and Validation

```text
Cargo.toml                   # rmcp/notify deps and MCP binary target
.devcontainer/devcontainer.json
.github/workflows/ci.yml
.github/workflows/release.yml
scripts/install.sh
scripts/install.ps1
README.md
src/
├── lib.rs                   # shared resolution API
├── vault.rs                 # root selection/index enumeration boundary
├── mcp.rs                   # MCP tool adapter, process-scoped cache, and diagnostics
└── bin/
    └── obsidian-link-resolver-mcp.rs
tests/
├── contract/                # tool schema, outcomes, and protocol contract
├── integration/             # stdio lifecycle and resolver parity
└── unit/                    # cache invalidation and refresh behavior
```

**Structure Decision**: Keep the resolver in the existing library crate and add a thin, separately invoked MCP adapter under `src/mcp.rs` plus a `src/bin/` entry point. The adapter reuses `resolve_with_vault`; only vault root/index lifecycle belongs in the cache. Keep diagnostic event creation alongside MCP lifecycle/request/cache code and test it with a captured writer. Tests extend the existing contract, integration, and unit test directories. No second project or network service is introduced.

## Complexity Tracking

No constitution violations. The SDK and watcher support standard MCP interoperability and the explicit cache freshness requirement; `structured-logger` provides the structured logging backend required by US6. The 60-second scan is a correctness fallback, not a per-request rebuild.
