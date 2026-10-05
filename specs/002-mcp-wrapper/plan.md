# Implementation Plan: Obsidian Link Resolver MCP Wrapper

**Branch**: `002-mcp-wrapper` | **Date**: 2026-10-04 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-mcp-wrapper/spec.md`

## Summary

Add a separately installable, long-running MCP server that exposes the existing resolver as one standard tool over stdio. The server delegates every request to the Rust library's existing resolution path and emits the same deterministic `ResolutionTarget` fields and five outcomes. It accepts a context file, optional vault root, and optional emplacement request; tool metadata explains when richer emplacement is useful. A process-scoped vault cache reuses indexes per canonical root, invalidates on filesystem events, and performs a full rescan at least every 60 seconds as a missed-event backstop. Publish a dedicated native executable alongside the current CLI/FFI assets, extend installers without changing their default CLI behavior, and document standard client registration.

## Technical Context

**Language/Version**: Rust 1.98.1, edition 2021 (the toolchain pinned in CI and release workflows)

**Primary Dependencies**: `rmcp` 3.5.0 for MCP server/tool/stdio protocol support; Tokio runtime as required by the SDK; `notify` 8.2.0 for cross-platform vault filesystem invalidation; existing `serde`/`serde_json`, `clap`, and resolver library. Keep SDK features limited to server, tool schema, and stdio needs; do not enable HTTP transport.

**Storage**: Read-only local vault filesystem; in-memory cache of vault indexes scoped to the server process.

**Testing**: `cargo test` with MCP protocol contract tests, resolver parity tests, cache behavior tests, and existing unit/integration/CLI/FFI suites; `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`; release benchmark gate remains ≤100 ms warm-run p50.

**Target Platform**: Local stdio MCP clients on Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64, matching the existing release targets. Windows ARM64 is not currently a supported release target.

**Project Type**: Single Rust crate with the existing CLI, library, and C ABI plus a dedicated MCP server binary.

**Performance Goals**: Repeated requests for a vault reuse its index and do not enumerate the vault per call. Filesystem events cause refresh before the next resolution; a full rescan occurs at least every 60 seconds. Preserve the existing ≤100 ms warm-resolution p50 target on a representative ~5,000-note vault.

**Constraints**: Standard MCP stdio messages only on stdout; diagnostics on stderr. No network listener. Keep resolution, paths, status, and result fields identical to the current resolver. Cache keys use canonical roots; refresh errors fail closed as `error` outcomes rather than serving known-stale index data. Note bodies are read for each resolution, so heading/block edits are observed without rebuilding the name index. New runtime/build dependencies must be reflected in `.devcontainer/devcontainer.json` and CI/release.

**Scale/Scope**: One long-lived server process, one cache entry per canonical vault root accessed during that process, and vaults of up to ~5,000 notes within the current performance target.

### Design Decisions

- Use the official Rust MCP SDK (`rmcp`) and stdio transport; run the server as a separate `obsidian-link-resolver-mcp` binary so the established one-shot CLI argument and output contract remain unchanged.
- Expose one `resolve_obsidian_link` tool. Its required `link` and `context_path` fields and optional `vault_root` and `with_emplacement` fields map directly to existing resolver inputs. A launch-time `--vault` is a default only; an explicit per-call `vault_root` takes precedence, otherwise the resolver's existing auto-detection applies.
- Return all resolver statuses as normal tool results, including unresolved and ambiguous. Invalid tool arguments and protocol/lifecycle failures use MCP errors. Include the result as structured content and serialized JSON text for client compatibility; the result object remains governed by the existing result schema.
- Split vault-root selection from index enumeration as needed so each request can identify and validate its canonical root without forcing a full vault walk. Cache `Vault` snapshots by canonical root for the server process lifetime.
- Use filesystem watcher events as dirty hints, coalesce bursts, and rebuild before the next resolution. Perform a periodic full rescan every 60 seconds even with a watcher active to bound missed-event staleness. If watcher setup fails, retain the periodic-rescan fallback. A refresh failure returns an error, not a stale resolution.
- Distribute prebuilt MCP executables for the existing supported release matrix. Add an explicit installer component selection while keeping the current default as CLI-only.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Requirement | Plan compliance |
|---|---|---|
| I. Performance & Agent Efficiency | Low latency and minimal repeated work | Native binary; process-scoped per-vault cache; no full enumeration per request; existing warm-run gate retained. PASS |
| II. CLI Interface & Consistent Output | Deterministic machine interface and stderr diagnostics | MCP stdio is the server's only stdout protocol; logs stay on stderr; results reuse deterministic resolver fields. PASS |
| III. Test-First | Tests precede implementation for public behavior/performance changes | Plan requires MCP schema/protocol, resolver-parity, and cache tests before implementation. PASS |
| IV. Integration Testing | Exercise real caller patterns and output/error behavior | Stdio initialize/list/call, client-like requests, all result outcomes, cache refresh, and release binaries receive integration coverage. PASS |
| V. Observability, Versioning & Simplicity | Small stable surface, semver, quiet defaults | One tool with stable names; no normal stdout diagnostics; separate binary avoids changing CLI defaults. PASS |
| VI. Interoperability & Cross-Technology Integration | Preserve language-agnostic surfaces | MCP is an additional standard protocol surface; existing CLI, JSON schema, and C ABI remain unchanged. PASS |
| VII. Reproducible Development Environment | Declare new tools/dependencies in devcontainer | Add Rust dependencies and any required system/toolchain support to `.devcontainer/devcontainer.json`; keep CI aligned. PASS, required implementation task |
| VIII. CI & Release Gating | Full tests gate all release artifacts | Existing tag workflow runs tests and benchmark before staging assets; stage MCP binary only after those gates pass for each current target. PASS, required implementation task |
| IX. Native Documentation for Named Symbols | Rust symbols use Rustdoc | New/modified Rust modules, types, methods, and fields require Rustdoc in the same patch; add API/tool usage docs. PASS |

**Pre-research gate**: PASS. The required watcher dependency and protocol SDK are justified by FR-017/018 and FR-001/012; simpler per-call rebuilding violates the cache requirement, and a hand-rolled protocol would increase compatibility risk.

**Post-design gate**: PASS. The design retains existing resolver contracts, adds one narrowly scoped adapter, documents schemas, tests refresh and parity, and fits the existing test-gated release model. New dependencies and release assets are explicitly assigned to devcontainer/CI/release work.

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
├── mcp.rs                   # MCP tool adapter and process-scoped cache
└── bin/
    └── obsidian-link-resolver-mcp.rs
tests/
├── contract/                # tool schema, outcomes, and protocol contract
├── integration/             # stdio lifecycle and resolver parity
└── unit/                    # cache invalidation and refresh behavior
```

**Structure Decision**: Keep the resolver in the existing library crate and add a thin, separately invoked MCP adapter under `src/mcp.rs` plus a `src/bin/` entry point. The adapter reuses `resolve_with_vault`; only vault root/index lifecycle belongs in the cache. Tests extend the existing contract, integration, and unit test directories. No second project or network service is introduced.

## Complexity Tracking

No constitution violations. The SDK and watcher are the minimum planned dependencies for standard MCP interoperability and the explicit cache freshness requirement. The 60-second scan is a correctness fallback, not a per-request rebuild.
