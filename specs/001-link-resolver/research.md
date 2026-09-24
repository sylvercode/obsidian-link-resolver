# Phase 0 Research: Obsidian Link Resolver CLI

All Technical Context items were resolved before planning; no `NEEDS CLARIFICATION` markers remain. This document records the decisions, rationale, and alternatives.

## Decision 0: Reference taxonomy — align the model with OR1–OR14

- **Decision**: Use the naming and semantics from [obsidian-reference.md](obsidian-reference.md) as the canonical vocabulary for design and implementation: `note` vs `attachment` (OR1–OR5), `wikilink` vs `markdown` syntax (OR6, OR7, OR8, OR9, OR10, OR11, OR12), and `display text` / `embed` metadata (OR13, OR14).
- **Rationale**: The latest spec revisions explicitly expanded the requirements to cover OR1–OR14, so the model names and the code structure should reflect the same vocabulary as Obsidian itself instead of a generic internal label set.
- **Alternatives considered**:
  - **Keep the older generic model names**: simpler to write but less faithful to the official behavior and the newly clarified requirements. Rejected because the new spec requires explicit coverage of the OR naming and semantics.
  - **Invent a second vocabulary layer**: maintain both domain terms and OR labels. Rejected as unnecessary complexity.

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

- **Decision**: Enumerate note file paths once (names + vault-relative paths) via a single directory walk that does not read file bodies. Resolve a bare note name case-insensitively across the vault; when a link is path-qualified (`folder/sub/Note`), match against the vault-relative path. Apply Obsidian's shortest-path preference; when still multiple matches remain, report `ambiguous` with the candidate list sorted ascending by vault-relative path using ordinal (byte-wise) comparison (FR-005, FR-005a, FR-005b, FR-010, FR-011).
- **Rationale**: Keeps name resolution independent of note size (only paths are indexed), preserving the latency budget on ~5,000-note vaults. Case-insensitive matching for names, folder segments, and heading/block text matches Obsidian's default (clarified in spec).
- **Alternatives considered**: Reading Obsidian's own metadata/cache — rejected as brittle, version-dependent, and unnecessary; the spec defines behavior in terms of vault contents.

## Decision 5: Output contract and determinism

- **Decision**: Serialize the primary result with `serde_json` in compact single-line form (`--format=json`), with fixed field order and no timestamps or other non-deterministic fields; provide a separate human-readable mode. Primary result on stdout, diagnostics on stderr (FR-015, FR-016, FR-018, SC-003). Constitution Principle II / Performance & Output Constraints. To guarantee byte-for-byte determinism regardless of filesystem walk order, `target_path` and every `candidates` entry are emitted as vault-relative paths with forward-slash (`/`) separators (never absolute, FR-006), and the ambiguous `candidates` list is sorted ascending by vault-relative path using ordinal (byte-wise) comparison (FR-011).
- **Rationale**: `serde` struct field order is stable, giving byte-for-byte identical output for identical inputs. Compact single-line JSON minimizes token cost for agent callers. Emitting vault-relative (not absolute) paths keeps output portable across machines, and ordinal sorting of candidates removes the only remaining source of nondeterminism (`walkdir` enumeration order), so identical inputs always produce identical records (SC-003).
- **Alternatives considered**: Pretty-printed JSON (larger, more tokens) — rejected for machine mode. Ad-hoc text formats — rejected (parsing ambiguity, violates Principle II). Absolute target/candidate paths — rejected because they defeat cross-machine determinism (FR-006). Leaving `candidates` in filesystem-walk order — rejected because that order is not stable across runs/filesystems and would break SC-003.

## Decision 6: Exit-status contract

- **Decision**: Distinct documented exit codes per outcome so callers branch without parsing text (FR-017, SC-002): `0` resolved, `2` unresolved (note not found), `3` sub-target-not-found, `4` ambiguous, `1` usage/error (bad args, vault undetermined, I/O). Codes fixed in [contracts/cli.md](contracts/cli.md).
- **Rationale**: Reserves `1` for generic/usage errors (conventional), uses `2`+ for domain outcomes; avoids collision with common shell conventions.
- **Alternatives considered**: Single non-zero for all failures — rejected; SC-002 requires distinguishing outcomes by exit status alone.

