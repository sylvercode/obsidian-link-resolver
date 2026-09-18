# Implementation Plan: Obsidian Link Resolver CLI

**Branch**: `001-link-resolver` | **Date**: 2026-09-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-link-resolver/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

A read-only command-line tool that takes an Obsidian link string plus the path of the context file containing it and returns the resolved target: the target file path, an optional 1-based target line (for heading/block links), and, on request, the "structured emplacement" (the ordered heading stack with begin/end line ranges). It must distinguish resolved / unresolved / sub-target-not-found / ambiguous outcomes via distinct exit codes, emit compact deterministic JSON in machine mode, and meet a ≤100 ms warm-run latency budget on a representative vault of up to ~5,000 notes.

**Technical approach**: Implement as a single self-contained native CLI binary in **Rust**. Rust directly satisfies the constitution's Performance & Agent Efficiency principle: a precompiled native binary with sub-millisecond process start-up, no runtime/VM warm-up, and low memory. Vault indexing uses a single directory walk that lists note file paths only (names + relative paths), so name resolution does not read note bodies. Only the resolved target note is parsed, using a minimal line-based scanner for ATX headings (`#`..`######`) and trailing block ids (`^id`), which keeps per-call work proportional to one file rather than the whole vault. Output shape (compact single-line JSON and human mode) and the exit-code contract are fixed by a contract test suite authored before implementation.

## Technical Context

**Language/Version**: Rust 1.83 (stable, 2021 edition)

**Primary Dependencies**:
- `clap` (v4, derive) — argument/flag parsing with a small, stable CLI surface
- `serde` + `serde_json` — deterministic, compact single-line JSON output (ordered struct fields, no timestamps in the primary record)
- `walkdir` — efficient recursive vault enumeration
- (dev) `assert_cmd` + `predicates` — CLI contract tests (stdout/stderr/exit code)
- (dev) `criterion` — warm-run latency benchmark for CI regression tracking
- Markdown handling: a small in-crate line scanner for ATX headings and block ids (no heavyweight markdown/AST dependency), to minimize cold-start and per-call cost. Rationale recorded in [research.md](research.md).

**Storage**: Local filesystem, read-only (the vault directory tree and the context file). No database.

**Testing**: `cargo test` (unit + integration), `assert_cmd`/`predicates` for CLI contract tests, `criterion` for performance/regression benchmarks. Test-first per constitution Principle III.

**Target Platform**: Cross-platform native binary — Linux, macOS, and Windows (x86_64 and arm64). No runtime dependencies.

**Project Type**: Single-project CLI (library core + thin CLI binary).

**Performance Goals**: Warm single-link resolution ≤100 ms (acceptance threshold, SC-005) on a representative vault of up to ~5,000 notes; cold process start-up target in low single-digit milliseconds. Common-case latency measured and tracked in CI.

**Constraints**: Fast cold-start; minimal per-invocation overhead; deterministic compact single-line JSON in machine mode with no non-deterministic fields in the primary result; read-only operation; UTF-8 markdown input; single link per invocation; primary results on stdout, diagnostics on stderr; documented exit-status contract; 1-based line numbers.

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
│   └── result.schema.json  # JSON Schema for machine-mode result record
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml               # Crate manifest (bin + lib targets)
src/
├── main.rs              # Thin CLI entrypoint: parse args, call lib, map outcome → exit code
├── lib.rs               # Library API (resolve entry point) reused by tests and wrappers
├── cli.rs               # clap argument/flag definitions and output-mode selection
├── link.rs              # Obsidian link parsing (wikilink + markdown styles → Link entity)
├── vault.rs             # Vault root detection + note/attachment enumeration & name resolution
├── note.rs             # Line scanner: ATX headings, block ids, section ranges (emplacement)
├── resolve.rs           # Core resolution pipeline (Link + Context + Vault → ResolutionTarget)
└── output.rs            # ResolutionTarget → compact JSON / human text; exit-code mapping

tests/
├── contract/            # CLI contract tests (stdout shape, exit codes, stderr split)
├── integration/         # End-to-end resolution over fixture vault(s)
├── unit/                # Focused unit tests (link parsing, section ranges, name matching)
└── fixtures/            # Sample vault(s) covering every documented link form
benches/
└── resolve.rs           # criterion warm-run latency benchmark (CI regression gate)
```

**Structure Decision**: Single-project Rust crate exposing both a library (`lib.rs` + modules) and a thin binary (`main.rs`). The library boundary keeps resolution logic testable in isolation and lets future MCP/skill wrappers (out of scope here) link against the same core. This matches "Option 1: Single project" from the template, specialized to Rust's `bin` + `lib` layout.

## Complexity Tracking

> No constitution violations. Section intentionally empty.
