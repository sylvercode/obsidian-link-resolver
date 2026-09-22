# Implementation Plan: Obsidian Link Resolver CLI

**Branch**: `001-link-resolver` | **Date**: 2026-09-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-link-resolver/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

A read-only command-line tool that resolves Obsidian-style links against the vault and a containing context note, returning the target file and, when applicable, the exact 1-based target line for a heading or block. The design is aligned to the Obsidian reference taxonomy in [obsidian-reference.md](obsidian-reference.md): it accepts supported notes and attachments per OR1–OR5, parses wikilink and markdown syntax per OR6–OR14, honors display-text semantics and embed metadata per OR13–OR14, recognizes structured-block anatomy per OR15 (quote/callout, table, list, fenced code, math), and resolves headings, nested heading paths, same-file references, and block ids using the rules defined in OR7–OR12.

The result model is intentionally simple: `target_path`/`candidates` stay vault-relative and forward-slash normalized (FR-006, FR-011), `target_line` is 1-based only when a heading or block is selected (FR-007, FR-020), and `emplacement` is included only for note targets when requested (FR-008, FR-014). The structured-block scanner will identify quote/callout ranges, table ranges, list ranges, fenced-code blocks, and math blocks per OR15 so the enclosing block boundaries are computed consistently for both `target_range` and `structured emplacement`. The resolver distinguishes the five mutually exclusive outcomes (`resolved`, `unresolved`, `sub_target_not_found`, `ambiguous`, `error`) via exit codes, emits compact deterministic JSON in machine mode, and preserves the ≤100 ms warm-run latency budget as a release gate on the representative vault scale (SC-002, SC-003, SC-005).

**Technical approach**: Implement the resolution logic as a reusable Rust library core, exposed through three thin adapters over one crate: a `bin` target (the CLI), a `cdylib` target (the C ABI/FFI boundary), and the `rlib` core linked by tests. Rust satisfies the constitution's Performance & Agent Efficiency principle: a precompiled native binary with low startup cost, no VM warm-up, and low memory. Vault indexing uses a single directory walk that records supported note and attachment paths only (names + relative paths), and prunes traversal so dot-prefixed directories are never descended into at any depth (OR4, FR-002f). The target note alone is scanned with a minimal line-based parser for ATX headings, fenced-code awareness, math-block awareness, structured-block boundaries, and trailing block ids (`^id`), keeping per-call work proportional to the resolved file rather than the full vault. The scanner recognizes OR15 forms explicitly: quote/callout blocks beginning with `>`, table blocks with leading/trailing pipe rows plus a separator row, list blocks starting with `-`, `*`, `+`, or numbered markers, fenced code blocks delimited by triple backticks, and math blocks delimited by `$$`. The FFI boundary exposes an opaque resolver-session handle so a host process can load a vault index once and issue many consecutive resolutions against it (FR-021a), returning the same compact JSON result record the CLI emits (single source of truth: [contracts/result.schema.json](contracts/result.schema.json)). Determinism (FR-016, SC-003) is preserved by normalizing all paths to vault-relative forward-slash form and sorting ambiguous `candidates` by ordinal byte-wise comparison. The C header is generated with `cbindgen`. Output shape (compact JSON and human-readable mode), exit codes, and the ABI signatures are fixed by the contract tests and by the OR1–OR15 specification.

## Technical Context

**Language/Version**: Rust 1.98.1 (stable, 2021 edition)

**Primary Dependencies**:
- `clap` (v4, derive) — argument/flag parsing with a small, stable CLI surface
- `serde` + `serde_json` — deterministic, compact single-line JSON output (ordered struct fields, no timestamps in the primary record); the same serializer feeds the JSON string returned across the FFI boundary
- `walkdir` — efficient recursive vault enumeration
- `cbindgen` (build) — generate the C header for the `cdylib` FFI boundary from the Rust `extern "C"` surface
- (dev) `assert_cmd` + `predicates` — CLI contract tests (stdout/stderr/exit code)
- (dev) `criterion` — warm-run latency benchmark for CI regression tracking
- Markdown handling: a small in-crate line scanner for ATX headings and block ids (no heavyweight markdown/AST dependency), to minimize cold-start and per-call cost. Rationale recorded in [research.md](research.md).

**Storage**: Local filesystem, read-only (the vault directory tree and the context file). No database.

**Testing**: `cargo test` (unit + integration), `assert_cmd`/`predicates` for CLI contract tests, an FFI contract test that loads the `cdylib` and drives the C ABI (resolve + session reuse), and `criterion` for performance/regression benchmarks. Test-first per constitution Principle III.