## Decision 7: Testing & performance tooling

- **Decision**: `cargo test` for unit/integration; `assert_cmd` + `predicates` for CLI contract tests; `criterion` for a warm-run latency benchmark wired into CI as a regression gate; a fixture vault under `tests/fixtures/` covering every documented link form.
- **Rationale**: Satisfies constitution Principles III & IV (test-first, integration, performance regression) and SC-001/SC-004/SC-005/SC-006. The `criterion` warm-run p50 is also the blocking release gate (Decision 9): a p50 above ≤100 ms fails the `vX.Y.Z` pipeline (SC-005).

- **Warm-run definition**: The benchmark measures steady-state repeated resolutions in one long-lived process after one unmeasured priming resolution. The priming call may populate vault/index and filesystem caches; its time, process startup, vault discovery, and any first-call initialization are excluded from the reported timing.
- **Alternatives considered**: Manual timing scripts — rejected; `criterion` gives statistically sound, CI-trackable measurements.

## Decision 8: Cross-technology interoperability surfaces (CLI + JSON schema + C ABI/FFI)

- **Decision**: Ship the resolver as one Rust crate whose library core is compiled to both `rlib` (for tests/linking) and `cdylib` (a C-compatible shared library), plus a `bin` CLI target. Consumers integrate through three documented, language-agnostic surfaces (FR-021): (a) the machine-mode CLI protocol (stdout JSON + documented exit statuses), (b) a versioned JSON schema data contract for the result record ([contracts/result.schema.json](contracts/result.schema.json)), and (c) a C ABI/FFI boundary defined in [contracts/ffi.md](contracts/ffi.md). The FFI boundary exposes an opaque resolver-session handle: a host loads a vault index once (`olr_session_open`) and issues many consecutive resolutions (`olr_resolve`) that return the same compact JSON result string the CLI emits, then releases strings/handle (`olr_string_free`, `olr_session_close`). The C header is generated from the Rust `extern "C"` surface with `cbindgen`.
- **Rationale**: Constitution Principle VI (Interoperability) and FR-021/FR-021a/FR-021b require the tool to be embeddable in other stacks (at minimum .NET and Node.js) without per-call process spawn, while reusing a loaded vault index across consecutive calls. A `cdylib` with a small, stable C ABI is the lowest-common-denominator boundary: .NET consumes it via P/Invoke and Node.js via N-API/FFI, with no per-language binding packages required for v1 (SC-007). Returning the existing JSON record as a UTF-8 string across the boundary keeps a single source of truth for the result shape (no divergent serialization) and preserves determinism (FR-016). Compiling both `rlib` and `cdylib` from one crate avoids a second project while satisfying the requirement.
- **Alternatives considered**:
  - **CLI + JSON schema only (no FFI)**: Simplest, but every in-process consumer would pay a process-spawn per call, violating FR-021a/SC-007 for frequent agent use. Rejected.
  - **Per-language native bindings (a .NET NuGet + a Node N-API addon) in v1**: Better ergonomics but larger scope and maintenance; the spec explicitly says ready-made bindings are not required for v1 provided the ABI/FFI and data contract make them feasible. Deferred beyond v1.
  - **Struct-based C ABI returning typed fields instead of a JSON string**: Tighter typing but a wider, more brittle ABI that must change whenever the result shape evolves; duplicates the JSON contract. Rejected in favor of the stable JSON-string boundary.
  - **Language-neutral RPC (gRPC/named pipes)**: Adds a runtime dependency and startup cost that conflict with Principle I and complicate embedding. Rejected for v1.
  - **WebAssembly (Wasm/WASI) module**: Attractive for Node.js and browser/edge hosts because a single portable `.wasm` avoids per-platform native builds and node-gyp/N-API toolchains, and it can be layered on the same library core. Not adopted as the v1 in-process boundary because (a) it does not satisfy FR-021's explicit C-compatible ABI/FFI requirement and is awkward for in-process .NET embedding, which would need a hosted Wasm runtime (e.g. Wasmtime) versus a simple P/Invoke against the `cdylib`; (b) the resolver's core job is reading arbitrary vault directories and context files from the local filesystem, which under Wasm requires WASI with explicitly pre-opened/granted directories and has less mature, more constrained FS support in Node; and (c) the extra JS↔Wasm boundary plus WASI FS shims add overhead against the ≤100 ms warm budget (Principle I). Recorded as a candidate *additive* Node/browser distribution surface for a later version, built on the same core, not a replacement for the C ABI/FFI boundary that also serves .NET.

