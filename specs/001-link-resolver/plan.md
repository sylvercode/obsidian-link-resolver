# Implementation Plan: Obsidian Link Resolver CLI

**Branch**: `001-link-resolver` | **Date**: 2026-09-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-link-resolver/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

A read-only command-line tool that takes an Obsidian link string plus the path of the context file containing it and returns the resolved target: the target file path, an optional 1-based target line (for heading/block links), and, on request, the "structured emplacement" (the ordered heading stack with begin/end line ranges). It must distinguish resolved / unresolved / sub-target-not-found / ambiguous outcomes via distinct exit codes, emit compact deterministic JSON in machine mode, and meet a ≤100 ms warm-run latency budget on a representative vault of up to ~5,000 notes. Beyond the CLI, the resolver must be consumable in-process from other technology stacks (at minimum .NET and Node.js) through three language-agnostic surfaces (FR-021): the machine-mode CLI protocol, a versioned JSON schema data contract, and a C-compatible ABI/FFI boundary that supports many consecutive resolutions per host process without per-call process spawn.

**Technical approach**: Implement the resolution logic as a reusable Rust library core, exposed through three thin adapters over one crate: a `bin` target (the CLI), a `cdylib` target (the C ABI/FFI boundary), and the `rlib` core linked by tests. Rust directly satisfies the constitution's Performance & Agent Efficiency principle: a precompiled native binary with sub-millisecond process start-up, no runtime/VM warm-up, and low memory. Vault indexing uses a single directory walk that lists note file paths only (names + relative paths), so name resolution does not read note bodies. Only the resolved target note is parsed, using a minimal line-based scanner for ATX headings (`#`..`######`) and trailing block ids (`^id`), which keeps per-call work proportional to one file rather than the whole vault. The FFI boundary exposes an opaque resolver-session handle so a host process can load a vault index once and issue many consecutive resolutions against it (FR-021a), returning the same compact JSON result record that the CLI emits (single source of truth: [contracts/result.schema.json](contracts/result.schema.json)). A C header is generated with `cbindgen`. Output shape (compact single-line JSON and human mode), the exit-code contract, and the C ABI signatures are fixed by contract test suites authored before implementation.

## Technical Context

**Language/Version**: Rust 1.83 (stable, 2021 edition)

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

**Target Platform**: Cross-platform native binary and shared library — Linux, macOS, and Windows (x86_64 and arm64). No runtime dependencies. The `cdylib` produces `.so`/`.dylib`/`.dll` for in-process embedding.

**Project Type**: Single Rust project exposing a library core plus three adapters — a thin CLI binary, a C-ABI shared library (`cdylib`), and the linkable `rlib` used by tests.

**Performance Goals**: Warm single-link resolution ≤100 ms (acceptance threshold, SC-005) on a representative vault of up to ~5,000 notes; cold process start-up target in low single-digit milliseconds. Through the FFI session handle, consecutive in-process resolutions reuse the loaded vault index and avoid per-call process spawn (FR-021a, SC-007). Common-case latency measured and tracked in CI.

**Constraints**: Fast cold-start; minimal per-invocation overhead; deterministic compact single-line JSON in machine mode with no non-deterministic fields in the primary result; read-only operation; UTF-8 markdown input; single link per invocation (per CLI call; the FFI surface allows many sequential calls per process); primary results on stdout, diagnostics on stderr; documented exit-status contract; 1-based line numbers; no hard dependency that precludes .NET or Node.js integration through the FR-021 surfaces (FR-021b).

**Scale/Scope**: Designed and performance-tested against vaults of up to ~5,000 notes; larger vaults still function but are outside the guaranteed latency budget for v1.

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

**Performance & Output Constraints**: compact single-line JSON, no timestamps in the primary record, short field names — satisfied by the contract in [contracts/](contracts/).

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

**Structure Decision**: Single Rust crate whose library (`lib.rs` + modules) holds all resolution logic, exposed through three thin adapters over one core: a `bin` target (`main.rs`, the CLI), a `cdylib` C-ABI target (`ffi.rs`, the in-process embedding boundary), and the linkable `rlib` used by tests. The library boundary keeps resolution logic testable in isolation and lets the CLI, the FFI surface, and future MCP/skill wrappers (out of scope here) share the same core and the same JSON result contract. Compiling the library as both `rlib` and `cdylib` satisfies the constitution's Principle VI (Interoperability) and FR-021's C-compatible ABI/FFI requirement without a second project. This matches "Option 1: Single project" from the template, specialized to Rust's `lib` (rlib+cdylib) + `bin` layout.

## Complexity Tracking

> No constitution violations. Section intentionally empty.
