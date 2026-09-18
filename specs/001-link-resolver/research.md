# Phase 0 Research: Obsidian Link Resolver CLI

All Technical Context items were resolved before planning; no `NEEDS CLARIFICATION` markers remain. This document records the decisions, rationale, and alternatives.

## Decision 1: Implementation language — Rust

- **Decision**: Implement the CLI as a native binary in Rust (stable, 2021 edition).
- **Rationale**: Constitution Principle I mandates low latency, minimal resource usage, and prefers "native or precompiled binaries when practical" with no heavy startup. Rust produces a statically-linkable native binary with sub-millisecond process start-up and no VM/interpreter warm-up, which is decisive for a tool that agents invoke frequently and that has a ≤100 ms warm-run budget (SC-005). Rust also has a mature CLI ecosystem (`clap`, `serde_json`) and strong correctness guarantees for the many edge cases in the spec.
- **Alternatives considered**:
  - **Go**: Also compiles to a fast-starting native binary with a good CLI ecosystem; slightly larger runtime/GC overhead and larger binaries, but a valid second choice. Rejected primarily to get the lowest possible per-call overhead and the strongest type/enum modelling of outcome states.
  - **Python**: Excellent developer velocity and markdown libraries, but interpreter cold-start (tens of ms plus import cost) makes the ≤100 ms warm budget fragile under frequent agent invocation and violates the "avoid heavy startup" guidance. Rejected.
  - **Node.js/TypeScript**: Same cold-start/runtime concern as Python; heavier distribution. Rejected.

## Decision 2: Markdown/heading parsing — minimal in-crate line scanner

- **Decision**: Parse only the resolved target note, using a small line-based scanner for ATX headings (`#`..`######`) and trailing block ids (`^id`), rather than a full markdown AST library.
- **Rationale**: The tool needs only heading lines, heading levels, block-id lines, and section begin/end ranges — all of which are line-oriented. A minimal scanner keeps the dependency graph and binary size small (faster cold-start) and makes section-range computation (FR-009) straightforward. It also avoids AST libraries that may normalize or reflow content in ways that complicate exact 1-based line mapping (FR-020).
- **Alternatives considered**:
  - **`pulldown-cmark`**: Fast, correct CommonMark parser, but Obsidian block ids (`^id`) and Obsidian heading semantics are not part of CommonMark, so custom handling would be needed anyway; adds start-up and complexity for little gain. Rejected for v1.
  - **`comrak`**: Feature-rich but heavier; same block-id gap. Rejected.
- **Note**: Fenced code blocks must be tracked so that `#`/`^id`-looking lines inside code fences are not treated as headings/block ids. The scanner tracks fence state.

## Decision 3: Vault root detection

- **Decision**: (a) Use an explicit caller-provided vault root when given; otherwise (b) walk up from the context file's directory to the nearest ancestor containing an `.obsidian` directory; (c) if neither, report "vault could not be determined" (FR-004, FR-004a).
- **Rationale**: Matches the spec exactly and mirrors how Obsidian identifies a vault. Deterministic and cheap (a bounded parent walk, stopping at the filesystem root).
- **Alternatives considered**: Guessing the vault as the context file's directory when no `.obsidian` is found — rejected because the spec explicitly requires reporting failure rather than guessing.

## Decision 4: Name resolution strategy (Obsidian shortest-path)

- **Decision**: Enumerate note file paths once (names + vault-relative paths) via a single directory walk that does not read file bodies. Resolve a bare note name case-insensitively across the vault; when a link is path-qualified (`folder/sub/Note`), match against the vault-relative path. Apply Obsidian's shortest-path preference; when still multiple matches remain, report `ambiguous` with the candidate list (FR-005, FR-005a, FR-005b, FR-010, FR-011).
- **Rationale**: Keeps name resolution independent of note size (only paths are indexed), preserving the latency budget on ~5,000-note vaults. Case-insensitive matching for names, folder segments, and heading/block text matches Obsidian's default (clarified in spec).
- **Alternatives considered**: Reading Obsidian's own metadata/cache — rejected as brittle, version-dependent, and unnecessary; the spec defines behavior in terms of vault contents.

## Decision 5: Output contract and determinism

- **Decision**: Serialize the primary result with `serde_json` in compact single-line form (`--format=json`), with fixed field order and no timestamps or other non-deterministic fields; provide a separate human-readable mode. Primary result on stdout, diagnostics on stderr (FR-015, FR-016, FR-018, SC-003). Constitution Principle II / Performance & Output Constraints.
- **Rationale**: `serde` struct field order is stable, giving byte-for-byte identical output for identical inputs. Compact single-line JSON minimizes token cost for agent callers.
- **Alternatives considered**: Pretty-printed JSON (larger, more tokens) — rejected for machine mode. Ad-hoc text formats — rejected (parsing ambiguity, violates Principle II).

## Decision 6: Exit-status contract

- **Decision**: Distinct documented exit codes per outcome so callers branch without parsing text (FR-017, SC-002): `0` resolved, `2` unresolved (note not found), `3` sub-target-not-found, `4` ambiguous, `1` usage/error (bad args, vault undetermined, I/O). Codes fixed in [contracts/cli.md](contracts/cli.md).
- **Rationale**: Reserves `1` for generic/usage errors (conventional), uses `2`+ for domain outcomes; avoids collision with common shell conventions.
- **Alternatives considered**: Single non-zero for all failures — rejected; SC-002 requires distinguishing outcomes by exit status alone.

## Decision 7: Testing & performance tooling

- **Decision**: `cargo test` for unit/integration; `assert_cmd` + `predicates` for CLI contract tests; `criterion` for a warm-run latency benchmark wired into CI as a regression gate; a fixture vault under `tests/fixtures/` covering every documented link form.
- **Rationale**: Satisfies constitution Principles III & IV (test-first, integration, performance regression) and SC-001/SC-004/SC-005/SC-006.
- **Alternatives considered**: Manual timing scripts — rejected; `criterion` gives statistically sound, CI-trackable measurements.

## Resolved unknowns summary

| Technical Context item | Resolution |
|------------------------|-----------|
| Language/Version | Rust 1.83 stable (Decision 1) |
| Primary Dependencies | clap, serde/serde_json, walkdir; dev: assert_cmd, predicates, criterion (Decisions 1,2,5,7) |
| Markdown parsing | In-crate line scanner (Decision 2) |
| Vault detection | Explicit → `.obsidian` ancestor walk → fail (Decision 3) |
| Name resolution | Path-index walk + case-insensitive shortest-path (Decision 4) |
| Output/determinism | Compact single-line serde JSON, stdout/stderr split (Decision 5) |
| Exit codes | 0/2/3/4 outcomes, 1 usage/error (Decision 6) |
| Testing/perf | cargo test, assert_cmd, criterion, fixture vault (Decision 7) |