## Decision 9: CI & test-gated release pipeline (GitHub Actions)

- **Decision**: Maintain a GitHub Actions CI workflow (`.github/workflows/ci.yml`) as the authoritative build/test automation (unit, contract, integration, and the `criterion` warm-run benchmark). A separate release workflow (`.github/workflows/release.yml`) triggers on `vX.Y.Z` tags and runs the full test suite as a gate — including a warm-run p50 ≤100 ms check — before cross-compiling and publishing artifacts. The gate failing (any test, or a p50 above ≤100 ms) blocks artifact publication (constitution Principle VIII, FR/SC: SC-005, SC-008).
- **Rationale**: Constitution Principle VIII requires GitHub Actions as the authoritative CI and a reproducible, test-gated release; the spec's 2026-09-19 clarifications make the ≤100 ms warm-run p50 a blocking gate and require prebuilt CLI binaries plus the C-compatible shared library for Linux (x64+arm64), macOS (x64+arm64), and Windows (x64). Rust's stable cross-compilation (`cargo`, `rustup target add`, plus `cross`/matrix runners for arm64) produces all six targets from CI.
- **Release artifacts (SC-008)**: for each supported target, the release publishes the CLI binary and the `cdylib` shared library (`.so`/`.dylib`/`.dll`) plus the generated C header, so integrators obtain both the CLI and the in-process embedding surface without building from source.
- **Alternatives considered**:
  - **Publishing artifacts without the p50 gate** — rejected; SC-005/Principle VIII require the latency budget to block the release.
  - **Single-platform release, build-from-source elsewhere** — rejected; SC-008 requires prebuilt binaries and the shared library for all six platform targets.
  - **A third-party CI (e.g. self-hosted only)** — rejected; Principle VIII names GitHub Actions as the authoritative source.

## Decision 10: Reproducible development environment (devcontainer)

- **Decision**: Keep a complete `.devcontainer/devcontainer.json` that declares the full toolchain required to build, test, and release: Rust stable 1.98.1, the cross-compilation targets for the released platforms, and `cbindgen` for header generation. Any change that adds a dependency, tool, or version requirement updates the devcontainer in the same change, and CI/release runners mirror this toolchain.
- **Rationale**: Constitution Principle VII requires the devcontainer to always reflect the full toolchain so a fresh container yields a working environment without manual setup, and Principle VIII requires CI/release to stay in sync with it — guaranteeing local, CI, and release build parity.
- **Alternatives considered**: Documenting setup steps in the README instead of the devcontainer — rejected; Principle VII mandates the devcontainer be the complete, authoritative environment definition.

| Technical Context item | Resolution |
|------------------------|-----------|
| Language/Version | Rust 1.98.1 stable (Decision 1) |
| Primary Dependencies | clap, serde/serde_json, walkdir; dev: assert_cmd, predicates, criterion (Decisions 1,2,5,7) |
| Markdown parsing | In-crate line scanner (Decision 2) |
| Vault detection | Explicit → `.obsidian` ancestor walk → fail (Decision 3) |
| Name resolution | Path-index walk + case-insensitive shortest-path (Decision 4) |
| Output/determinism | Compact single-line serde JSON, stdout/stderr split (Decision 5) |
| Exit codes | 0/2/3/4 outcomes, 1 usage/error (Decision 6) |
| Testing/perf | cargo test, assert_cmd, criterion, fixture vault (Decision 7) |
| Interoperability surfaces | CLI protocol + JSON schema + C ABI/FFI via cdylib + cbindgen (Decision 8) |
| CI & release gating | GitHub Actions CI + `vX.Y.Z` test-gated release (incl. ≤100 ms p50) publishing 6-platform artifacts (Decision 9) |
| Dev environment | Complete devcontainer mirroring the CI/release toolchain (Decision 10) |