**Target Platform**: Cross-platform native binary and shared library — Linux, macOS, and Windows (x86_64 and arm64). No runtime dependencies. The `cdylib` produces `.so`/`.dylib`/`.dll` for in-process embedding. Each `vX.Y.Z` release publishes prebuilt CLI binaries and the C-compatible shared library for Linux (x64+arm64), macOS (x64+arm64), and Windows (x64) (SC-008).

**Project Type**: Single Rust project exposing a library core plus three adapters — a thin CLI binary, a C-ABI shared library (`cdylib`), and the linkable `rlib` used by tests.

**Performance Goals**: Warm single-link resolution ≤100 ms (acceptance threshold, SC-005) on a representative vault of up to ~5,000 notes; cold process start-up target in low single-digit milliseconds. Through the FFI session handle, consecutive in-process resolutions reuse the loaded vault index and avoid per-call process spawn (FR-021a, SC-007). Common-case latency measured and tracked in CI, and enforced as a **blocking release gate**: a warm-run p50 above ≤100 ms fails the `vX.Y.Z` release pipeline and prevents artifact publication (SC-005, constitution Principle VIII).

**Constraints**: Fast cold-start; minimal per-invocation overhead; deterministic compact single-line JSON in machine mode with no non-deterministic fields in the primary result; read-only operation; UTF-8 markdown input; single link per invocation (per CLI call; the FFI surface allows many sequential calls per process); primary results on stdout, diagnostics on stderr; documented exit-status contract; 1-based line numbers; no hard dependency that precludes .NET or Node.js integration through the FR-021 surfaces (FR-021b).

**Scale/Scope**: Designed and performance-tested against vaults of up to ~5,000 notes; larger vaults still function but are outside the guaranteed latency budget for v1.

**Build & Release Environment**: A complete `.devcontainer/devcontainer.json` declares the full Rust toolchain (stable 1.98.1, cross-compilation targets, `cbindgen`) so a fresh container builds, tests, and runs the project without manual setup (constitution Principle VII). A GitHub Actions workflow is the authoritative build/test automation; a `vX.Y.Z` tag triggers a test-gated release pipeline that runs the full suite (including the ≤100 ms warm-run p50 gate) before cross-compiling and publishing the artifacts in Target Platform above (constitution Principle VIII, SC-008). CI/release configuration stays in sync with the devcontainer toolchain.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Requirement | Plan compliance |
|-----------|-------------|-----------------|
| I. Performance & Agent Efficiency | Low latency, minimal resources, prefer native/precompiled binaries, no heavy startup | Rust native binary; no runtime warm-up; name resolution avoids reading note bodies; only the resolved note is parsed. ✅ |
| II. CLI Interface & Consistent Output | args → stdout for results, stderr for diagnostics; `--format=json` compact single-line; human mode for interactive | `--format=json` emits compact single-line JSON via serde; human mode separate; stdout/stderr split enforced by contract tests. ✅ |
| III. Test-First (NON-NEGOTIABLE) | Tests before implementation: unit, contract, performance/regression | Contract + corpus-driven test plan defined in Phase 1; `criterion` benchmark authored before implementation; tasks ordered tests-first. ✅ |
| IV. Integration Testing | Real invocation patterns, exit codes, stdout shape, error forms | `assert_cmd` integration tests over a fixture vault covering every link form and every outcome/exit code. ✅ |
| V. Observability, Versioning & Simplicity | stderr structured logs w/ verbosity flag, semver, small stable CLI surface | `-v/--verbose` diagnostics to stderr only; small flag set; semver from v0.1.0; prefer adding flags over changing defaults. ✅ |
| VI. Interoperability & Cross-Technology Integration | Language-agnostic surfaces (CLI protocol, C-compatible ABI/FFI, documented JSON schema data contract); no hard deps precluding .NET/Node.js | Three surfaces delivered: machine-mode CLI protocol, `cdylib` C ABI/FFI (opaque session handle, `cbindgen` header) for in-process embedding with vault-index reuse, and versioned JSON schema contract shared by CLI and FFI. No dependency precludes .NET (P/Invoke) or Node.js (N-API/ffi) consumption. Contract fixed in [contracts/ffi.md](contracts/ffi.md). ✅ |
| VII. Reproducible Development Environment | `.devcontainer/devcontainer.json` complete and reflects full toolchain; new deps update it in the same change | Devcontainer declares the full Rust toolchain (stable 1.98.1, cross-compilation targets for the released platforms, `cbindgen`) so a fresh container builds/tests/runs without manual setup; any new dependency is added there in the same change. ✅ |
| VIII. Continuous Integration & Release Gating | GitHub Actions as authoritative CI; `vX.Y.Z` tag runs full test suite as a gate before publishing artifacts; CI stays in sync with devcontainer | GitHub Actions CI runs unit/contract/integration/perf tests; a `vX.Y.Z` tag triggers a test-gated release pipeline that runs the full suite (including the ≤100 ms warm-run p50 gate, SC-005) and only then cross-compiles and publishes the CLI binaries + `cdylib` for all supported platforms (SC-008). CI/release toolchain mirrors the devcontainer. ✅ |
| IX. Native Documentation for Named Symbols | All named symbols documented with native language standard (Rust: Rustdoc `///` and `//!`); must explain purpose, constraints, semantics, side effects; incomplete work if undocumented | Rustdoc comments on all modules (`//!`), types, enums, functions, and public constants; documentation added in same patch as symbol definition; `cargo doc --open` verifies completeness; CI gate checks for missing docs. ✅ |

**Performance & Output Constraints**: compact single-line JSON, no timestamps in the primary record, short field names — satisfied by the contract in [contracts/](contracts/).

**Documentation Gate**: Constitution Principle IX (native documentation) MUST be verified at the end of each phase — all symbols created or modified in that phase MUST include native-language documentation before moving to the next phase.

**Result**: PASS (no violations; Complexity Tracking left empty).

## Project Structure

### Documentation (this feature)

```text
specs/001-link-resolver/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   ├── cli.md              # CLI arguments, exit codes, stdout/stderr contract
│   ├── ffi.md              # C-compatible ABI/FFI boundary (session handle, functions, header)
│   └── result.schema.json  # JSON Schema for machine-mode result record (shared by CLI + FFI)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml               # Crate manifest (lib with crate-types rlib+cdylib, plus bin target)
cbindgen.toml            # cbindgen config for generating the C header from the FFI surface
build.rs                 # Generates include/obsidian_link_resolver.h via cbindgen
.devcontainer/
└── devcontainer.json    # Complete toolchain: Rust 1.98.1, cross targets, cbindgen (Principle VII)
.github/
└── workflows/
    ├── ci.yml           # Authoritative build/test automation on push/PR (Principle VIII)
    └── release.yml      # vX.Y.Z tag: test gate (incl. ≤100 ms p50) → cross-build → publish artifacts (SC-008)
include/
└── obsidian_link_resolver.h  # Generated C header for the ABI/FFI boundary (FR-021)
src/
├── main.rs              # Thin CLI entrypoint: parse args, call lib, map outcome → exit code
├── lib.rs               # Library API (resolve entry point) reused by CLI, FFI, and tests
├── cli.rs               # clap argument/flag definitions and output-mode selection
├── ffi.rs               # C-ABI surface (extern "C"): resolver session handle, resolve, free
├── link.rs              # Obsidian link parsing (wikilink + markdown styles → Link entity)
├── vault.rs             # Vault root detection + note/attachment enumeration & name resolution
├── note.rs             # Line scanner: ATX headings, block ids, section ranges (emplacement)
├── resolve.rs           # Core resolution pipeline (Link + Context + Vault → ResolutionTarget)
└── output.rs            # ResolutionTarget → compact JSON / human text; exit-code mapping

tests/
├── contract/            # CLI contract tests (stdout shape, exit codes, stderr split)
├── ffi/                 # C-ABI contract tests: load cdylib, resolve, session/index reuse
├── integration/         # End-to-end resolution over fixture vault(s)
├── unit/                # Focused unit tests (link parsing, section ranges, name matching)
└── fixtures/            # Sample vault(s) covering every documented link form
benches/
└── resolve.rs           # criterion warm-run latency benchmark (CI regression gate)
```

**Structure Decision**: Single Rust crate whose library (`lib.rs` + modules) holds all resolution logic, exposed through three thin adapters over one core: a `bin` target (`main.rs`, the CLI), a `cdylib` C-ABI target (`ffi.rs`, the in-process embedding boundary), and the linkable `rlib` used by tests. The library boundary keeps resolution logic testable in isolation and lets the CLI, the FFI surface, and future MCP/skill wrappers (out of scope here) share the same core and the same JSON result contract. This design matches the semantic naming in the Obsidian reference: the model distinguishes `note` vs `attachment` targets (OR1, OR2, OR3, OR4, OR5), `wikilink` vs `markdown` link styles (OR6, OR7, OR8, OR9, OR10, OR11, OR12, OR13, OR14), and self-link / heading / block behavior from the same reference taxonomy. Compiling the library as both `rlib` and `cdylib` satisfies the constitution's Principle VI (Interoperability) and FR-021's C-compatible ABI/FFI requirement without a second project. This matches "Option 1: Single project" from the template, specialized to Rust's `lib` (rlib+cdylib) + `bin` layout.

## Complexity Tracking

> No constitution violations. Section intentionally empty.
